# Application architecture

The application uses Leptos for server rendering and browser hydration, Axum on
Tokio for HTTP, and Lettre for SMTP delivery. Public content comes from files;
private SQLite storage holds approval and session tables. Build and
test commands are in [BUILD.md](BUILD.md), and
runtime configuration and maintenance are in [README.md](README.md).

## Source organization

| Location | Responsibility |
| --- | --- |
| `justfile` | Shared build/run/test, deployment, and content-maintenance commands |
| `src/main.rs` | Logging, validated server/Leptos configuration, route generation, and TCP listener |
| `src/server_config.rs` | Server-only origin/data-path validation and optional development Google credentials |
| `src/storage.rs` | Private layout, ownership/mode validation, and public/data separation |
| `src/database.rs`, `migrations/` | Shared SQLx pool settings, embedded application migrations, and session schema setup |
| `src/cli.rs` | Clap operator interface, explicit approval/revocation, command dispatch |
| `src/auth.rs` | SQLite sessions, browser-bound login transactions, shared authorization/CSRF guards |
| `src/content.rs` | Validated offline import and checksummed database/gallery backups |
| `src/app.rs` | HTML shell, shared navigation/audio layout, routes, and dialog dismissal helper |
| `src/lib.rs` | Browser hydration entry point and shared library exports |
| `src/serve.rs` | Axum router, server functions, request tracing, compression, and response headers |
| `src/fallback.rs` | Static file service and missing-file HTML fallback |
| `src/routes/` | Pages, content loaders, and contact server function |
| `src/audio/` | Audio player and browser media event handling |
| `style/` | SCSS entry point and partials |
| `public/` | Source images, fonts, music, and JSON content |
| `tests/`, `end2end/tests/` | Rust integration fixtures/tests and Playwright browser tests |

## Rendering and request flow

`cargo-leptos` builds the server binary with `ssr` and the browser library with
`hydrate`. Server-only dependencies, including Lettre, `parking_lot`, and `secrecy`,
are enabled only by `ssr`. The browser entry point hydrates the server-rendered
`App` so controls and navigation become interactive.

The shared layout keeps navigation and the audio player outside the page routes.
Routes cover `/`, `/about_me`, `/shows` (including `/past` and `/future`),
`/discography`, `/discography/:name`, and `/media`. Leptos server functions connect
the page components to content loading and contact delivery.

Axum registers the Leptos routes, server functions, and `/health-check`, then uses
the static-file handler as its fallback. Middleware attaches and propagates
`x-request-id`, records request/response events, compresses responses, and applies
Cache-Control headers. Docker packages the release binary and generated site
directory together; Fly forwards HTTP traffic to the server's port 8080.

The process sets Unix umask `0077` before creating the Tokio runtime. Configuration
and private storage are validated before binding the listener, and configuration
is shared through
an Axum `Extension<Arc<ServerConfig>>`. Production requires a public HTTPS origin,
an absolute data path, and paired Google OAuth credentials. Explicit loopback
development can omit both credentials; it does not grant administrator access.
The secret uses `SecretString` and configuration has no `Debug` implementation.
No auth configuration is serialized to the browser. Google login and the editing
UI are not implemented yet. Storage contains `admin.sqlite3`
with mode `0600`, plus original/thumbnail and staging directories with
mode `0700`. Existing files are retained; unsafe ownership, modes, links, or public
path overlap stops startup. Gallery/staging share the data root's filesystem for
later atomic publication. Public asset trees are checked for symlinks at startup
and must remain immutable at runtime; the static handler serves only built assets.

Before binding the listener, `DatabaseManager` opens that database with foreign
keys, WAL, `synchronous=FULL`, and a five-second busy timeout on every pooled connection.
Application migrations are embedded via `sqlx::migrate!`; `build.rs` tracks changes
to the migration directory. The session store initializes its own table using the
same pool. Initialization failure aborts startup. The manager owns a private SQLx
pool; cloning the manager shares its connections without an additional mutex.
Application SQL lives in `database.rs`, and other routines call named manager
methods. Direct pool access is available only to Rust tests; versioned migrations
and independent Python database checks remain separate. The shared pool is closed
after serving ends. No migration approves a user.
`admin migrate` performs the same setup without HTTP/provider configuration and
exits. Fly startup also requires a separate filesystem mounted at `/data`.

`serve::router` builds the middleware/context stack used by production and tests.
SQLite session middleware wraps private namespace guards. Axum receives the manager
and configuration through extensions; Leptos receives the same values through
additional server context and extracts the same session from request parts.
Private response cache/referrer headers survive release-mode mapping. Guards
enforce idle/absolute expiry and current approval/generation; mutation guards add
Origin and constant-time CSRF checks. Login transaction consumption is an atomic
browser-bound delete/return operation. A scoped cleanup task removes expired
transactions and sessions every five minutes and is aborted when serving ends.

