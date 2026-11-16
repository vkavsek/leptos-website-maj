# Security review

Date: 2026-11-16. Reviewed commit: `d58ea938894fb7d7c8484660214ac5b612e2b6d4`, including the staged `fly.toml` configuration.

**Result: dependency security checks fail. Three source-grounded risks need further validation; no application exploit was reproduced.** Address the dependency release blockers and validate the session-invalidation and upload-limit observations before an administration release.

This is a partial, source-first review. The review was finalized at the user's request before the planned coverage-critic and independent candidate-verification passes. The observations below are unvalidated candidates, not confirmed vulnerabilities, and have no assigned severity. Application tests and browsers were not run: the isolation probe failed with `bwrap: loopback: Failed to create NETLINK_ROUTE socket: Operation not permitted`. No deployed endpoints, Google accounts, SMTP services, or production data were tested.

## Dependency checks

Installed analysis tools inspected the lockfile and dependency graph without building or executing application code. Both checks used cached advisory data; the specific advisory descriptions were separately checked against the official RustSec website. These results do not constitute a freshly fetched, successful release gate.

| Check | Result |
| --- | --- |
| `cargo audit --no-fetch --json` | Exit 1: `libsqlite3-sys` vulnerability; `paste` maintenance warning. |
| `cargo deny --locked --offline --format json check advisories bans sources` | Exit 1: SQLite vulnerability and `paste` maintenance policy errors. Bans: 0 errors, 33 duplicate-version warnings. Sources: 0 errors. |

The cargo-audit database was at commit `02e2517f457a359fc6d3440ea4560c8ea188fee8`, dated 2026-11-15. Cargo-deny's initial advisory-cache lock failure was retried with approved cache access; the results above are from the completed retry.

### 1. Resolve the SQLite dependency release blocker

