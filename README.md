# Maj Kavšek's portfolio website

A personal website for trumpeter and composer Maj Kavšek, with a biography,
photo gallery, show listings, discography, videos, audio playback, and a contact form.
Built with Rust, Leptos, and Axum.

The public `/info` page describes the portfolio. The admin sign-in page and
signed-in editor link to it and the privacy policy; these links are not part of
the public portfolio navigation. Both pages
remain readable without sign-in. For Google branding, use `/info` as the
application homepage and `/privacy` as the privacy policy URL.
The public `/privacy` page describes identity-only Google authentication, local
sessions, contact messages, external services, retention and deletion. Keep its
text in `src/routes/privacy.rs` aligned with implementation and actual operating
practices; publishing the source change still requires a website release.

Use `just` as the project command runner; `just --list` shows the commands in
[justfile](justfile). Common commands are `just dev`, `just build`,
`just build-release`, `just test`, and `just check`. Recipes work from repository
subdirectories and use Bash, including when your interactive shell is Fish.

Use `just coverage` for native Rust coverage reports and `just security` for
cargo-audit/cargo-deny dependency checks. Coverage improves incrementally without
an initial percentage gate; security findings must be investigated before release.
Commands and tool installation are documented in [BUILD.md](BUILD.md).

- [BUILD.md](BUILD.md): dependencies, development and release builds, Docker builds,
  and Rust/browser test instructions.
- [ARCHITECTURE.md](ARCHITECTURE.md): application structure, request handling,
  content loading, and contact form protections.
- [MAINTENANCE.md](MAINTENANCE.md): routine operations, migrations, backups,
  restores, deployments, rollback, and administrator access.
- [AUDIT.md](AUDIT.md): audit findings and their resolution history.

## Development disk usage

During continuous development, run `just clean` daily, preferably at the end of
the work session. It runs `cargo clean`. Incremental compilation and compiled
dependencies can accumulate enough output to fill the drive, particularly when
building server, WASM, release, and coverage variants of this project.

Stop development watchers and active builds/tests before cleaning. Save any
coverage reports you want to retain outside `target/`, then run:

```sh
just clean
```

With the default configuration this removes the entire `target/` tree, including
`target/front/`, `target/tarpaulin/`, generated `target/site/` assets, compiled
dependencies, and `target/coverage/` reports. The next build recompiles dependencies;
rebuild with `just build` before using `just run`. Source files and runtime data
kept outside `target/` are retained. Cargo's downloaded dependency cache and
installed command-line tools are separate and are not removed by this command.
If you override Cargo's target directory, cleanup follows that setting.

Check available space before larger build/test sessions with `df -h .`; use
`du -sh target` to inspect this project's build output when present. Keep coverage
builds limited to the relevant changes and avoid accumulating unnecessary build
variants between daily cleanups. Cleaning reduces disk use at the cost of a slower
first rebuild; it does not replace checking free space during long sessions.

## Runtime configuration

| Variable | Purpose |
| --- | --- |
| `PORKMAIL_PWD` | Optional SMTP password, loaded once at startup into `ServerConfig` as a `SecretString`; supply through the environment or hosting secrets |
| `LEPTOS_SITE_ROOT` | Directory containing built assets and import sources; defaults to `target/site` for local builds |
| `LEPTOS_SITE_ADDR` | Server bind address; defaults to `0.0.0.0:8080` |
| `RUST_LOG` | Required logging filter for release binaries; use `info` for normal operation |
| `APP_ENVIRONMENT` | `development` (loopback origin only) or `production`; defaults to development in debug builds and production in release builds |
| `MAJ_APP_ORIGIN` | Required canonical browser origin, without a trailing slash; production requires a public HTTPS hostname |
| `MAJ_DATA_DIR` | Required absolute private data path outside public assets; startup prepares storage and migrates SQLite before listening |
| `GOOGLE_CLIENT_ID` | OAuth client ID, paired with the client secret; required in production |
| `GOOGLE_CLIENT_SECRET` | Server-only OAuth secret; use hosting secrets in production or the interactive development launcher |

Never commit credentials or generated secret configuration. Builds and automated
tests do not need SMTP or Google credentials. Google sign-in, local approval,
logout, gallery uploads/removals, show/video editors, and private operator commands
are available. Missing or invalid production settings
stop startup without logging their values. Local development may omit both
Google credentials; a partial or placeholder pair is rejected.