The HTTP process and identity commands hold shared filesystem locks. Offline
import/backup commands require exclusive ownership across processes. Import stages
validated images before atomically linking private files and committing catalog
rows, with a singleton marker preventing a second import. The manager inserts the
catalog in one transaction, calls the content routine's file-publication closure,
and commits only after publication succeeds. Publication failure removes new links
and rolls back rows; ambiguous commit failure retains files to avoid breaking any
committed references. Backup opens the database without applying pending migrations,
uses `VACUUM INTO`, copies matching gallery
files, and writes SHA-256 manifest/completion files. Startup never imports bundled
content automatically. Public loaders still use built files until the later
content workflow change.

SIGINT and SIGTERM initiate Axum's graceful shutdown. Active connections get up to
25 seconds to drain; remaining connections close when that deadline expires. Fly's
30-second shutdown allowance leaves time for this bounded drain. Contact SMTP work
has a separate 20-second overall timeout.

### Static file responses

Static requests pass their original headers to `ServeDir`, which handles byte
ranges and cache preconditions. Audio range requests return `206` with the requested
bytes. A matching `If-Modified-Since` condition returns an empty `304` response.
Only missing files render the app's HTML `404` page. Redirects, failed preconditions,
unsatisfiable ranges, and other file-service responses pass through.

Development responses use `no-store, no-cache, must-revalidate, proxy-revalidate`;
release responses use `max-age=3600, must-revalidate`.

### Album URLs

`/discography/minorflaw` and `/discography/galaterna` return HTTP `200`. Unknown
album names return HTTP `404` with the shared error display and a home link.
The discography index remains available at `/discography`.

## Content loading and state

Shows, videos, and gallery filenames load from `LeptosOptions.site_root`, the same
directory used for static files. Gallery URLs remain `/img/about_pics/<filename>`.
The default local root is `target/site`, populated from `public/` by cargo-leptos.

Each loader uses Tokio's async `OnceCell` to cache its result, including errors,
for the life of the server process. Initialization awaits file I/O. The cache is
not keyed by site root: the process uses one configured root. See
[content maintenance](README.md#content-maintenance) for reload requirements.

Show dates are validated before sorting or caching any records. An invalid date
rejects the whole file. Shows, Media, and the gallery use blocking resources so
their error boundaries can set HTTP `500` before the response headers are sent.
Successful pages keep HTTP `200`.

The gallery lists supported regular image files and percent-encodes each UTF-8
filename as a URL path segment. It ignores links, directories, and unsupported
files. Viewer navigation uses exact URL equality.

The gallery initially requests 480×270 WebP thumbnails. The first four load eagerly;
the rest use native lazy loading. An original photo is requested only when the
viewer opens.

## Contact form protections

The `SendMail` server function in `src/routes/home.rs` trims surrounding whitespace,
then validates the submission before reserving a sending slot or contacting SMTP.

| Field | Requirement |
| --- | --- |
| Name | Optional; at most 100 characters |
| Email | Required; valid address syntax and at most 254 UTF-8 bytes |
| Subject | Required; 1–200 characters on a single line |
| Message | Required; 1–10,000 characters; line breaks and tabs allowed |

Text lengths use UTF-16 code units to match browser `maxlength` behavior.
Single-line fields reject control characters. Messages reject control characters
other than line breaks and tabs. Email validation checks syntax without DNS lookups
or verification mail. Invalid submissions return HTTP `400`.

Each server process permits **10 validated send attempts per rolling hour**, shared
by all visitors. A short `parking_lot::Mutex` update reserves an attempt atomically;
no lock is held during SMTP or other async work. The mutex does not poison on panic:
recorded attempts stay in effect after the lock is released. Invalid submissions
use no slot; missing or empty SMTP credentials also use no slot and return HTTP
`500` with a static configuration log. SMTP failures and timeouts use a slot.
A full allowance with configured SMTP credentials returns HTTP `429` until an attempt
is at least one hour old.

The allowance resets on restart, each replica has its own allowance, and one visitor
can exhaust the shared allowance. A durable cap across replicas would require shared
storage. Concurrent tests use `std::sync::Barrier`, which has no `parking_lot`
equivalent.

Delivery uses Lettre with TLS through `smtp.porkbun.com`. `PORKMAIL_PWD` is read
once at startup into `ServerConfig` as an optional `SecretString`; absent or empty
values disable email. `SendMail` extracts the shared configuration from the Axum
request extension and borrows its password. Explicit exposure creates the
owned string required by Lettre's credentials API. The wrapper wipes its own buffer
on drop; it does not wipe Lettre's internal password copies or the process
environment. Delivery logging omits private fields; see [logs and privacy](README.md#logs-and-privacy).

## Browser interaction

Audio time, duration, playback state, and volume follow native media events instead
of a polling timer. Metadata already loaded before hydration is read on mount.
Unknown or non-finite durations disable seeking; rejected play requests leave the
control paused.

Dialogs use native modal behavior for focus, background inertness, and Escape
handling. The shared light-dismiss helper supplies outside-click dismissal when
the browser lacks `closedby` support, ignoring padding clicks and selection drags
that start inside. Listeners use `leptos-use-event-listener` with its SSR feature.
Visitor-facing controls are described in [README.md](README.md#keyboard-controls-and-dialogs).

The shared page layout reads the router location to set the background position.
Navigation links have no background-changing click handlers, so direct loads,
nested routes, and browser history use the same framing.
