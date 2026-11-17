What this repo does: This is Maj Kavšek’s portfolio, built with Leptos and Axum. Google sign-in protects an editor for gallery photos, shows, and YouTube videos; SQLite and private image files hold the published content. This audit assumes one region, one Machine, one volume, one or two administrators, and modest public traffic that includes bots.

Audit date: 2026-11-16. Reviewed commit `d58ea93` and the then-staged `fly.toml` changes. Later fixes are marked resolved below without renumbering the original findings.

**Must fix**

1. **RESOLVED: An in-flight request can restore a session after logout** (`src/auth/login.rs`, `src/database.rs`, `src/auth.rs`)

   - **What this is:** Each HTTP request loads its own session record, and the session middleware saves changed records after the handler finishes.
   - **Problem:** Before the fix, a slow authenticated request could finish after logout and recreate the deleted session under its old ID. A deterministic HTTP regression reproduced restored administrator access before the fix.
   - **Fix:** Logout now advances only that administrator's durable `session_generation` after authorization/Origin/CSRF checks and before flushing the session. All existing sessions for that account are invalidated without disabling approval or affecting other administrators; late session records can still be stored but cannot authorize access.
   - **Verification:** The HTTP regression pauses a request across logout and covers both a late session upsert and an editor-style delayed mutation. It checks old/same-account cookies, unaffected other administrators, fresh login, and cookie clearing; a separate test checks that generation exhaustion returns an error rather than successful logout.

2. **The bundled SQLite dependency still fails the security audit** (`Cargo.lock:2495`, `Cargo.toml:57`)

   - **What this is:** SQLx’s `sqlite` feature bundles the database engine through `libsqlite3-sys 0.37.0`.
   - **Problem:** Fresh `cargo audit` reports [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html): opening a crafted database and companion write-ahead log can expose process memory. This requires less-trusted local files; private storage reduces exposure, and no public HTTP path for supplying such a pair was identified.
   - **Fix:** Use a supported dependency update that permits patched `libsqlite3-sys >=0.39.0`, or explicitly link and verify a patched system SQLite. SQLx 0.9 currently requires `<0.38.0`, so a simple lockfile bump cannot resolve this; keep the advisory unsuppressed and rerun `just security` after the dependency change.
   - **If we skip it:** The release audit remains unsuccessful, and opening an attacker-supplied database/log pair remains unsafe.

**Should fix**

3. **Missing-page logging includes private query parameters** (`src/fallback.rs:30`)

   - **What this is:** The fallback handler logs the method and complete URI when a static file is missing.
   - **Problem:** A request such as `/missing?private=canary` writes the query value into production logs; a misspelled OAuth callback URL can similarly carry a temporary login code. The normal request tracing uses only the path, but this fallback bypasses that protection.
   - **Fix:** Log `parts.uri.path()` instead of the complete URI, and add a log-capture test using a harmless query marker.
   - **If we skip it:** Unmatched requests can copy private URL values into logs and retain them beyond the request.

4. **The contact API returns raw SMTP failures** (`src/routes/home.rs:478`)

   - **What this is:** The public contact endpoint sends mail through SMTP, the protocol used to deliver email.
   - **Problem:** `ServerFnError::from` forwards the mail provider’s error text to the caller, even though the page displays a generic failure message. A provider authentication or recipient rejection therefore exposes backend diagnostics to anyone calling the endpoint directly; this audit did not demonstrate password disclosure.
   - **Fix:** Return a fixed public delivery error for provider failures while keeping the existing safe delivery-status logs. Add a simulated transport failure and assert that its diagnostic marker is absent from the response.
   - **If we skip it:** The API’s error details remain dependent on what an external mail server returns.