At startup, the server creates `gallery/originals/`, `gallery/thumbnails/`, and
`staging/` beneath `MAJ_DATA_DIR`, plus `admin.sqlite3`. Directories
use mode `0700` and the database file mode `0600`. A process creation mask of
`0077` is set before Tokio starts, covering future companion/upload files. This
setup requires Unix filesystem permissions. SQLite contains the administrator
approval, login-transaction, shows, videos, gallery, import-marker, and session
tables. New databases have zero approved
administrators and empty content catalogs. Public shows, videos and gallery listings
read SQLite directly. Import the bundled content once before switching an existing
site to this build; an empty database intentionally displays empty content.

Existing data is preserved. Startup rejects incorrect ownership, non-private
directory/database permissions, symlinked managed subdirectories, and database
symlinks or hard links. Root aliases are canonicalized before use. Existing SQLite
companion files must also be private. Staging and gallery
directories must share the data root's filesystem. For a new dedicated data root,
run as the server's runtime user:

```sh
install -d -m 700 /path/to/dedicated/maj-data
```

For existing data, inspect the contents and ownership before correcting the
permissions named in a startup error. Filesystem preparation never changes
existing ownership/permissions or truncates files; database schema migrations
run during normal startup and operator initialization. Keep runtime data outside the Docker
build context as well as the public trees.

`LEPTOS_SITE_ROOT` remains the built asset directory (`/app/site` in Docker).
Data and public roots may not overlap, including through aliases. Public asset
trees must be symlink-free; startup rejects even dangling links that could expose
data created later. Keep built assets immutable while the server runs. Dedicated
gallery handlers serve only files referenced by a published database record; bundled
gallery files cannot bypass these lookups. Biography updates still require editing
Rust and releasing.

Database initialization uses SQLx with foreign keys, WAL, `synchronous=FULL`, and
a five-second busy timeout on every connection. Embedded versioned migrations and
the session library's schema setup run before HTTP starts. Missing/corrupt files,
migration failures, or checksum mismatches stop startup. On Fly (`FLY_APP_NAME`
present), `/data` must be a separate mounted filesystem; an ordinary directory
on the container's root filesystem is rejected before creating storage. Mount
checks do not verify volume identity or backups; provision persistence separately.

To initialize or migrate local data without Google/SMTP credentials or an HTTP
origin, run `just db-migrate`. It uses the normal development data path unless
`MAJ_DATA_DIR` is set. For an existing deployed binary, run
`./leptos-website-maj admin migrate` as the runtime user with `MAJ_DATA_DIR` set.
The command validates private storage, runs the same database setup as HTTP
startup, closes the pool, and exits without opening a listener. Unknown commands
fail instead of starting HTTP.

Create new migrations with `just migration-add descriptive_name` (requires
sqlx-cli). Review the generated SQL, rebuild, and apply it with `just db-migrate`.
Applied migration files are immutable: add a new migration for later changes.
Back up existing data before schema-changing releases. Leave SQLite's WAL/SHM
files in place while running; copying only a live main file is not a backup.
Use the coordinated backup command below. Automatic scheduling, encryption, and
off-machine transfer remain operator responsibilities.

### Private operator commands

`just admin --help` lists Clap's subcommands. The recipes remove Google/SMTP
credentials and default to the development data directory. Set `MAJ_DATA_DIR`
to operate on another directory. Deployed binaries accept the same `admin ...`
arguments without Cargo/Just, provider credentials, or HTTP-origin configuration.
Command results go to stdout and diagnostics to stderr; `list-users` emits JSON.

| Command | Behaviour |
| --- | --- |
| `just admin migrate` | Prepare storage and migrate schemas; also available as `just db-migrate` |
| `just admin list-users` | List explicit identity approvals, labels, enabled flags, and generations |
| `just admin grant-google --issuer https://accounts.google.com --subject VERIFIED_SUBJECT --label owner` | Approve the exact verified subject; repeat grants keep an enabled user's generation unchanged |
| `just admin revoke-google --issuer https://accounts.google.com --subject VERIFIED_SUBJECT` | Disable the identity and advance its generation |
| `just admin revoke-all-sessions` | Advance all generations and remove browser sessions/login transactions |
| `just admin import-content --source public` | Import the bundled catalog once; stop HTTP first |
| `just admin backup --output /absolute/private/backup-name` | Snapshot database/gallery into a new directory; stop HTTP first |

