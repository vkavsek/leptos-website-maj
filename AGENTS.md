# Repository Guidelines

## Project Structure & Module Organization

- `src/app.rs` defines the application shell and routes; `src/routes/` contains pages and `src/audio/` contains the audio player.
- `src/main.rs`, `src/serve.rs`, and `src/fallback.rs` handle the Axum server; `src/lib.rs` exposes browser hydration.
- `style/main.scss` loads SCSS partials. `public/` holds images, fonts, music, and content in `public/data_json/`.
- `end2end/tests/` contains Playwright tests. See `BUILD.md`, `Dockerfile`, and `fly.toml` for container builds and deployment.

## Build, Test, and Development Commands

Install Rust, the `wasm32-unknown-unknown` target, `cargo-leptos`, and `just`. Linux linker settings in `.cargo/config.toml` require Clang and LLD. Use the root `justfile` for project commands; `just --list` lists recipes. Recipes use Bash even from Fish and run from the repository root.

- `just dev`: rebuild and serve locally at `http://localhost:8080` without Google/SMTP credentials.
- `just dev-auth`: prompt for development Google credentials and clear the clipboard before serving.
- `just build` / `just build-release`: build the server and browser WASM/CSS assets.
- `just clean`: run `cargo clean` for daily continuous-development cleanup.
- `just run` / `just run-release`: run an existing local build; `just dev-source` serves source content.
- `just test`: run Rust tests; append Cargo arguments or a test filter as needed.
- `just coverage`: measure native Rust coverage with cargo-tarpaulin; HTML/LCOV reports go to `target/coverage/`.
- `just audit` / `just deny`: run cargo-audit or cargo-deny; `just security` runs both.
- `just test-scripts` / `just test-shutdown`: run script/launcher checks or build and run shutdown checks.
- `just fmt` / `just fmt-check`: format the justfile and Rust or check formatting.
- `just lint`: lint server and browser targets; `just check` also checks formatting and runs Rust/script tests.
- `just test-server`: build and serve on port 8082. In another terminal, use `just e2e-setup`, then `just e2e`.

## Development Disk Usage

During continuous development, run `just clean` (`cargo clean`) daily at a natural
stopping point between work sessions. Incremental artifacts and compiled
dependencies across server, WASM, release, and coverage builds can fill the drive.
Stop watchers and active builds/tests first; preserve needed reports outside
`target/`. Cleanup also removes generated site assets and binaries, so rebuild
before running them again. Check free space before large build/test sessions and
avoid redundant builds. With the default target directory, cleanup also removes
the separate `target/front/` and `target/tarpaulin/` builds and coverage reports.

## Coding Style & Naming Conventions

Use Rust 2021 and four-space indentation. Use `snake_case` for modules/functions and `PascalCase` for types and Leptos components. Match the existing two-space indentation in SCSS and TypeScript. Keep server-only code behind `ssr` and hydration code behind `hydrate`. Reuse existing components and SCSS theme variables.

## Testing Guidelines

Add focused Rust tests alongside changed logic and browser tests as `end2end/tests/*.spec.ts`. No coverage threshold is defined. Browser tests require a separately running server; `just e2e` defaults to the `just test-server` address at `http://127.0.0.1:8082`. Set `PLAYWRIGHT_BASE_URL` to test another server.

Use cargo-tarpaulin through `just coverage` when changing non-trivial Rust logic.
Improve coverage gradually, prioritizing validation, error paths, authorization,
configuration, and operations that can lose data. Review uncovered changed code
and report material gaps with the validation results. There is no initial
percentage gate: add useful behavioural tests rather than tests that only raise
the number, and do not exclude application code to inflate coverage. The LLVM
report covers native `ssr` Rust tests, not browser/WASM execution or Python/Fish
scripts; keep their separate tests. Use `just coverage --jobs 2` to limit memory
during the separate instrumented build. Never provide real Google/SMTP secrets
or make live provider calls in automated tests.

## Dependency Security

Run `just security` after adding/updating dependencies or changing Cargo features,
and before deployment. `cargo-audit` checks `Cargo.lock` against RustSec;
`cargo-deny` uses `deny.toml` to inspect all feature dependencies for advisories,
yanked/unmaintained crates, wildcard requirements, and unexpected registry/Git
sources. Duplicate dependency versions are warnings. These recipes do not perform
license checks; a license policy has not been selected.

Use current advisory data for release decisions. Offline/cached checks are useful
during development but must be reported as such; a failed fetch is not a clean
audit. Investigate findings and report unresolved risks. Do not suppress
advisories, loosen source policy, or add broad exceptions merely to make a check
pass. Any necessary exception must identify the specific advisory/crate, explain
why it applies, and state when it should be reviewed. Security checks and coverage
are separate from the faster `just check` recipe.

## Commit & Pull Request Guidelines

Recent commits use `FEAT:`, `FIX:`, or `UPDATE:` followed by short descriptions. Keep commits focused. PRs should describe behavior changes, list validation and known gaps, link related issues, and include screenshots for visible UI changes.

## Security & Generated Files

Supply `PORKMAIL_PWD` through the environment; never commit credentials. Treat `target/site/` and `pkg/` as generated output. Edit source assets in `public/` and `style/` instead.

## Additional Guidelines

The project is being actively developed on multiple different machines. Avoid including machine specific instructions in general documentation. Keep instructions generic to Linux.

## Additional Resources

Use these additional resources when developing this project:

- "<https://book.leptos.dev>"
- "<https://docs.rs/axum/latest/axum/>"
- "<https://docs.rs/leptos/latest/leptos/>"
