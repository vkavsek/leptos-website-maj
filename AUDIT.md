What this repo does: Maj Kavšek's portfolio serves public pages, music, photos,
shows and videos. Google sign-in plus explicit local approval protects content
editing. This review assumes modest public traffic, bots, one or two equally
privileged administrators, and one Fly Machine with one private volume.

Audit date: **2026-11-17**. Reviewed commit
`45c779bc9b58c8d1bc56383ac7cc7a0c6a8c8fba` **plus the staged Jiff migration and
calendar-date fixes**. This is a standard-profile source review using
`ponytail-audit` and `security-audit`; it does not establish deployed security.
Application source was not changed during the audit; subsequent fixes are recorded below.

**Result: no security vulnerability was confirmed in this pass.** One conditional
recovery lead survived two independent reviews and still needs validation. The
dependency-policy blocker and reliability observations below are separate from
confirmed vulnerabilities; none is assigned an exploit severity.

## Must fix before release

1. **RESOLVED by a scoped maintenance decision: Chrono remains in the dependency graph after the Jiff migration**
   (`Cargo.lock`, `Cargo.toml:70`, `deny.toml:5`, `README.md:483`)

   - **What this is:** Application dates now use Jiff, but OAuth2/OpenID Connect
     still pull in `chrono 0.4.45`.
   - **Problem:** The cached RustSec database reports the unmaintained advisory
     `RUSTSEC-2026-0543`. The repository's policy rejects unmaintained dependencies
     and has no Chrono exception, so replacing direct application uses does not
     clear that release requirement. This is a maintenance-policy blocker, not
     evidence of an exploitable date-handling flaw.
   - **Fix:** Resolve the remaining transitive dependency with compatible maintained
     upstream versions when available, then run fresh `just security` checks. If
     no compatible path exists, make an explicit, narrowly scoped maintenance-risk
     decision with rationale and a review deadline; do not silently suppress it.
   - **If we skip it:** The release still contradicts its dependency security
     policy. This audit's cached scan is not a substitute for a fresh release gate.

## Should fix — reliability and publication hygiene

2. **RESOLVED: The ZIP helper accepts files larger than the application can import**
   (`scripts/update_json.sh:64`, `src/content.rs:21`, `README.md:442`)

   - **What this is:** `just import-content` validates ZIP entries and replaces
     bundled show/video JSON before a later native database import.
   - **Problem:** `archive.read(entry)` has no decompressed-size limit, while the
     native importer rejects JSON above 2 MiB. For example, valid JSON consisting
     of 2 MiB of whitespace followed by `[]` passes the helper's validation but
     exceeds the application's limit. Much larger compressed inputs can also
     consume the operator process's memory. This path is operator-only; shared
     service exhaustion or unauthorized access was not demonstrated.
   - **Fix:** Reject oversized ZIP entries before reading, bound the actual
     decompressed read, and align record/field validation with native import.
     Add a regression that rejects over-limit input without replacing either source file.
   - **If we skip it:** A successful helper result can prepare an unusable import
     source. Sequential replacement of the two files also remains vulnerable to
     a mixed source version if the second filesystem operation fails.

3. **RESOLVED: Seed show/video JSON remains publicly packaged after catalog removal**
   (`Cargo.toml`, `src/fallback.rs:39`, `public/data_json`, `README.md:439`)

   - **What this is:** Leptos copies `public/` into the served site, including the
     one-time import JSON. The fallback blocks bundled gallery aliases, but not
     `/data_json/shows.json` or `/data_json/yt_videos.json`.
   - **Problem:** Removing an imported record from SQLite changes the live page
     and API but leaves its old seed record in these static assets. Those records
     were already public; no disclosure of private data or violated confidentiality
     guarantee was established.
   - **Fix:** Keep import sources outside the served asset tree and provide them
     separately during initial import, or explicitly document that catalog removal
     does not withdraw source-published JSON. Check direct seed URLs in a packaging regression.
   - **If we skip it:** Public copies can disagree with the live catalog, and
     removal through the editor does not remove every source-published copy.

## Needs validation — not a confirmed vulnerability

4. **Confirm which administrator approvals are authoritative after recovery**
   (`src/content.rs:202`, `src/database.rs:162`, `src/database.rs:211`,
   `MAINTENANCE.md:315`, `MAINTENANCE.md:348`)

   - **What this is:** A database backup includes historical enabled approvals.
     `revoke-all-sessions` invalidates sessions and login transactions while
     deliberately preserving those approvals.
   - **Possible problem:** Restoring a snapshot from before an identity's revocation
     can restore its enabled row. That account would then meet the fresh-login
     approval check unless the operator disables it again. Google identity
     verification is still required; this is not a login-protocol bypass.
   - **Existing control:** The maintained recovery instructions explicitly preserve
     approvals and require `list-users` before reopening HTTP. They also document
     `revoke-google`. The audit therefore does **not** claim that approval review
     is missing or that the owner unintentionally reauthorizes old accounts.
   - **What remains unknown:** Whether recovery intentionally accepts the restored
     roster or must preserve later revocations, and whether the owner reconciles
     that roster before exposure. No dummy recovery/fresh-login sequence was run.
   - **Safe next step:** Review the runbook against the authoritative current
     administrator roster without performing a live restore. In a fully isolated
     fixture, grant dummy A, snapshot, revoke A, restore the snapshot, apply the
     documented session-revocation and user-list steps, and test fresh establishment
     for an already verified dummy identity without Google calls. Finally revoke A
     in the restored database and verify rejection. This establishes mechanics;
     the owner must still define which recovered approvals are authorized.

This lead has **no severity**. Reconciliation or deliberate reauthorization would
refute the unauthorized-reactivation claim. Its stable identifier in the supporting
records is `auth/restore-preserves-obsolete-enabled-approvals`.