An approval is a local access decision, not identity verification. Copy the issuer
and subject from `/auth/identity` after Google sign-in; an email address or Google
Console Test users entry does not identify the local approval record. Re-enabling
a revoked identity advances its generation again so old sessions cannot revive.
Use the same binary version as the serving application for identity commands;
perform schema upgrades in a planned maintenance window.

HTTP and ordinary identity commands hold shared locks on `MAJ_DATA_DIR/admin.lock`.
Import and backup require an exclusive lock and fail promptly while HTTP or another
operator process holds one. Stop the server for these maintenance operations, then
restart it afterward. Leave the lock file in place; removing it does not safely
unlock another process. Directory/database validation applies to these commands
too. The lock file is private (`0600`) and excluded from Git/Docker contexts.

Import validates all JSON dates, HTTPS YouTube URLs/IDs, image bytes and thumbnails
before publishing catalog rows. Optional show fields remain optional. Originals
are JPEG/PNG/WebP, at most 20 MiB, 40 million pixels and 12000 pixels per side;
thumbnails must be 480x270 WebP. Decoder allocation is capped at 256 MiB per image;
allow additional memory for the process and input/output buffers. Import handles
one image at a time. JSON files are limited to 2 MiB, with at most 10000 shows,
1000 videos, and 1000 photos. Source symlinks, malformed data, and existing catalogs
are rejected. Files receive opaque stored names; public filenames remain in the
database for the catalog-gated gallery handlers. Failed publication can leave private
orphans after a crash, but must not expose partially imported records. Public
pages now read the live database; no restart is needed after online edits.

Backup does **not** run pending schema migrations. Provision its parent directory
privately, keep it outside public trees and the Docker build context, and choose
a new destination name. The command uses SQLite `VACUUM INTO`, copies gallery
files, validates all referenced originals/thumbnails, and writes `manifest.json`
with SHA-256 checksums. `COMPLETE` is written last and contains the manifest hash.
An incomplete directory lacks a valid completion marker and must not be restored;
inspect failed backups before removing only their dedicated destination. Existing
destinations are never overwritten. Staging uploads are excluded; unreferenced
gallery files may be included. Do not copy only a live SQLite main file as a backup.

For restoration, stop all processes using the target data directory, verify the
completion marker, manifest version, and every file checksum, then restore the
database and gallery together into a new private directory with the documented
modes. Run `admin revoke-all-sessions` on the restored database before exposing
it. Native tests cover isolated restore and revocation; production recovery and
off-machine backups still require verification during deployment planning.

### Google sign-in and administrator access

