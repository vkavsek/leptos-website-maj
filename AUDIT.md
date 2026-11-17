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

1. **RESOLVED: Missing-page logging includes private query parameters** (`src/fallback.rs`)

   - **What this is:** The fallback handler logs the method and complete URI when a static file is missing.
   - **Problem:** A request such as `/missing?private=canary` writes the query value into production logs; a misspelled OAuth callback URL can similarly carry a temporary login code. The normal request tracing uses only the path, but this fallback bypasses that protection.
   - **Fix:** The fallback now logs structured method/path fields using `parts.uri.path()` and omits the query string.
   - **Verification:** A router-level log-capture regression requests `/missing?private=query_canary&code=oauth_canary`, confirms a 404 with the method/path logged, and verifies both query markers are absent.

2. **The contact API returns raw SMTP failures** (`src/routes/home.rs:478`)

   - **What this is:** The public contact endpoint sends mail through SMTP, the protocol used to deliver email.
   - **Problem:** `ServerFnError::from` forwards the mail provider’s error text to the caller, even though the page displays a generic failure message. A provider authentication or recipient rejection therefore exposes backend diagnostics to anyone calling the endpoint directly; this audit did not demonstrate password disclosure.
   - **Fix:** Return a fixed public delivery error for provider failures while keeping the existing safe delivery-status logs. Add a simulated transport failure and assert that its diagnostic marker is absent from the response.
   - **If we skip it:** The API’s error details remain dependent on what an external mail server returns.

3. **RESOLVED: The production build can change the dependency lockfile** (`Dockerfile`, `justfile`)

   - **What this is:** Local build recipes use `--cargo-locked` to enforce the exact dependency versions recorded in `Cargo.lock`.
   - **Problem:** The Docker build previously omitted this flag, allowing a manifest/lockfile mismatch to resolve new dependencies inside the image instead of failing. That could ship dependencies different from the reviewed lockfile.
   - **Fix:** Docker now uses `cargo leptos build --release --cargo-locked -vv`, matching the Just recipes.
   - **Verification:** Running that exact command against a temporary project with a deliberately stale lockfile failed at Cargo metadata without modifying the lockfile. A full Docker image build remains untested because daemon access is unavailable.

4. **RESOLVED by temporary risk acceptance: unmaintained `paste`** (`Cargo.lock`, `deny.toml`, `README.md`)

   - **What this is:** `paste 1.0.15` is a transitive compile-time macro dependency.
   - **Problem:** [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) reports missing maintenance, with no patched release or known vulnerability. Five stable Leptos crates still require this version; a compatible stable update does not remove it.
   - **Fix:** The fallback maintenance decision is now documented: accept only this advisory temporarily in cargo-deny for trusted-source, compile-time macros, review by 2026-12-17 and on dependency/advisory changes, and remove the exception when stable upstream replaces Paste. Unknown sources and other advisories remain denied.
   - **Limit:** Paste has not been removed or made maintained; cargo-audit deliberately retains its warning. This is an explicit maintenance-risk acceptance, not a security patch or an exception for a future vulnerability.

5. **RESOLVED for the requested local scope: largest accepted upload memory check** (`src/content/gallery.rs`, `justfile`, `README.md`)

   - **What this is:** Uploads allow up to 12 million pixels, with a 96 MiB decoder allocation limit and one processing job at a time.
   - **Problem:** The previous successful fixtures were tiny; large-image tests only exercised rejection. Rotation, conversion and encoding allocate beyond the decoder limit. The new local check found a rotated 16-bit PNG peaking at 199.3 MiB, exceeding its reserved upload allowance.
   - **Fix:** Normalize to the final 8-bit format before applying orientation, avoiding rotation of the larger 16-bit pixel buffer. Retain the 12-megapixel limit and single processing slot. `just test-upload-memory` measures five successful boundary fixtures in fresh processes against a 192 MiB allowance, leaving 64 MiB within the owner's requested 256 MiB budget.
   - **Verification:** JPEG with EXIF rotation: 87.0 MiB; 6000×2000 transparent PNG: 90.1 MiB; 16-bit RGBA PNG: 154.6 MiB; rotated 16-bit RGBA PNG: 154.7 MiB; transparent WebP: 89.3 MiB. All reach 12 megapixels and pass publication, orientation and thumbnail checks. A small nonuniform 16-bit regression verifies exact normalized pixels and alpha after rotation.
   - **Limit:** Local measurement only, as requested. Fixture generation is excluded; the compressed fixtures are below the 10 MiB byte limit. This measures the isolated upload pipeline, not total HTTP-server memory under traffic or a deployed Fly Machine. It is evidence for the current inputs/build, not a production OOM guarantee; monitor deployed memory and rerun on codec/limit/processing changes.