[Cargo.lock:2495](Cargo.lock#L2495) locks `libsqlite3-sys` 0.37.0 through SQLx 0.9. The selected SQLx `sqlite` feature enables bundled SQLite. [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html) affects this version; the patched crate range starts at 0.39.0. The advisory concerns disclosure of process memory when opening a crafted database and sibling WAL with inconsistent page sizes.

Application exposure is narrower than the package flag: [DatabaseManager::open](src/database.rs#L43) opens a fixed private database after [storage validation](src/storage.rs#L174), and the public editor does not accept database/WAL uploads. This review found no remote path that supplies those files. Restoring files from a less trusted source would require a separate provenance assessment.

Adopt a compatible SQLx/session-store dependency combination permitting the patched SQLite crate. The inspected `sqlx-sqlite` 0.9.0 manifest requires `libsqlite3-sys >=0.30.1, <0.38.0`, so updating only the lockfile entry cannot reach the fix. Re-run current `just security` after dependency changes. Do not suppress the advisory merely to pass the gate.

### 2. Resolve the unmaintained dependency policy failure

`paste` 1.0.15 remains in the Leptos dependency graph. [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) is a maintenance notice, not evidence of a runtime exploit. Nevertheless, the repository's `unmaintained = "all"` policy makes it a cargo-deny error. Prefer an upstream dependency update that removes it; any exception requires a specific rationale and review date under repository policy.

### Existing RSA exception

[deny.toml](deny.toml) and [.cargo/audit.toml](.cargo/audit.toml) narrowly ignore [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html), concerning private-key timing leakage in `rsa`. The inspected production OIDC path verifies signatures with public keys; no affected private-key operation was identified there. The exception is documented for review by 2026-12-16 and on usage/dependency changes. This assessment does not justify expanding it.

## Unvalidated source observations

The order below is a validation priority, not a severity rating. None has completed independent verification or a bounded runtime reproduction.

### A. A delayed session save may undo logout

**Boundary:** a previously valid administrator cookie should stop authorizing requests after successful logout.

The [logout handler](src/auth/login.rs#L403) checks approval, Origin and CSRF, then calls `session.flush()` at line 422. Another request can already have loaded the same session. [require_admin](src/auth.rs#L282) updates that request's cached `last_seen`, and the [session middleware](src/serve.rs#L162) can save modified state after the handler completes. The locally inspected locked `tower-sessions-sqlx-store` 0.16.0 implementation saves using `INSERT ... ON CONFLICT(id) DO UPDATE`, which can recreate a deleted row. Logout does not change the administrator generation checked by [require_admin](src/auth.rs#L270).

**Possible result:** a request finishing after logout could restore the old session ID, allowing its reuse. This requires possession of an existing valid cookie and an overlapping modified request. It does not explain how to obtain a cookie or bypass operator revocation; generation revocation and the eight-hour absolute expiry remain controls.

**Safe validation:** with dummy SQLite/session fixtures, deterministically pause request A after it loads and modifies the session; complete valid logout B; release A through normal middleware persistence; then use a fresh request with the original ID. Assert rejection. Repeat with operator generation revocation as a control. The existing sequential logout test at [login.rs:635](src/auth/login.rs#L635) does not establish this interleaving.

If reproduced, enforce invalidation across delayed saves, preserving the documented logout scope. A durable per-session invalidation mechanism or store semantics that refuse resurrection merits evaluation; changing every administrator session's generation would also change logout behavior.

Tracking fingerprint: `src/auth/login.rs:logout/session-delete-concurrent-upsert`.

### B. WebP internal allocations may bypass the upload limits

**Boundary:** an approved image upload should remain within the server's configured decoding limits.

[gallery::prepare](src/content/gallery.rs#L65) checks the input size, reported dimensions, 12-million-pixel limit and configured 96 MiB decoder allocation limit before decoding. Inspection of locked `image` 0.25.10 / `image-webp` 0.2.4 found a potentially inconsistent dimension path: extended WebP reports outer canvas dimensions, while the inner VP8 decoder allocates frame buffers from its embedded dimensions before comparing them with the canvas. In the inspected dependency source, see `image-webp/src/decoder.rs:699` and `src/vp8.rs:1190–1204`. The WebP adapter does not forward the application's allocation limit into those internal allocations.

**Possible result:** malformed WebP bytes with a small advertised canvas and larger inner frame could cause allocations beyond the intended bound before rejection. An approved administrator and valid Origin/CSRF are required. The [single processing permit](src/auth/editor.rs#L275) limits concurrent work and stays held through decoding. No accepted fixture, excess allocation, process termination, or production memory impact was observed.

**Safe validation:** use a small malformed fixture and bounded allocation instrumentation in an isolated decoder harness. Establish the allocation order and limit enforcement without exhausting memory. Deployment impact additionally requires the owner to inspect process memory limits. If reproduced, enforce limits before internal frame allocation through a suitable decoder fix or an effective processing boundary.

Tracking reference: upload-expansion coverage unit beginning `src/auth/editor.rs#POST /admin/content` → `src/content/gallery.rs#prepare`.

### C. Invalid callbacks consume other browsers' login budget

**Boundary:** an anonymous invalid callback should not consume all capacity for another browser's valid login completion.

The public [callback handler](src/auth/login.rs#L291) calls `reserve(true)` before accepting the query and consuming browser-bound state. [LoginService::reserve](src/auth/login.rs#L225) charges one shared process-wide deque, capped at 60 callbacks per rolling minute. Requests without a cookie or valid state can therefore occupy those slots without a provider exchange. A valid callback reaching a full deque would receive HTTP 429 before verification.

**Possible result:** denial of new login completions on the affected process while the window stays full. Existing authenticated access is outside this claim. The checked-in Fly settings limit concurrent connections, but effective deployed filtering and routing were not inspected.

**Safe validation:** in the existing local mock-provider fixture, prepare browser B's valid login, admit exactly 60 tiny invalid callbacks through the same service, then submit B's callback. Record its status and whether the mock exchange ran; verify recovery by advancing the stored test timestamps. Do not run this against a deployed service. Inspect ingress configuration separately. If reproduced, separate invalid-request throttling from capacity reserved for browser-bound completions while retaining bounds on pre-validation work.

Tracking fingerprint: `src/auth/login.rs:LoginService.reserve/unbound-callback-global-budget`.

## Additional hardening and recovery notes

- **Restore current approval policy.** [README.md:186](README.md#L186) requires revoking sessions after restoration. A historical database can also restore identities that were enabled at backup time. Reconcile `admin list-users` with current approvals before exposing the restored service; session revocation alone preserves enabled flags. This is an operator recovery consideration, not a demonstrated unauthorized restore path.
- **Review static seed copies after removal.** [README.md:350](README.md#L350) describes removal as unpublishing, but bundled `public/data_json/shows.json` and `yt_videos.json` are copied to the site root and are not blocked by the static fallback. Removing imported database rows does not remove those historical seed copies. Assess whether the unpublish promise includes them, and exclude or block them if so. Newly added database records are not exposed by this observation.
- **Reduce container/build authority.** The runtime Dockerfile has no `USER` directive; build inputs use mutable base tags and latest installers. Consider a dedicated runtime user with correctly provisioned data ownership and pinned, verified build tools. No container escape or compromised build input was demonstrated.
- **Bound operator ZIP expansion.** `scripts/update_json.sh` reads accepted ZIP members without an expanded-byte cap. A cap would protect the operator process; this is not a public upload endpoint or a demonstrated shared-service denial.
- **Keep public SMTP errors generic.** [home.rs:479](src/routes/home.rs#L479) converts provider errors into `ServerFnError` and returns the result. Consider mapping failures to a fixed public message. No credential disclosure was established; a generic browser display does not by itself prove the serialized response is generic.

## Controls inspected

Source review found exact issuer/subject approval, browser-bound single-use state, PKCE and nonce checks, pinned provider endpoints, RS256/claim verification, production cookie attributes, idle/absolute expiry, generation-based operator revocation, exact Origin and session CSRF checks, private no-store responses and restrictive admin CSP. SQL values use bindings. Uploads authenticate before buffering files, recheck approval after slow work, re-encode pixels and use opaque stored names. Gallery serving checks publication, and fallback blocks bundled gallery aliases. Storage validates private ownership/modes and separation from public roots; offline import/backup use exclusive locks and coordinated publication/snapshots.

These are source observations, not a runtime certification. A bounded current-tracked-text scan found only the documented test-only RSA private-key fixture among the checked key/token markers. No credential values are included here. This was not a full-history or entropy-based secret scan.

## Coverage and limits

The planned standard review had 18 units across authentication, content/rendering/uploads, storage/operator workflows, dependencies, deployment and developer tooling. The authentication and storage results were consolidated first; content/deployment work and final coverage disposition are retained in ignored `target/security-audit/run-1/`. No compatible prior structured coverage ledger was found; historical prose audits were not treated as evidence of current safety.

The planned coverage critics and independent candidate-verification passes were not completed before finalization. The two fingerprinted auth candidates and upload-expansion lead above remain unvalidated. No application exploit receives confirmed status or severity in this report. Existing tests were inspected, not executed; coverage measurement was not run. No application code was changed.

Excluded checks: live Google/SMTP and hosting configuration, deployed headers/routing/volumes, browser behavior, actual backup provenance and recovery, production upload memory, container/OS image advisories, npm advisory scanning, generated binaries/assets, and complete Git history. Existing staged `fly.toml` and concurrent `AUDIT.md` edits were preserved. Re-run fresh dependency checks and complete the bounded validations before relying on this review for release approval.