The server shares `DatabaseManager`, validated configuration, and session context
with Axum handlers and Leptos server functions. `src/auth.rs` provides shared
administrator and mutation guards; every future mutation must use them too.
Private `/admin`, `/api/admin`, and `/auth` responses retain `no-store` and
`Referrer-Policy: no-referrer`, including errors and redirects. The authenticated
`/admin` page uses `same-origin` so native forms send a usable Origin header;
cross-site referrers remain suppressed and exact Origin/CSRF checks stay required.
See the [Fetch Origin rules](https://fetch.spec.whatwg.org/#append-a-request-origin-header).

Visit `/admin` using the exact `MAJ_APP_ORIGIN` browser origin to sign in;
`localhost` and `127.0.0.1` have different cookies and Origin headers.
An account is never enrolled automatically: an
unapproved Google account is redirected to `/auth/identity`, which shows only
that browser's verified issuer and subject, returns HTTP 403, and expires after
five minutes. Share those values privately with the operator. Run the approval
command against the same data directory, then sign in again. The protected admin
landing page provides Gallery, Shows and YouTube video forms, account status,
and sign-out. Biography remains a source-code edit.

| Route | Purpose |
| --- | --- |
| `GET /auth/google/start` | Persist browser-bound state and redirect to Google with PKCE and the `openid` scope |
| `GET /auth/google/callback` | Consume state once, exchange the code, verify identity, and check local approval |
| `GET /auth/identity` | Temporarily show the current browser's unapproved, verified identity |
| `POST /auth/logout` | Verify Origin/CSRF, revoke all sessions for this administrator, and clear this browser's session/cookie |
| `GET /admin` | Show the sign-in prompt or the approved administrator landing page |
| `POST /admin/content` | Add/remove a photo, show or video after approval, Origin and multipart CSRF checks |

Production sessions use `__Host-maj_admin`, Secure, HttpOnly, SameSite=Lax, Path=/,
and no Domain; loopback development uses `maj_admin_dev` without Secure. Guards
enforce 30-minute idle and eight-hour absolute limits, checking current approval
and generation on each protected request. Mutations require an exact Origin and
session-bound `x-csrf-token`; the native sign-out form carries the same token in
its hidden `csrf_token` field. Signing out invalidates all existing website
sessions for that administrator across browsers/devices; other administrators
remain signed in. The generation is advanced before this browser's session is
flushed, so an in-flight request cannot restore access by saving a cached record.
Approval remains enabled for a fresh sign-in. Signing out does not sign the user
out of Google. Expired sessions/transactions are cleaned every five
minutes. Login transactions use OAuth2-generated state, PKCE and nonce material,
expire after five minutes, and are consumed once for their initiating browser.
At most three outstanding transactions per browser and 128 overall are allowed.
Login starts and callbacks also have process-wide limits of 20 and 60 attempts
per minute, respectively. These limits reset on process restart.

OAuth2 handles the PKCE code exchange; `openidconnect` verifies RS256 signatures,
issuer, audience, expiry and nonce. Additional checks enforce issuance time,
optional not-before, expiration ordering, subject shape, and authorized party.
Discovery and public keys are fetched lazily from pinned Google endpoints, with
five-second request timeouts, 64 KiB response limits, and bounded refresh/backoff
for rotation and failures. `http-cache-semantics` determines freshness from HTTP
headers, capped at one hour; non-cacheable responses are discarded after use, and
responses without usable freshness are immediately stale. Startup and existing
local sessions make no Google requests. No access/refresh tokens are retained.

The SQLite advisory is addressed by a pinned upstream SQLx fix and bundled
`libsqlite3-sys` 0.39.1. The application and session store share this dependency
path. See [the dependency review](#patched-sqlite-dependency) for the exact source,
verification, and removal plan; no SQLite advisory exception is used.

For ordinary local development, `just dev` supplies the loopback origin and
defaults `MAJ_DATA_DIR` to `$HOME/.local/share/maj-admin-dev`, removing Google and
SMTP credentials. After a complete build, start an existing binary with
`just run` (debug) or `just run-release` (release). For example:

```sh
just build-release
just run-release
```

For production, supply the four required auth settings and run
`just run-production`, which uses the release binary and preserves provisioned
credentials. Set `PORKMAIL_PWD` in that process environment for real contact delivery.
If it is absent or empty, contact submissions return HTTP `500` without contacting
SMTP or consuming an hourly sending slot. The rest of the site remains available.
Restart the server after changing the password; handlers use the startup configuration.
To serve a different content directory with an existing binary:

```sh
env LEPTOS_SITE_ROOT=/path/to/site just run-release
```

For server-only development against source content, without rebuilding browser assets:

```sh
just dev-source
```

This serves source content; use the complete build in [BUILD.md](BUILD.md#development-build)
when testing browser hydration and compiled styles.

The Docker image defaults to production and requires all four authentication
configuration variables. For a local public-page check without Google credentials,
run the image built by the build guide with:

```sh
just docker-run
```

The image sets `RUST_LOG=info`, serves `/app/site`, and listens on port 8080.
`GET /health-check` returns HTTP `200`. For real contact delivery, add
`--env PORKMAIL_PWD` using `just docker-run --env PORKMAIL_PWD` to pass an
already-set host variable.

## Deployment

Install the Fly.io CLI (`flyctl`) and authenticate to the configured application
before deploying. For the first administration release, follow the
[first deployment checklist](AUTH_RECOMMENDATION.md#11-first-production-administration-deployment-checklist)
to verify the image and populate the volume before starting public HTTP.
For subsequent releases after that setup, from the repository root:

```sh
just deploy --ha=false
```

Fly builds remotely by default; use `just deploy --local-only` to build with local
Docker. The app and service settings are in `fly.toml`; the service forwards traffic
to port 8080 and redirects HTTP to HTTPS. Supply `PORKMAIL_PWD` through Fly secrets,
never through tracked configuration.

The production Google client ID is configured in `fly.toml`. Before deploying
the configuration changes, provide `MAJ_APP_ORIGIN` and `MAJ_DATA_DIR` as ordinary
runtime settings, and install `GOOGLE_CLIENT_SECRET` through Fly secrets. The owner
has completed production Google setup and staged the secret; deployment is pending.
Production volume verification and real production Google sign-in still require
completion before an administration release. Uploads have been measured locally
against a 256 MiB budget; full-server memory on Fly remains unverified.

The server handles Ctrl+C (`SIGINT`) and `SIGTERM` by stopping new connections and
allowing active requests up to 25 seconds to finish. SMTP delivery has a 20-second
overall timeout. `fly.toml` gives the process 30 seconds before forced shutdown;
requests still unfinished after the server deadline are closed.

### Testing Google configuration locally

With Fish 3.7 or newer and the complete development build dependencies, run:

```fish
just dev-auth
```

The script defaults to the configured client ID for `maj-webpage-admin-development`;
paste its secret at the masked prompt. An existing `GOOGLE_CLIENT_ID` can override
that public default. The script clears the current clipboard
before starting Cargo and refuses to start if clearing fails. It supports
`wl-copy` on Wayland, `xclip` on X11, and Windows PowerShell through WSL interop.
The secret stays out of command history and files; never paste it into the script.
Saved clipboard history is separate and is not erased. Open `/admin` after startup
to test the development Google flow. Consent cancellation, approval, sign-out,
and the configured callback URL should also be checked manually before deployment.

## Content maintenance

The administrator page at `/admin` adds/removes gallery photos, shows, and YouTube
videos. Its navigation opens one editor at a time (`?section=gallery`, `shows`, or
`videos`); Gallery opens by default. The Published list below each add form includes
all current records, including content imported before the administrator signed in.
Select the item's confirmation box, then Remove to unpublish it. Successful changes
return to the same editor; ordinary browser Back/Forward also switches sections. Forms work without JavaScript. Removal requires checking the confirmation
box for that item; a stale removal cannot delete a newly created item because IDs
are never reused. Each successful submission redirects to a fresh catalog listing.
Existing visitors see changes on their next page load/resource reload; there is no
live push. Editable pages, APIs and gallery images use `Cache-Control: no-store`.

Before first use, stop the HTTP server and import the existing catalog into the
same `MAJ_DATA_DIR` used by the server:

```sh
just admin import-content --source public
```

Import refuses to overwrite an initialized catalog. Empty catalogs stay empty;
there is no automatic filesystem fallback. Back up an existing database before
upgrading: the content ID migration preserves rows while preventing ID reuse.

### Gallery uploads and removal

Upload one JPEG, PNG or WebP at a time. Limits are **10 MiB per file**, **12 million
pixels**, and **6000 pixels per side**, with a 96 MiB decoder allocation limit.
The multipart request allows an additional 16 KiB for form framing; each text
field is bounded separately. A process-wide slot covers receiving and decoding,
with a 30-second body deadline and immediate rejection while another submission
is processing. Decoding runs on a blocking worker; disconnecting does not release
its slot while the decoder is still working.

The server inspects actual bytes, applies EXIF orientation, and writes pixels only,
removing metadata. Full-size JPEGs are re-encoded as JPEG; PNG/WebP become 8-bit PNG
(preserving transparency). Thumbnails are center-cropped 480×270 WebP. Upload names
use lowercase letters with spaces replaced by underscores; letters, numbers,
underscores and hyphens are accepted in the stem. Client paths are rejected.
The output format determines the extension. A duplicate normalized name returns
409 without overwriting existing files. Rename the source file before retrying.

Both outputs are completed and synced in private staging before atomic no-replace
publication and database commit. Removal deletes the row first, then the files.
Failed cleanup or an ambiguous commit can leave an unreferenced private file; it
is never listed or served. Cleanup failures are logged without file contents.
Do not delete suspected orphans while a server/import is writing; compare against
catalog references under an offline maintenance lock and retain a backup first.

Gallery URLs retain `/img/about_pics/<name>` and
`/img/about_thumbnails/<name>.webp`, including encoded imported names. Every request
checks publication. Deleted URLs return 404 even if an older built asset exists.

`just test-upload-memory` checks successful 12-megapixel JPEG, PNG and WebP uploads
in fresh local processes, including rotation, transparency and 16-bit PNG. On
2026-11-17, peak resident memory ranged from **87.0 to 154.7 MiB**. Normalizing to
8-bit pixels before rotation reduced the rotated 16-bit case from 199.3 to
154.7 MiB without changing the output. The check allows 192 MiB, reserving 64 MiB
of the intended **256 MiB** Machine for HTTP, SQLite, input buffers and other work.

This measures the isolated upload pipeline; it does not establish full-server
memory under traffic or on Fly. Fixtures reach the pixel/side boundaries and
compress below the 10 MiB byte limit. Rerun after image-library, limit or processing
changes; monitor deployed memory and volume capacity. Keep one processing job per
server. See [BUILD.md](BUILD.md#rust-tests-and-checks) for test dependencies and execution.

### Shows and YouTube videos

Shows require a real calendar date. Event name, venue and location remain optional,
with a 500-byte limit per text field and no control characters. Dates are stored as
ISO dates and displayed in the existing day.month.year format, sorted chronologically.
YouTube forms accept recognized HTTPS watch, short, live, embed and `youtu.be`
URLs. The server validates exact hosts and the 11-character ID, stores that ID,
and builds the public embed URL. It never fetches a submitted URL or accepts iframe HTML.
Duplicate video IDs return 409. Server validation is authoritative; browser checks
only help the administrator complete the form.

### Bundled assets and seed content

Biography, album artwork, styles, music and other static assets remain source
changes in Rust, `public/` and `style/` and require rebuilding/releasing. The JSON
files and gallery photos under `public/` are now **one-time import sources**, not
live content. Editing them does not change an initialized database.

`just import-content /path/to/content.zip` still validates/replaces the bundled
show/video JSON; use it only to prepare a fresh import source. `just thumbnails`
still uses ImageMagick to prepare 480×270 WebP thumbnails for bundled originals.
Online uploads generate their own thumbnails and need no ImageMagick. Keep source
picture names lowercase with underscores, album art in `public/img/albums/`, and
matching `<original filename>.webp` names for bundled gallery thumbnails.

## Keyboard controls and dialogs

Gallery thumbnails are labeled buttons: activate them with Enter or Space. The
photo viewer has named previous, next, and close buttons. Audio seeking and volume
use range inputs with arrow-key, Home, and End support. Seeking stays disabled until
the duration is known; volume and mute controls stay synchronized. The player uses
the Minor Flaw promo. Volume uses a vertical slider: Up increases volume and Down
decreases it. It sits to the right of the mute button, inside the player in both
orientations, and stays fixed when hovered. Media videos use responsive 4:3 frames
with fullscreen support and display an error if video content cannot be loaded.
Shows keeps a readable list area and allows the page to scroll at high magnification.
Album detail pages scroll to reveal all notes, artwork,
service links, and the return link.

Contact, external-link confirmation, and photo dialogs move focus inside, make the
background inert, and restore focus when closed. Escape, close buttons, and clicking
outside dismiss them. Clicking dialog padding or dragging a text selection outside
does not dismiss them.
External sites opened in a new tab cannot access the original tab through `window.opener`.
The biography color button is named "Change biography text color". Background
framing follows the current route, including nested pages, Back/Forward, and reloads.

Contact drafts survive dismissal and failed submissions. Errors are brought into
view. A successful send resets the form and closes the dialog. See
[contact form protections](ARCHITECTURE.md#contact-form-protections) for validation
and send limits.

## Coverage and dependency checks

`just coverage` measures native Rust line coverage with cargo-tarpaulin. The
initial baseline on 2026-11-12 is **69.25%** (831/1,200 lines), with all 27 Rust
tests passing. Reports are in `target/coverage/`. This excludes browser/WASM
execution and the script tests; coverage should improve through useful tests for
changed behaviour, without an initial percentage gate.

`just security` runs cargo-audit and cargo-deny with current advisory data.
The 2026-11-17 follow-up fixed the SQLite advisory and documented temporary
acceptance of `paste` maintenance risk below. The refreshed database also reports
unmaintained `chrono` 0.4.45
([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html));
cargo-deny still fails on that separate release blocker. Cargo-audit reports the
two maintenance warnings; bans and sources checks pass. There is no Chrono exception.
See [BUILD.md](BUILD.md#dependency-security-checks) for commands and prerequisites.

### Patched SQLite dependency

SQLx's stable 0.9.0 release caps `libsqlite3-sys` below the versions patched for
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html).
`Cargo.toml` therefore patches SQLx to the first upstream fix,
[`10462fa4414c6c7f1ed1d39bb31678e3b726878c`](https://github.com/launchbadge/sqlx/commit/10462fa4414c6c7f1ed1d39bb31678e3b726878c)
(SQLx #4502). This permits the locked `libsqlite3-sys` 0.39.1, which bundles SQLite
3.54.1. All SQLx workspace crates come from the same revision, including the
session store's SQLx dependency; there is no custom fork or system-library setup.
The database initialization test queries the actual linked SQLite version and
requires at least 3.54.0, so checking the lockfile alone is insufficient.

`deny.toml` allows only this named Git repository, requires revision-pinned Git
sources, and continues to deny unknown repositories/registries. Review the pin by
**2026-12-17**, on dependency/advisory changes, and before release. Switch back to
a compatible stable SQLx release that permits patched SQLite when available,
remove the Cargo patch and Git allowance, update the lockfile, and rerun security,
database, migration, session, and backup/recovery checks. Do not select a moving branch.
Use the application operator commands for database maintenance; the separately
installed SQLx CLI used to create migration files has its own dependency graph.

### Paste maintenance-risk acceptance

Reviewed **2026-11-17** for `paste` **1.0.15** under
[RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html).
The current stable `leptos` 0.8.23, `either_of` 0.1.9, `reactive_graph` 0.2.16,
`reactive_stores` 0.4.4, and `tachys` 0.2.19 depend on it. There is no patched
`paste` release or compatible stable framework update that removes these uses.

The accepted risk is lack of future maintenance: this advisory reports no known
vulnerability. `paste` is a compile-time procedural macro over trusted Rust source;
public requests/uploads are not supplied to it. Its generated code is exercised
by server/browser checks. This does not protect against a compromised dependency
or make an abandoned crate maintained. Cargo-deny permits only this advisory ID
with a documented reason; **cargo-audit keeps its warning visible**. Other advisory
IDs and source checks remain enforced.

Reassess by **2026-12-17**, whenever dependencies/advisories change, and before
adding new macro usage. Remove the exception when the stable framework replaces
`paste`; do not extend it to a newly reported vulnerability without a fresh
applicability review. This resolves the audit's missing maintenance decision,
not the upstream maintenance status.

### RSA advisory applicability

Reviewed **2026-11-16** for `openidconnect` **4.1.0** and its `rsa` **0.9.10**
dependency. Both [.cargo/audit.toml](.cargo/audit.toml) and [deny.toml](deny.toml)
except [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html);
cargo-deny's separate Paste exception is described above.
This private-key timing advisory does not apply to our Google sign-in verification:
the library constructs an `RsaPublicKey` from Google's published modulus/exponent
and verifies an RS256 signature. The application supplies no RSA private key and
does not call RSA signing or decryption. Encrypted ID tokens and non-RS256
signatures are rejected; the OAuth client secret is used only for HTTPS token
exchange. Local fixture tokens are signed separately with AWS-LC.

The reviewed path is `verify_rsa_signature` → `RsaPublicKey::verify` → public
exponentiation and signature-padding verification. Private-key APIs exposed by
the dependency are outside this path. See the
[OpenID Connect implementation](https://github.com/ramosbugs/openidconnect-rs/blob/main/src/core/crypto.rs)
and [RSA verification implementation](https://github.com/RustCrypto/RSA/blob/v0.9.10/src/pkcs1v15.rs).

Review this exception by **2026-12-16**, whenever dependencies or the advisory
change, and before adding any new RSA use. Remove it once upstream adopts patched
`rsa >=0.10`. The configurations suppress this specific advisory ID, so each
dependency change must check for new RSA paths; they do not enforce call-path
reachability. This RSA exception does not suppress the SQLite or maintenance
advisories; their fixes and individual policies are documented above.

## Logs and privacy

Contact delivery logs contain only `delivery=sent` or `delivery=failed` within the
request span (`req`, with its `id` from `x-request-id`). Use that ID to correlate
delivery results with HTTP request logs. The handler does not log names, email
addresses, subjects, message bodies, or raw SMTP replies.
The public API returns a fixed delivery failure message for SMTP errors; provider
diagnostics are discarded before server-function serialization. Delivery still
has a 20-second deadline and a separate fixed timeout message.
Missing or empty mail credentials produce a separate static configuration error;
the credential value is never logged.

Earlier private data already present in hosting logs is not removed by code changes.
Manage log access and retention through the hosting provider.