Validation performed: source review traced startup/storage checks, Google login and session guards, public content reads, editor writes, import/backup, mail delivery, and deployment commands. An isolated in-memory SQLite check used the installed session store’s exact save statement: save a session, cache its data, delete the row, then save the cached data; the deleted row returned.

The original audit's `cargo audit --json` exited with status 1: one vulnerability (`libsqlite3-sys`) and one unmaintained warning (`paste`), using advisory data updated on 2026-11-15. The existing scoped RSA exception was reviewed as an existing decision and was not counted as a new finding; `cargo-deny` was not rerun during that original audit.

Verdict: Logout, SQLite, private-query logging and build lock enforcement are fixed; Paste has a documented temporary maintenance decision. Local boundary-upload memory checks pass for a 256 MiB budget. Resolve the new Chrono blocker described below and the remaining SMTP error-response finding before release; deployed full-server memory is still unverified. The assumed single-Machine setup does not require replication for these fixes.

Not checked by the original audit: application builds/tests/coverage, browser tests, Docker build, and production memory. Item 1 now has a concurrent HTTP regression; its follow-up validation is recorded below. Live Google/SMTP behavior, deployed Fly settings, backup restoration on Fly, JavaScript dependency advisories, and a full visual/accessibility review remain untested by this audit.

Follow-up validation for item 1: the new HTTP regression failed before the fix because the old cookie regained access, then passed after durable generation revocation was added. Targeted logout tests passed, as did server Clippy and formatting checks. `just coverage --jobs 2` passed 81 native tests, with the browser workflow test ignored, and reported 80.83% line coverage (2403/2973). The changed SQL revocation path and logout ordering were exercised, including generation exhaustion; the defensive rejection when an administrator disappears between lookup and update was not triggered. A session-store flush failure was not injected. Browser/real-Google checks remain pending; tests used local fixtures and mock providers, not real credentials. Other findings were not re-audited or resolved by this fix.


New dependency follow-up (open): the refreshed advisory data reports `chrono 0.4.45` under `RUSTSEC-2026-0543` as unmaintained, with no fixed release. Chrono handles application dates and UTC timestamps; cargo-deny's maintenance policy now fails on it. Assess replacement with a maintained date/time crate across parsing, display, expiry checks, and tests; no Chrono exception was added. Until resolved, the security release gate remains unsuccessful despite the SQLite fix and Paste maintenance decision.

Follow-up validation for SQLite and Paste (2026-11-17): `just check` passed both Clippy targets, 81 native tests (browser/doc tests ignored), and all 10 Python script tests plus Fish syntax checks. Native coverage passed at 80.83% (2403/2973), including the linked-engine minimum-version regression; all eight real-process shutdown/storage/migration/import/backup tests passed. Fresh cargo-audit reports no unsuppressed vulnerabilities and retains Paste/Chrono maintenance warnings; cargo-deny accepts the scoped Paste decision and exact revision-pinned SQLx source, passes bans/sources, and fails only on unexcepted Chrono. The unchanged RSA exception still applies only to `openidconnect` public-key verification. Container builds and live Fly/Google checks were not run in this follow-up; no deployment was performed.

Follow-up for Should fix items 1, 3 and 5 (2026-11-17): `just check` passed both Clippy targets, 83 native tests, 10 Python script tests and Fish syntax checks. Native coverage passed at 80.82% (2402/2972); the query-safe fallback and changed image normalization/rotation path are covered. The browser workflow and two explicit memory tests are ignored by ordinary tests/coverage; `just test-upload-memory` separately passed all five measured child uploads. The stale-lockfile refusal was tested with the actual Docker build command locally. Added `#[inline]` only to the two short show-list accessors and the CLI storage-lock predicate; no speedup is claimed or benchmarked. No dependencies or Cargo features changed, and security checks were not rerun: the existing Chrono release blocker remains open. Browser tests, a complete Docker image build, live Google/SMTP, and Fly memory were not run in this follow-up.
