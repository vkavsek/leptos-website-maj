What this repo does: This is Maj Kavšek’s portfolio, built with Leptos and Axum. Google sign-in protects an editor for gallery photos, shows, and YouTube videos; SQLite and private image files hold the published content. This audit assumes one region, one Machine, one volume, one or two administrators, and modest public traffic that includes bots.

Audit date: 2026-11-16. Reviewed commit `d58ea93` and the then-staged `fly.toml` changes. Later fixes are marked resolved below without renumbering the original findings.

**Must fix**

1. **RESOLVED: An in-flight request can restore a session after logout** (`src/auth/login.rs`, `src/database.rs`, `src/auth.rs`)

   - **What this is:** Each HTTP request loads its own session record, and the session middleware saves changed records after the handler finishes.
   - **Problem:** Before the fix, a slow authenticated request could finish after logout and recreate the deleted session under its old ID. A deterministic HTTP regression reproduced restored administrator access before the fix.
   - **Fix:** Logout now advances only that administrator's durable `session_generation` after authorization/Origin/CSRF checks and before flushing the session. All existing sessions for that account are invalidated without disabling approval or affecting other administrators; late session records can still be stored but cannot authorize access.
   - **Verification:** The HTTP regression pauses a request across logout and covers both a late session upsert and an editor-style delayed mutation. It checks old/same-account cookies, unaffected other administrators, fresh login, and cookie clearing; a separate test checks that generation exhaustion returns an error rather than successful logout.

2. **RESOLVED: The bundled SQLite dependency fails the security audit** (`Cargo.lock`, `Cargo.toml`, `src/database.rs`)

   - **What this is:** SQLx's SQLite feature bundles the database engine for application and session storage.
   - **Problem:** The original `libsqlite3-sys 0.37.0` was affected by [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html): crafted local database/WAL files could expose process memory. Stable SQLx 0.9.0 prevents selecting the patched crate.
   - **Fix:** Cargo now pins SQLx's upstream #4502 revision `10462fa4414c6c7f1ed1d39bb31678e3b726878c` and locks patched `libsqlite3-sys 0.39.1` (SQLite 3.54.1). The named Git source requires a revision; review the temporary pin by 2026-12-17 and remove it when a compatible stable release includes the fix.
   - **Verification:** Fresh advisory checks no longer report the SQLite vulnerability; there is no SQLite advisory exception. The database initialization test queries the linked engine and requires SQLite >=3.54.0.

**Should fix**

1. **Missing-page logging includes private query parameters** (`src/fallback.rs:30`)

   - **What this is:** The fallback handler logs the method and complete URI when a static file is missing.
   - **Problem:** A request such as `/missing?private=canary` writes the query value into production logs; a misspelled OAuth callback URL can similarly carry a temporary login code. The normal request tracing uses only the path, but this fallback bypasses that protection.
   - **Fix:** Log `parts.uri.path()` instead of the complete URI, and add a log-capture test using a harmless query marker.
   - **If we skip it:** Unmatched requests can copy private URL values into logs and retain them beyond the request.

2. **The contact API returns raw SMTP failures** (`src/routes/home.rs:478`)

   - **What this is:** The public contact endpoint sends mail through SMTP, the protocol used to deliver email.
   - **Problem:** `ServerFnError::from` forwards the mail provider’s error text to the caller, even though the page displays a generic failure message. A provider authentication or recipient rejection therefore exposes backend diagnostics to anyone calling the endpoint directly; this audit did not demonstrate password disclosure.
   - **Fix:** Return a fixed public delivery error for provider failures while keeping the existing safe delivery-status logs. Add a simulated transport failure and assert that its diagnostic marker is absent from the response.
   - **If we skip it:** The API’s error details remain dependent on what an external mail server returns.

3. **The production build can change the dependency lockfile** (`Dockerfile:27`, `justfile:32`)

   - **What this is:** Local build recipes use `--cargo-locked` to enforce the exact dependency versions recorded in `Cargo.lock`. - **Problem:** The Docker build omits this flag, so a manifest/lockfile mismatch can resolve new dependencies inside the image instead of failing. That can ship a dependency graph different from the checked-in lockfile inspected by the security tools; a consistent lockfile is not automatically upgraded on every build.
   - **Fix:** Add `--cargo-locked` to the Docker `cargo leptos build` command and verify that a deliberately stale lockfile stops the build.
   - **If we skip it:** Local validation and production packaging can disagree about which dependency versions are allowed.

