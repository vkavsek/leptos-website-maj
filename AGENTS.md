# Repository Guidelines

## Project Structure & Module Organization

- `src/app.rs` defines the application shell and routes; `src/routes/` contains pages and `src/audio/` contains the audio player.
- `src/main.rs`, `src/serve.rs`, and `src/fallback.rs` handle the Axum server; `src/lib.rs` exposes browser hydration.
- `style/main.scss` loads SCSS partials. `public/` holds images, fonts, music, and content in `public/data_json/`.
- `end2end/tests/` contains Playwright tests. See `BUILD.md`, `Dockerfile`, and `fly.toml` for container builds and deployment.

## Build, Test, and Development Commands

Install Rust, the `wasm32-unknown-unknown` target, and `cargo-leptos`. Linux linker settings in `.cargo/config.toml` require Clang and LLD.

- `cargo leptos watch`: rebuild and serve locally at `http://localhost:8080` with automatic reload.
- `cargo leptos build --release`: build the server and browser WASM/CSS assets.
- `cargo test --features ssr`: run Rust tests with server code enabled.
- `cargo fmt --all -- --check`: check Rust formatting.
- `cargo clippy --features ssr --all-targets`: lint server-enabled Rust targets.
- From `end2end/`, run `npm ci`, `npx playwright install`, then `npx playwright test` to install and run browser tests.

## Coding Style & Naming Conventions

Use Rust 2021 and four-space indentation. Use `snake_case` for modules/functions and `PascalCase` for types and Leptos components. Match the existing two-space indentation in SCSS and TypeScript. Keep server-only code behind `ssr` and hydration code behind `hydrate`. Reuse existing components and SCSS theme variables.

## Testing Guidelines

No Rust tests or coverage threshold are currently defined. Add focused Rust tests alongside changed logic and browser tests as `end2end/tests/*.spec.ts`.

The existing Playwright spec still targets port 3000 and Leptos starter text. Update its URL and assertions for this app before relying on it. Start the server separately; Playwright's server startup configuration is commented out.

## Commit & Pull Request Guidelines

Recent commits use `FEAT:`, `FIX:`, or `UPDATE:` followed by short descriptions. Keep commits focused. PRs should describe behavior changes, list validation and known gaps, link related issues, and include screenshots for visible UI changes.

## Security & Generated Files

Supply `PORKMAIL_PWD` through the environment; never commit credentials. Treat `target/site/` and `pkg/` as generated output. Edit source assets in `public/` and `style/` instead.
