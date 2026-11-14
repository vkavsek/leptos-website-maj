set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

export RUSTFLAGS := "--cfg erase_components"

# List project commands.
default:
    @just --list

# Rebuild and serve public pages on localhost:8080 without Google or SMTP credentials.
dev:
    env -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET just _watch

# Prompt for development Google credentials and clear the clipboard before serving.
dev-auth:
    @fish --private scripts/dev-google-auth.fish

[private]
_watch:
    env -u PORKMAIL_PWD APP_ENVIRONMENT=development MAJ_APP_ORIGIN=http://localhost:8080 \
      MAJ_DATA_DIR="${MAJ_DATA_DIR:-$HOME/.local/share/maj-admin-dev}" cargo leptos watch

# Serve source content without building browser assets.
dev-source *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET \
      APP_ENVIRONMENT=development MAJ_APP_ORIGIN=http://localhost:8080 \
      MAJ_DATA_DIR="${MAJ_DATA_DIR:-$HOME/.local/share/maj-admin-dev}" \
      LEPTOS_SITE_ROOT=public cargo run --locked --features ssr "$@"

# Build the debug server, browser WASM, and styles.
build *args:
    cargo leptos build --cargo-locked "$@"

# Build release server and browser assets.
build-release *args:
    cargo leptos build --release --cargo-locked "$@"

# Daily development cleanup: remove build output, generated site assets, and coverage reports.
clean:
    cargo clean

# Run the existing debug binary locally without credentials.
run:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET \
      APP_ENVIRONMENT=development MAJ_APP_ORIGIN="${MAJ_APP_ORIGIN:-http://localhost:8080}" \
      MAJ_DATA_DIR="${MAJ_DATA_DIR:-$HOME/.local/share/maj-admin-dev}" \
      RUST_LOG="${RUST_LOG:-info}" ./target/debug/leptos-website-maj

# Run the existing release binary locally without credentials.
run-release:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET \
      APP_ENVIRONMENT=development MAJ_APP_ORIGIN="${MAJ_APP_ORIGIN:-http://localhost:8080}" \
      MAJ_DATA_DIR="${MAJ_DATA_DIR:-$HOME/.local/share/maj-admin-dev}" \
      RUST_LOG="${RUST_LOG:-info}" ./target/release/leptos-website-maj

# Run the existing release binary with provisioned production settings and secrets.
run-production:
    env APP_ENVIRONMENT=production RUST_LOG="${RUST_LOG:-info}" ./target/release/leptos-website-maj

# Run Rust tests; additional Cargo arguments and test filters are forwarded.
test *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET cargo test --locked --features ssr "$@"

# Initialize/migrate the selected database without HTTP or provider credentials.
db-migrate:
    just admin migrate

# Run a private operator command; use `just admin --help` for available subcommands.
admin *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET \
      MAJ_DATA_DIR="${MAJ_DATA_DIR:-$HOME/.local/share/maj-admin-dev}" \
      RUST_LOG="${RUST_LOG:-info}" cargo run --locked --features ssr -- admin "$@"

# Create a versioned SQL migration with sqlx-cli; review it before applying.
migration-add name:
    cargo sqlx migrate add "$1"

# Measure native Rust test coverage; write HTML and LCOV reports under target/coverage.
coverage *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET \
      cargo tarpaulin --locked --features ssr --tests --engine llvm \
      --target-dir target/tarpaulin --output-dir target/coverage --out Html Lcov "$@"

# Check Cargo.lock against the current RustSec advisory database.
audit *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET cargo audit "$@"

# Check dependency advisories, bans, and sources; forward cargo-deny's global options.
deny *args:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET cargo deny --locked "$@" check advisories bans sources

# Run both dependency security checks; requires current advisory data.
security: audit deny

# Test content imports and the Fish credential launcher with temporary data and mocks.
test-scripts:
    python3 scripts/test_update_json.py
    fish --no-config --no-execute scripts/dev-google-auth.fish
    python3 scripts/test_dev_google_auth.py

# Build the server and test graceful shutdown with real localhost connections.
test-shutdown: _build-server
    python3 scripts/test_shutdown.py

[private]
_build-server:
    cargo build --locked --features ssr

# Format the justfile and Rust source.
fmt:
    just --fmt
    cargo fmt --all

# Check justfile and Rust formatting.
fmt-check:
    just --fmt --check
    cargo fmt --all -- --check

# Lint server and browser targets.
lint: lint-ssr lint-hydrate

# Lint all server-enabled Rust targets.
lint-ssr:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET cargo clippy --locked --features ssr --all-targets -- -D warnings

# Lint browser WASM without server-only features.
lint-hydrate:
    env -u PORKMAIL_PWD -u GOOGLE_CLIENT_ID -u GOOGLE_CLIENT_SECRET cargo clippy --locked --lib --no-default-features --features hydrate --target wasm32-unknown-unknown --target-dir target/front -- -D warnings

# Run formatting checks, both linters, Rust tests, and script tests.
check: fmt-check lint test test-scripts

# Build assets and serve an isolated browser-test server on 127.0.0.1:8082.
test-server: build
    env MAJ_APP_ORIGIN=http://localhost:8082 LEPTOS_SITE_ADDR=127.0.0.1:8082 \
      MAJ_DATA_DIR="$HOME/.local/share/maj-admin-test" just run

# Install Playwright dependencies and browser/system packages.
e2e-setup:
    cd end2end && npm ci
    cd end2end && npx playwright install --with-deps chromium firefox webkit

# Run browser tests against the separately started test-server; forward Playwright arguments.
e2e *args:
    cd end2end && PLAYWRIGHT_BASE_URL="${PLAYWRIGHT_BASE_URL:-http://127.0.0.1:8082}" npx playwright test "$@"

# Build the Docker image.
docker-build:
    docker build -t maj-web .

# Run the Docker image locally without Google or SMTP credentials.
docker-run *args:
    docker run --rm -p 8080:8080 --env APP_ENVIRONMENT=development \
      --env MAJ_APP_ORIGIN=http://localhost:8080 --env MAJ_DATA_DIR=/data "$@" maj-web

# Deploy to Fly after production configuration and secrets are provisioned.
deploy *args:
    fly deploy --app maj-webpage "$@"

# Validate and import shows/videos from a ZIP; quote paths containing spaces.
import-content archive:
    bash scripts/update_json.sh "$1"

# Generate gallery thumbnails; requires ImageMagick with WebP support.
thumbnails:
    sh scripts/generate-gallery-thumbnails.sh
