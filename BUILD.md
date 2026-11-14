# Build and test instructions

Use the [justfile](justfile) as the project's command runner. Run `just` or
`just --list` to list commands. Recipes run from the repository root, including
when invoked from a subdirectory, and use Bash regardless of your interactive
shell. Run the installation commands below from the repository root.
For runtime configuration, content maintenance, and deployment, see [README.md](README.md).
For application design, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Dependencies

For native Linux builds, install:

- Rust 1.89 or newer and Cargo through rustup (native file locking requires 1.89).
- [`just`](https://just.systems/man/en/), the project command runner.
- The `wasm32-unknown-unknown` Rust target.
- `cargo-leptos`, which builds the server, browser WASM, and SCSS assets.
- A C/C++ build toolchain, Clang, and LLD. The x86_64 Linux linker configuration
  in `.cargo/config.toml` uses Clang with LLD.
- OpenSSL development headers and `pkg-config` for Lettre's native TLS support.
- CA certificates for dependency downloads and SMTP TLS certificate verification.

SQLx builds bundled SQLite with the C toolchain; no external database server or
SQLite development package is needed. For creating migration files, install the
matching SQLx CLI (0.9):

```sh
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features sqlite
```

On Debian or Ubuntu, install the system dependencies with:

```sh
sudo apt-get update
sudo apt-get install build-essential clang lld pkg-config libssl-dev ca-certificates
```

With Rust and Cargo installed:

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
cargo install just --locked
```

Docker builds require a running Docker daemon; the builder image installs Rust
build tools itself. Browser tests additionally require Node.js 20 or newer and npm.

For coverage and dependency security checks, also install:

```sh
cargo install cargo-tarpaulin --locked
cargo install cargo-audit --locked
cargo install cargo-deny --locked
```

## Development build

Build and serve the complete site with automatic reload:

```sh
just dev
```

Open `http://localhost:8080`. Automatic reload uses port 8081. This recipe supplies
local runtime settings and removes Google and SMTP credentials. Use
`just dev-auth` for the interactive Google credential launcher described in
[README.md](README.md#testing-google-configuration-locally).

To build without starting a server:

```sh
just build
```

The server binary is `target/debug/leptos-website-maj`; browser assets and copied
public files are in `target/site/`. The component-erasure flag matches the Docker
build and is set by the justfile for builds and checks. `just run` starts the
existing debug binary without rebuilding; `just dev-source` runs against source
content without building browser assets.

During continuous development, run `just clean` (`cargo clean`) daily between
sessions to prevent incremental compilation and compiled dependencies from
filling the drive. Stop watchers/builds first and preserve needed coverage reports.
Cleanup removes generated assets and binaries too, so the next run requires a
rebuild. See [development disk usage](README.md#development-disk-usage) for details.

## Release build

```sh
just build-release
```

The server binary is `target/release/leptos-website-maj`. Keep `target/site/` with
it when preparing a runtime environment. Generated `target/site/` and `pkg/`
output must not be edited or committed. `just run-release` runs this binary with
local development settings; see README for production runtime configuration.

## Docker build

```sh
just docker-build
```

The multi-stage Dockerfile builds release server and browser assets and packages
these with runtime OpenSSL libraries and CA certificates. The build context excludes
`target/`, so local build output is rebuilt inside the container.

The builder selects the x86_64 or aarch64 `cargo-binstall` archive using its own
architecture (`uname -m`), saving it as `cargo-binstall.tgz`. For `--platform`
builds, configure a native builder or emulation for the requested platform.

## Rust tests and checks

Run the full Rust test suite without SMTP credentials:

```sh
just test
```

The tests do not send email. They cover contact validation and rate limiting,
content loading, show dates, gallery thumbnails, static file responses, and album
route HTTP status codes. The recipe removes SMTP and Google credentials. For a
focused group, use `just test --lib FILTER` with one of these filters:

| Filter | Coverage |
| --- | --- |
| `routes::home::tests` | Validation, rolling expiry, concurrent attempts, rejected HTTP requests |
| `serve::tests` | File responses and content APIs using the configured site root |
| `routes::shows::tests` | Calendar dates, sorting, and content loading |
| `routes::about::tests` | Gallery loading, error rendering, and thumbnail assets |
| `routes::media::tests` | Video content loading |
| `server_config::tests` | Origin/path validation, required production credentials, local operation without Google, and private errors |
| `storage::tests` | Private modes/layout, preservation of data, public path separation, and rejection of unsafe links |
| `database::tests` | Pooled connection settings, migrations/reopen, constraints, corruption, writer contention, and concurrent login limits across manager clones |
| `cli::tests` | Clap validation, explicit approvals, idempotency, revocation, and generation overflow |
| `auth::tests` | Cookie policy, expiry, revocation, CSRF/Origin, PKCE state binding/replay, and transaction bounds |
| `content::tests` | YouTube validation, import rollback/repeat safety, private files, and checksummed backup/restore |

Run the album HTTP regression test separately with:

```sh
just test --test album_routes
```

Run content-import and development-launcher regression checks with:

```sh
just test-scripts
```

These require Python 3 with its standard library, Fish 3.7 or newer, and just.
Temporary directories cover valid imports, archive path and link rejection,
content validation, and preservation of existing files on invalid input. Dummy
credentials and mocked clipboard/Cargo commands check clipboard clearing before
launch, failed clearing, empty input, and credential handling. The tests exercise
the launcher's just recipe without accessing the real clipboard, Google, or SMTP.

Build the debug server and run the Linux shutdown regression checks with:

```sh
just test-shutdown
```

These start isolated servers on automatically selected localhost ports, without
SMTP credentials. They check complete downloads during SIGINT and SIGTERM and the
25-second shutdown deadline with a stalled download. They also check private
storage modes/umask and that database/staging files cannot be downloaded, including
through traversal requests. Database command tests check repeat migrations without
HTTP/provider configuration and startup rejection for corruption, missing Fly
storage, and unknown commands. Operator regressions import real bundled content,
exercise identity commands and cross-process locks, and verify backups of both
current and pre-migration databases. The deadline check takes about 25 seconds. Python's
standard library is sufficient.

Check formatting and lint both compilation targets:

```sh
just fmt-check
just lint
```

Use `just fmt` to format the justfile and Rust source. `just lint-ssr` and `just lint-hydrate`
run each target separately. `just check` combines formatting checks, both linters,
Rust tests, and script tests; shutdown and browser tests are separate.

Recipes that accept arguments forward them without shell interpolation. Examples:
`just test --offline --lib server_config::tests` and `just build --cargo-offline`
use cached dependencies. Build and test recipes enforce the existing lockfile.

## Rust coverage

Run the native Rust unit and integration tests with cargo-tarpaulin:

```sh
just coverage
```

The recipe uses `ssr` and the LLVM backend, preserves the component-erasure flag,
and removes Google/SMTP credentials. Instrumented builds are isolated in
`target/tarpaulin/`; reports are written to `target/coverage/tarpaulin-report.html`
and `target/coverage/lcov.info`. Reports are ignored under `target/`, and raw LLVM
profile files are ignored separately. No coverage percentage currently causes a
failure, but failing tests still fail the command.

The first run rebuilds instrumented dependencies and can take longer than normal
tests. To limit compiler memory and use cached crates, run
`env CARGO_NET_OFFLINE=true just coverage --jobs 2 --offline` after the dependency
metadata and sources are cached. The report measures native Rust code; it does
not include browser/WASM execution or the Python/Fish script tests. Continue to
run `just test-scripts` and the browser checks separately.

See the [Tarpaulin documentation](https://github.com/xd009642/tarpaulin) for
additional command options.

## Dependency security checks

```sh
just audit
just deny
```

`just security` runs both commands. cargo-audit checks `Cargo.lock`; cargo-deny
uses [deny.toml](deny.toml) with all features enabled for dependency graph analysis,
without compiling `ssr` and `hydrate` together. Its selected checks are advisories,
bans, and sources. Both commands retain their nonzero exit status on failure;
no advisory exceptions are configured.

Normal runs need network access to refresh advisory data and dependency metadata.
For an explicitly cached development check after the databases are populated,
use `just audit --no-fetch` or `just deny --offline`. A missing/stale database
may still fail. These checks are separate from `just check` and do not upload
reports. See [RustSec](https://rustsec.org/) and the
[cargo-deny configuration reference](https://embarkstudios.github.io/cargo-deny/checks/cfg.html).

## Browser checks

Build the complete application and start a separate browser-test server without
SMTP or Google credentials:

```sh
just test-server
```

Leave that process running on `127.0.0.1:8082`. In another terminal, install browser
test dependencies if needed:

```sh
just e2e-setup
```

Installing browser system libraries may require administrator privileges. Run all
three browser projects with:

```sh
just e2e
```

To run only installed Chromium and Firefox browsers:

```sh
just e2e --project=chromium --project=firefox
```

The just recipe defaults to `http://127.0.0.1:8082`, matching `just test-server`.
To test an existing development server, use
`env PLAYWRIGHT_BASE_URL=http://127.0.0.1:8080 just e2e`.
Direct Playwright invocations still use the configuration's default port 8080.
Playwright does not start a server. Stop the test server with Ctrl+C when finished.

Tests cover the homepage, keyboard and dialog focus behavior, photo navigation,
portrait sizing, thumbnail loading, audio event synchronization, rejected playback,
mocked contact success/failure, new-tab isolation, video errors, and show rows at
high zoom, gallery filename collisions, accessible icon names, and background
positions across history and reloads. External pages and email responses are
mocked; no email is sent.
The light-dismiss fallback is tested with native light dismiss disabled, and audio
checks guard against reintroducing the old 10 ms polling interval.