5. **The production build can change the dependency lockfile** (`Dockerfile:27`, `justfile:32`)

   - **What this is:** Local build recipes use `--cargo-locked` to enforce the exact dependency versions recorded in `Cargo.lock`.
   - **Problem:** The Docker build omits this flag, so a manifest/lockfile mismatch can resolve new dependencies inside the image instead of failing. That can ship a dependency graph different from the checked-in lockfile inspected by the security tools; a consistent lockfile is not automatically upgraded on every build.
   - **Fix:** Add `--cargo-locked` to the Docker `cargo leptos build` command and verify that a deliberately stale lockfile stops the build.
   - **If we skip it:** Local validation and production packaging can disagree about which dependency versions are allowed.

6. **The unmaintained `paste` dependency remains unresolved** (`Cargo.lock:3002`, `deny.toml:7`)

   - **What this is:** `paste 1.0.15` is a transitive macro dependency used during compilation.
   - **Problem:** Fresh `cargo audit` still reports [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html), and the repository’s deny policy includes unmaintained dependencies. There is no patched `paste` release; this is a maintenance finding, not a demonstrated runtime vulnerability.
   - **Fix:** Move the owning upstream dependency to a compatible release that removes or replaces `paste` when available. If that path is unavailable, record a specific, justified maintenance-risk decision with a review date under the existing dependency policy rather than broadly disabling checks.
   - **If we skip it:** The maintenance warning remains unresolved, and the security checks cannot be reported as fully clean.

7. **The largest accepted uploads have no successful memory-budget test** (`src/content/gallery.rs:86`, `src/content/gallery.rs:298`, `src/content/gallery.rs:442`, `README.md:397`)

   - **What this is:** Uploads allow up to 12 million pixels, with a 96 MiB decoder allocation limit and one processing job at a time.
   - **Problem:** Successful image fixtures are tiny, while the large-image tests exercise rejection; they do not establish that an accepted 4000×3000 image fits the intended Machine’s memory. Rotation, pixel conversion, and encoding allocate beyond the decoder limit, and the README already records production memory measurement as unfinished.
   - **Fix:** Add successful boundary-size fixtures, including orientation and transparency, and measure peak resident memory on the intended Machine. Use the measurement to confirm or lower the pixel limit, or increase Machine memory.
   - **If we skip it:** A valid administrator upload could exhaust memory and interrupt public browsing on the single Machine; this audit has not demonstrated that failure.

Validation performed: source review traced startup/storage checks, Google login and session guards, public content reads, editor writes, import/backup, mail delivery, and deployment commands. An isolated in-memory SQLite check used the installed session store’s exact save statement: save a session, cache its data, delete the row, then save the cached data; the deleted row returned.

Fresh `cargo audit --json` exited with status 1: one vulnerability (`libsqlite3-sys`) and one unmaintained warning (`paste`), using advisory data updated on 2026-11-15. The existing scoped RSA exception was reviewed as an existing decision and was not counted as a new finding; `cargo-deny` was not rerun during this audit.

Verdict: Item 1 is resolved. Resolve the SQLite release blocker next, then close the logging, error-response, build, and upload-validation gaps. The assumed single-Machine setup does not require database replication to address these findings.

Not checked by the original audit: application builds/tests/coverage, browser tests, Docker build, and production memory. Item 1 now has a concurrent HTTP regression; its follow-up validation is recorded below. Live Google/SMTP behavior, deployed Fly settings, backup restoration on Fly, JavaScript dependency advisories, and a full visual/accessibility review remain untested by this audit.

Follow-up validation for item 1: the new HTTP regression failed before the fix because the old cookie regained access, then passed after durable generation revocation was added. Targeted logout tests passed, as did server Clippy and formatting checks. `just coverage --jobs 2` passed 81 native tests, with the browser workflow test ignored, and reported 80.83% line coverage (2403/2973). The changed SQL revocation path and logout ordering were exercised, including generation exhaustion; the defensive rejection when an administrator disappears between lookup and update was not triggered. A session-store flush failure was not injected. Browser/real-Google checks remain pending; tests used local fixtures and mock providers, not real credentials. Other findings were not re-audited or resolved by this fix.
