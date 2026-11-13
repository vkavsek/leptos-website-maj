# Maj Kavšek's portfolio website

A personal website for trumpeter and composer Maj Kavšek, with a biography,
photo gallery, show listings, discography, videos, audio playback, and a contact form.
Built with Rust, Leptos, and Axum.

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
| `LEPTOS_SITE_ROOT` | Directory containing built assets and content; defaults to `target/site` for local builds |
| `LEPTOS_SITE_ADDR` | Server bind address; defaults to `0.0.0.0:8080` |
| `RUST_LOG` | Required logging filter for release binaries; use `info` for normal operation |
| `APP_ENVIRONMENT` | `development` (loopback origin only) or `production`; defaults to development in debug builds and production in release builds |
| `MAJ_APP_ORIGIN` | Required canonical browser origin, without a trailing slash; production requires a public HTTPS hostname |
| `MAJ_DATA_DIR` | Required absolute private data path outside public assets; startup prepares storage and migrates SQLite before listening |
| `GOOGLE_CLIENT_ID` | OAuth client ID, paired with the client secret; required in production |
| `GOOGLE_CLIENT_SECRET` | Server-only OAuth secret; use hosting secrets in production or the interactive development launcher |

Never commit credentials or generated secret configuration. Builds and automated
tests do not need SMTP or Google credentials. Google sign-in and administration
are not implemented yet; these settings currently validate configuration and
make it available to server handlers. Missing or invalid production settings
stop startup without logging their values. Local development may omit both
Google credentials; a partial or placeholder pair is rejected.

At startup, the server creates `gallery/originals/`, `gallery/thumbnails/`, and
`staging/` beneath `MAJ_DATA_DIR`, plus `admin.sqlite3`. Directories
use mode `0700` and the database file mode `0600`. A process creation mask of
`0077` is set before Tokio starts, covering future companion/upload files. This
setup requires Unix filesystem permissions. SQLite contains the administrator
approval table and the library's session table. New databases have zero approved
administrators. Login and editing are still to implement; public content continues
loading from `LEPTOS_SITE_ROOT`.

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
permissions named in a startup error. The server does not automatically chmod,
chown, truncate, or migrate existing data. Keep runtime data outside the Docker
build context as well as the public trees.

`LEPTOS_SITE_ROOT` remains the built asset directory (`/app/site` in Docker).
Data and public roots may not overlap, including through aliases. Public asset
trees must be symlink-free; startup rejects even dangling links that could expose
data created later. Keep built assets immutable while the server runs. No handler
serves private storage; gallery publication through controlled lookups remains to
implement. Biography updates still require editing Rust and releasing.

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
Stop writers and use SQLite's backup API or a coordinated backup of database and
gallery files. Automatic backups and the administration backup command remain pending.

**Database dependency release blocker (2026-11-13):** current advisory checks flag
bundled `libsqlite3-sys` 0.37.0 under
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html).
A crafted database/WAL pair can expose process memory. Private storage restricts
who can supply those files, but does not patch SQLite. SQLx 0.9.0 requires
`libsqlite3-sys <0.38.0`, while the fix requires `>=0.39.0`; the current SQLx release
cannot resolve the patched crate. Deployment requires a supported patched SQLite
dependency path. No advisory exception has been added. The existing unmaintained
`paste` advisory also remains.

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
before deploying. From the repository root:

```sh
just deploy
```

Fly builds remotely by default; use `just deploy --local-only` to build with local
Docker. The app and service settings are in `fly.toml`; the service forwards traffic
to port 8080 and redirects HTTP to HTTPS. Supply `PORKMAIL_PWD` through Fly secrets,
never through tracked configuration.

The production Google client ID is configured in `fly.toml`. Before deploying
the configuration changes, provide `MAJ_APP_ORIGIN` and `MAJ_DATA_DIR` as ordinary
runtime settings, and install `GOOGLE_CLIENT_SECRET` through Fly secrets. The production secret is
still pending. Volume setup, Google login, and the editor
require further implementation;
do not treat the configuration foundation as a completed administration release.

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
Saved clipboard history is separate and is not erased. The launcher currently
tests configuration; actual Google sign-in is still to implement.

## Content maintenance

Edit source assets in `public/` and styles in `style/`. Rebuild to copy source
assets into the default `target/site/` directory. Deploy or restart the server
after content changes: content loaders cache both successes and failures for the
life of the process.

The configured site root must contain:

- `data_json/shows.json`: show records. Every record needs a real calendar date in
  day.month.year format, such as `29.02.2024`. Missing, null, empty, malformed, or
  impossible dates reject the whole file.
- `data_json/yt_videos.json`: video records.
- `img/about_pics/`: original gallery photos. Regular AVIF, GIF, JPEG (`.jpg` or
  `.jpeg`), PNG, SVG, and WebP files are listed, regardless of extension case.
  Links, subdirectories, other files, and filenames that are not valid UTF-8 are ignored.
- `img/about_thumbnails/`: generated WebP thumbnails for the gallery.

Correct a failed content file and restart or redeploy to clear its cached error.
Changing `LEPTOS_SITE_ROOT` also requires a restart.

### Importing show and video content

With Python 3 installed, run from the repository root:

```sh
just import-content /path/to/content.zip
```

The ZIP must contain `shows.json`, `yt_videos.json`, or both at its top level.
Only regular files are accepted; links, directories, unexpected paths, and duplicate
names are rejected. Every file is validated before existing content is replaced:
shows need valid dates and string-or-null fields; videos need URL strings.
Invalid archives leave existing content unchanged. Each accepted file is replaced
atomically. Rebuild and restart or redeploy after importing.

### Gallery thumbnails

After adding or replacing original photos, install ImageMagick with WebP support
and the `convert` command, then run from the repository root:

```sh
just thumbnails
```

The script applies EXIF orientation, crops to 480×270, and strips thumbnail metadata.
Originals stay unchanged. Commit thumbnails alongside their source photos. When
deleting an original, delete its matching `<original filename>.webp` thumbnail.
A Rust test checks that every original has a smaller WebP thumbnail.
Gallery URLs encode filenames, so spaces, Unicode, `#`, `?`, and `%` are supported.
Next and Previous compare complete filenames and wrap at either end.

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
The 2026-11-12 check found no known vulnerabilities, but Leptos transitively uses
unmaintained `paste` 1.0.15 ([RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)).
Cargo-audit reports a warning; cargo-deny's advisory check fails under the current
policy. Its bans and sources checks pass. No exception suppresses this finding.
See [BUILD.md](BUILD.md#dependency-security-checks) for commands and prerequisites.

## Logs and privacy

Contact delivery logs contain only `delivery=sent` or `delivery=failed` within the
request span (`req`, with its `id` from `x-request-id`). Use that ID to correlate
delivery results with HTTP request logs. The handler does not log names, email
addresses, subjects, message bodies, or raw SMTP replies.
Missing or empty mail credentials produce a separate static configuration error;
the credential value is never logged.

Earlier private data already present in hosting logs is not removed by code changes.
Manage log access and retention through the hosting provider.
