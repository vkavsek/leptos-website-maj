# Build and test instructions

Run commands from the repository root unless another directory is specified.
For runtime configuration, content maintenance, and deployment, see [README.md](README.md).
For application design, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Dependencies

For native Linux builds, install:

- Rust and Cargo through rustup.
- The `wasm32-unknown-unknown` Rust target.
- `cargo-leptos`, which builds the server, browser WASM, and SCSS assets.
- A C/C++ build toolchain, Clang, and LLD. The x86_64 Linux linker configuration
  in `.cargo/config.toml` uses Clang with LLD.
- OpenSSL development headers and `pkg-config` for Lettre's native TLS support.
- CA certificates for dependency downloads and SMTP TLS certificate verification.

On Debian or Ubuntu, install the system dependencies with:

```sh
sudo apt-get update
sudo apt-get install build-essential clang lld pkg-config libssl-dev ca-certificates
```

With Rust and Cargo installed:

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
```

Docker builds require a running Docker daemon; the builder image installs Rust
build tools itself. Browser tests additionally require Node.js 20 or newer and npm.

## Development build

Build and serve the complete site with automatic reload:

```sh
env RUSTFLAGS="--cfg erase_components" cargo leptos watch
```

Open `http://localhost:8080`. Automatic reload uses port 8081.

To build without starting a server:

```sh
env RUSTFLAGS="--cfg erase_components" cargo leptos build --cargo-locked
```

The server binary is `target/debug/leptos-website-maj`; browser assets and copied
public files are in `target/site/`. The component-erasure flag matches the Docker
build and is also used by the checks below.

## Release build

```sh
env RUSTFLAGS="--cfg erase_components" cargo leptos build --release --cargo-locked
```

The server binary is `target/release/leptos-website-maj`. Keep `target/site/` with
it when preparing a runtime environment. Generated `target/site/` and `pkg/`
output must not be edited or committed.

## Docker build

```sh
docker build -t maj-web .
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
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --features ssr
```

The tests do not send email. They cover contact validation and rate limiting,
content loading, show dates, gallery thumbnails, static file responses, and album
route HTTP status codes. For a focused group, append one of these filters to
`cargo test --locked --lib --features ssr`, keeping the same environment settings:

| Filter | Coverage |
| --- | --- |
| `routes::home::tests` | Validation, rolling expiry, concurrent attempts, rejected HTTP requests |
| `serve::tests` | File responses and content APIs using the configured site root |
| `routes::shows::tests` | Calendar dates, sorting, and content loading |
| `routes::about::tests` | Gallery loading, error rendering, and thumbnail assets |
| `routes::media::tests` | Video content loading |

Run the album HTTP regression test separately with:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --features ssr --test album_routes
```

The content import regression checks require Python 3 with its standard library:

```sh
python3 scripts/test_update_json.py
```

They use temporary directories to check valid imports, archive path and link
rejection, content validation, and preservation of existing files on invalid input.

After building the debug server, run the Linux shutdown regression checks with:

```sh
python3 scripts/test_shutdown.py
```

These start isolated servers on automatically selected localhost ports, without
SMTP credentials. They check complete downloads during SIGINT and SIGTERM and the
25-second shutdown deadline with a stalled download. The deadline check takes about
25 seconds. Python's standard library is sufficient.

Check formatting and lint both compilation targets:

```sh
cargo fmt --all -- --check
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo clippy --locked --features ssr --all-targets -- -D warnings
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo clippy --locked --lib --no-default-features --features hydrate --target wasm32-unknown-unknown --target-dir target/front -- -D warnings
```

After dependencies are cached, add `--offline` to Cargo test/check commands or
`--cargo-offline` to `cargo leptos build` to avoid network access.

## Browser checks

Playwright does not start a server. Build the complete application and start a
separate test server without SMTP credentials:

```sh
env RUSTFLAGS="--cfg erase_components" cargo leptos build --cargo-locked
env -u PORKMAIL_PWD LEPTOS_SITE_ADDR=127.0.0.1:8082 ./target/debug/leptos-website-maj
```

Leave that process running. In a separate terminal, install browser test
dependencies from `end2end/`:

```sh
cd end2end
npm ci
npx playwright install --with-deps chromium firefox webkit
```

Installing browser system libraries may require administrator privileges. Run all
three browser projects with:

```sh
env PLAYWRIGHT_BASE_URL=http://127.0.0.1:8082 npx playwright test
```

To run only installed Chromium and Firefox browsers:

```sh
env PLAYWRIGHT_BASE_URL=http://127.0.0.1:8082 npx playwright test --project=chromium --project=firefox
```

The default base URL is `http://127.0.0.1:8080` if `PLAYWRIGHT_BASE_URL` is unset.
Stop the test server with Ctrl+C when finished.

Tests cover the homepage, keyboard and dialog focus behavior, photo navigation,
portrait sizing, thumbnail loading, audio event synchronization, rejected playback,
mocked contact success/failure, new-tab isolation, video errors, and show rows at
high zoom, gallery filename collisions, accessible icon names, and background
positions across history and reloads. External pages and email responses are
mocked; no email is sent.
The light-dismiss fallback is tested with native light dismiss disabled, and audio
checks guard against reintroducing the old 10 ms polling interval.