## Other hardening observations

Docker uses mutable base tags and unversioned downloaded build tools; pinning them
would improve release reproducibility. The runtime has no dedicated `USER`, so a
separate compromise would retain the container's current root authority. Broader
dotenv ignore patterns would reduce accidental credential inclusion. No substituted
build, privilege escalation, or real leaked credential was established, so these
are hardening notes rather than vulnerability findings.

The process-wide contact allowance and its reset on restart are documented design
limits. This audit does not relabel those accepted single-Machine tradeoffs as
new security defects.

## Evidence and coverage

- Four reconnaissance reviews mapped the source and trust boundaries. Three
  focused hunters inspected 14 coverage units. Separate post-wave and final
  coverage critics found no additional source-review assignments. One candidate
  received independent candidate validation and a distinct final-record review.
  Total: 11 agent assignments; no strict agent budget was set.
- Final ledger: **13 source-covered units and 1 candidate unit with an independently
  verified `needs_validation` record**; no deferred or out-of-scope units. Covered
  means source-inspected, not dynamically proven. There was no compatible prior
  structured ledger to carry forward, and this pass is not exhaustive assurance.
- Positive controls traced include browser-bound one-use OAuth state/PKCE/nonce,
  pinned provider URLs and RS256 claims, session rotation and durable revocation,
  exact Origin/CSRF checks, authorization after upload IO, bound SQL, escaped
  rendering, catalog-gated gallery reads, private storage separation, staged
  publication, coordinated backups, private no-store responses, and fixed SMTP
  errors with outcome-only logs. Relevant test source was read, not rerun.
- A trusted `cargo-audit` executable scanned the current lockfile without fetching
  dependencies or executing project code. It used an empty environment, a neutral
  working directory, explicit lockfile/database paths, `--no-fetch`, and
  `--no-yanked`. Cached advisory snapshot: `233155b4`, dated
  `2026-11-17T11:14:35+01:00`, containing 1,527 advisories; 546 dependencies scanned.
- That scan reported RSA `RUSTSEC-2023-0071` and the Chrono/Paste maintenance
  warnings. It intentionally did not load repository exceptions from its neutral
  working directory. The existing RSA exception is limited to public-key
  verification, matching the reviewed login path; the advisory concerns private-key
  operations. Paste has a dated compile-time maintenance-risk decision. Neither
  exception was widened. See the primary
  [RSA advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) and
  [Paste advisory](https://rustsec.org/advisories/RUSTSEC-2024-0436.html).
- **No target code was executed for this audit.** Individual namespace/resource
  probes did not establish a complete combined sandbox: Bubblewrap's all-namespace
  probe failed with `NETLINK_ROUTE ... Operation not permitted`, and the alternative
  combined containment/teardown path was not validated. The security-audit skill
  requires: “If every control cannot be enforced, do not execute target code.”
  Earlier development test/coverage results are not new audit reproductions.

Supporting records are outside the repository at
`/home/vkav/security-audit-skill/leptos-website-maj/run-1/`: architecture, coverage
ledger, structured findings, detailed reports and run metadata. Both structured
validators passed. Only this report was added to the repository.

**Verdict:** No exploit was confirmed. Issues 1–3 are resolved as recorded below;
Chrono remains a documented maintenance risk. Recovery approval semantics and
production acceptance still require validation.

**Not checked:** Runtime reproductions, new builds/tests/coverage, browser behavior,
Docker build/run, fresh cargo-deny/advisory/yank/npm security checks, full third-party
dependency implementation, production Google/SMTP, deployed Fly settings and memory,
backup access/retention and a real recovery drill. No live service, provider, registry,
or cloud control plane was probed.


## Resolution follow-up — 2026-11-17

1. Checked crates.io: the latest stable `oauth2 5.0.0` and `openidconnect 4.1.0`
   still require Chrono. Only maintenance advisory `RUSTSEC-2026-0543` is temporarily
   accepted in cargo-deny, with rationale and review due 2026-12-17 and on
   dependency/advisory changes. Future vulnerability advisories remain enforced;
   cargo-audit retains its warning. Fresh `just security` passes advisories, bans
   and sources. This resolves the policy decision, not Chrono's maintenance status.
2. The ZIP helper now caps compressed archive size, declared and actual expanded
   JSON at 2 MiB per entry, and record counts. It validates show text/calendar
   values and YouTube IDs/duplicates before either source file is replaced. Native
   seed imports reuse online show validation. Regressions cover over-limit failure
   preserving both files and successful input at exactly the byte boundary.
   Sequential filesystem replacement is still not a cross-file transaction.
3. Moved initial JSON and gallery pairs to `seed/`, outside Leptos public assets.
   Docker copies it separately to `/app/seed`; initial import commands, thumbnail
   tooling, fixtures and runbooks use the new root. The fallback also rejects old
   `/data_json/` URLs and encoded/traversal aliases even when stale build files exist.
   A source-layout regression verifies seed JSON is outside `public/`.

The supporting audit records outside the repository remain the original review
snapshot. These implementation checks are a separate follow-up, using local dummy
storage/providers without real Google or SMTP credentials. No deployment was made.

Follow-up validation: `just check` passed both Clippy targets, 85 native Rust tests,
12 Python script tests and Fish syntax checks. `just coverage --skip-clean --jobs 2`
passed at **81.08%** (2413/2976 native lines), including the new seed URL denial and
import field-validation paths. All eight real-process storage/import/backup/shutdown
checks passed using the relocated seed tree. Fresh `just security` passed; Chrono
and Paste warnings remain visible in cargo-audit. Docker image builds, browser
workflows and live Fly/Google/SMTP were not run. Issue 4 is unchanged.