4. **RESOLVED by temporary risk acceptance: unmaintained `paste`** (`Cargo.lock`, `deny.toml`, `README.md`)

   - **What this is:** `paste 1.0.15` is a transitive compile-time macro dependency.
   - **Problem:** [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) reports missing maintenance, with no patched release or known vulnerability. Five stable Leptos crates still require this version; a compatible stable update does not remove it.
   - **Fix:** The fallback maintenance decision is now documented: accept only this advisory temporarily in cargo-deny for trusted-source, compile-time macros, review by 2026-12-17 and on dependency/advisory changes, and remove the exception when stable upstream replaces Paste. Unknown sources and other advisories remain denied.
   - **Limit:** Paste has not been removed or made maintained; cargo-audit deliberately retains its warning. This is an explicit maintenance-risk acceptance, not a security patch or an exception for a future vulnerability.

5. **The largest accepted uploads have no successful memory-budget test** (`src/content/gallery.rs:86`, `src/content/gallery.rs:298`, `src/content/gallery.rs:442`, `README.md:397`)

   - **What this is:** Uploads allow up to 12 million pixels, with a 96 MiB decoder allocation limit and one processing job at a time.
   - **Problem:** Successful image fixtures are tiny, while the large-image tests exercise rejection; they do not establish that an accepted 4000×3000 image fits the intended Machine’s memory. Rotation, pixel conversion, and encoding allocate beyond the decoder limit, and the README already records production memory measurement as unfinished.
   - **Fix:** Add successful boundary-size fixtures, including orientation and transparency, and measure peak resident memory on the intended Machine. Use the measurement to confirm or lower the pixel limit, or increase Machine memory.
   - **If we skip it:** A valid administrator upload could exhaust memory and interrupt public browsing on the single Machine; this audit has not demonstrated that failure.

Validation performed: source review traced startup/storage checks, Google login and session guards, public content reads, editor writes, import/backup, mail delivery, and deployment commands. An isolated in-memory SQLite check used the installed session store’s exact save statement: save a session, cache its data, delete the row, then save the cached data; the deleted row returned.

The original audit's `cargo audit --json` exited with status 1: one vulnerability (`libsqlite3-sys`) and one unmaintained warning (`paste`), using advisory data updated on 2026-11-15. The existing scoped RSA exception was reviewed as an existing decision and was not counted as a new finding; `cargo-deny` was not rerun during that original audit.

Verdict: Logout and SQLite are fixed; Paste has a documented temporary maintenance decision. Resolve the new Chrono blocker described below and the remaining logging, error-response, build, and upload-validation gaps before release. The assumed single-Machine setup does not require replication for these fixes.

Not checked by the original audit: application builds/tests/coverage, browser tests, Docker build, and production memory. Item 1 now has a concurrent HTTP regression; its follow-up validation is recorded below. Live Google/SMTP behavior, deployed Fly settings, backup restoration on Fly, JavaScript dependency advisories, and a full visual/accessibility review remain untested by this audit.

Follow-up validation for item 1: the new HTTP regression failed before the fix because the old cookie regained access, then passed after durable generation revocation was added. Targeted logout tests passed, as did server Clippy and formatting checks. `just coverage --jobs 2` passed 81 native tests, with the browser workflow test ignored, and reported 80.83% line coverage (2403/2973). The changed SQL revocation path and logout ordering were exercised, including generation exhaustion; the defensive rejection when an administrator disappears between lookup and update was not triggered. A session-store flush failure was not injected. Browser/real-Google checks remain pending; tests used local fixtures and mock providers, not real credentials. Other findings were not re-audited or resolved by this fix.


New dependency follow-up (open): the refreshed advisory data reports `chrono 0.4.45` under `RUSTSEC-2026-0543` as unmaintained, with no fixed release. Chrono handles application dates and UTC timestamps; cargo-deny's maintenance policy now fails on it. Assess replacement with a maintained date/time crate across parsing, display, expiry checks, and tests; no Chrono exception was added. Until resolved, the security release gate remains unsuccessful despite the SQLite fix and Paste maintenance decision.

Follow-up validation for SQLite and Paste (2026-11-17): `just check` passed both Clippy targets, 81 native tests (browser/doc tests ignored), and all 10 Python script tests plus Fish syntax checks. Native coverage passed at 80.83% (2403/2973), including the linked-engine minimum-version regression; all eight real-process shutdown/storage/migration/import/backup tests passed. Fresh cargo-audit reports no unsuppressed vulnerabilities and retains Paste/Chrono maintenance warnings; cargo-deny accepts the scoped Paste decision and exact revision-pinned SQLx source, passes bans/sources, and fails only on unexcepted Chrono. The unchanged RSA exception still applies only to `openidconnect` public-key verification. Container builds and live Fly/Google checks were not run in this follow-up; no deployment was performed.
