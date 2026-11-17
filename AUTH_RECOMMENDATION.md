# Google authentication: finish production setup

## Completed checklist

- [x] Separate development and production Google projects, consent configuration,
  and Web OAuth clients created; production branding and publishing completed.
- [x] Runtime configuration, private storage, SQLite migrations, sessions, guards,
  and shell-only administrator commands implemented.
- [x] Google OpenID Connect login and the Gallery, Shows, and YouTube editors
  implemented. Biography editing remains a source change and release.
- [x] Development credential launcher, local import, first local enrollment, and
  real Google sign-in tested. Local editing, expiry, revocation, restart, and
  account-isolation checks recorded as passed in the previous guide.
- [x] Automated build, Rust/script, authenticated browser, shutdown, coverage, and
  isolated upload-memory checks performed. Dependency fixes and temporary risk
  decisions are recorded in [README.md](README.md#coverage-and-dependency-checks).
- [x] Production Google secret staged, initial catalog imported, and the application
  deployed, as confirmed by the owner. The initial build/import/deployment steps
  are finished; do not repeat them to enroll an administrator.

**Current state: the website is deployed; no production administrators are approved.**
Successful deployment does not establish that production sign-in, editing,
persistence, full-server memory, or recovery have passed acceptance. Those are
what remains below. The development logout recheck also remains outstanding and
is included in the production checks. Google cancellation was not exercised
because Google did not offer that control; it is not recorded as passed.

Work through sections 1–6 once, in order. Leave checks unchecked until actually
performed. Keep a private record of the date, deployed commit/image digest,
Machine/volume IDs, browser/version, results, and backup location. Never record
client secrets, cookies, tokens, callback query strings, or CSRF values.

Workstation examples use **Fish**. Commands inside a Machine use **sh**.
Replace uppercase placeholders before running commands. Build/test instructions
are in [BUILD.md](BUILD.md); ongoing operations are in [MAINTENANCE.md](MAINTENANCE.md).

## 1. Select the existing production Machine

Use the already deployed application and its existing data. Do not create another
volume, re-import the catalog, or stage the same secret again.

1. Inspect the current deployment from the workstation:

   ```fish
   fly status --app maj-webpage
   fly machine list --app maj-webpage
   fly volumes list --app maj-webpage
   ```

2. Record the serving Machine ID and the unique volume ID, not just the volume
   name. Expect one serving Machine in `ams`, with `maj_data` mounted at `/data`.
   If the inventory differs, investigate before granting access or testing edits;
   independent databases on multiple Machines would split approvals and content.
3. Set the selected Machine for the following commands in this Fish session:

   ```fish
   set -l maj_machine_id REPLACE_WITH_MACHINE_ID
   fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
     '/app/leptos-website-maj admin list-users'
   ```

   Expect `[]` for the current state. If records exist, inspect them privately
   before changing approval. These commands use the deployed executable and can
   run alongside HTTP. Unlike backup/import, they do not require downtime.

The canonical origin is `https://majkavsek.com`; the registered production
callback is `https://majkavsek.com/auth/google/callback`. Keep this hostname
throughout sign-in. Development uses its separate client at `http://localhost:8080`.

## 2. Approve the first production administrator

1. Secure the intended Google account with MFA or a passkey and current recovery
   methods. Keep Fly operator access available independently of that account.
2. In a fresh private browser window, open `https://majkavsek.com/admin`, choose
   **Sign in with Google**, and select the intended production administrator's
   account. An existing Google session may skip account selection or consent;
   verify the selected account rather than assuming a consent screen must appear.
3. After Google sign-in, expect `/auth/identity` with **HTTP 403**. This is the
   expected unapproved-account result, not failed Google authentication. The page
   shows only this browser's verified issuer and subject for five minutes and
   grants no editor access. If it expires, start a new sign-in attempt.
4. Confirm ownership privately and copy the verified identity. The issuer must be
   `https://accounts.google.com`; use the displayed subject, never an email address
   or a guessed identifier. From the workstation, grant that identity:

   ```fish
   fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
     '/app/leptos-website-maj admin grant-google --issuer https://accounts.google.com --subject REPLACE_WITH_VERIFIED_GOOGLE_SUBJECT --label owner'
   fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
     '/app/leptos-website-maj admin list-users'
   ```

   Require exactly one enabled administrator with the intended identity. Keep
   subject/list output private. Google Console Test users and published branding
   do not grant local administration permission.
5. Return to `/admin` and **start a new Google sign-in**. Approval does not convert
   the earlier unapproved browser state into an administrator session. Verify that
   `/admin` now displays the owner label and the Gallery editor, and that Shows
   and YouTube videos can each be selected.

- [ ] Exactly one intended production identity is enabled and can sign in.

## 3. Verify production login and editing

Run these checks on the canonical HTTPS origin. Use Firefox first, then repeat
sign-in, an edit, and logout in another installed browser. Use separate browser
profiles for separate identities; private windows in one browser can share cookies.
Use uniquely named disposable content and preserve all existing records.

### Access and session checks

- [ ] **Anonymous browsing:** a signed-out profile can browse the public website;
  `/admin` and direct `/admin?section=shows` or `?section=videos` show no editors.
  `/auth/identity` without a recent verified identity returns 401.
- [ ] **Approved sign-in:** the owner reaches `/admin` through a fresh Google
  flow. The final URL contains no authorization code or token. The production
  session cookie is `__Host-maj_admin`, Secure, HttpOnly, SameSite=Lax, Path `/`,
  with no Domain attribute. Inspect attributes without copying the cookie value.
- [ ] **Private responses:** `/admin` is `no-store` with same-origin referrers;
  login/error/identity responses are `no-store` with no-referrer. No Google client
  secret or provider token appears in HTML, browser storage, or scripts.
- [ ] **Unapproved identity:** a separate Google account completes Google sign-in
  but receives only its own `/auth/identity` page with HTTP 403. It cannot access
  editors; after five minutes its identity display expires. Do not approve it.
  Google rejecting an account before callback does not test local approval.
  If no second account is available, record this check as not tested.
- [ ] **Cancellation, if offered:** decline a fresh Google attempt and confirm
  no editor session is granted. Closing the Google tab alone produces no callback;
  the website must remain signed out. Record an unavailable consent control as
  not exercised. Begin a new attempt after errors; do not replay real callback
  URLs or paste codes into terminals or tickets.
- [ ] **Logout regression:** sign in as the owner in two separate browsers. Sign
  out in one; reload protected pages in both. Both must require sign-in, including
  after Back/reload, while public content remains visible. A later fresh Google
  sign-in must work. App logout does not sign the browser out of Google.
- [ ] **Revocation:** with a disposable form open, disable the owner using the
  following workstation command. Its next protected request/submission must fail
  without saving the form. Keep operator access open during this check.

  ```fish
  fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
    '/app/leptos-website-maj admin revoke-google --issuer https://accounts.google.com --subject REPLACE_WITH_VERIFIED_GOOGLE_SUBJECT'
  ```

  Restore the same verified identity's approval, then check the roster:

  ```fish
  fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
    '/app/leptos-website-maj admin grant-google --issuer https://accounts.google.com --subject REPLACE_WITH_VERIFIED_GOOGLE_SUBJECT --label owner'
  fly ssh console --app maj-webpage --machine "$maj_machine_id" -C \
    '/app/leptos-website-maj admin list-users'
  ```

  An old browser session must not regain access when approval is restored; a
  fresh sign-in must work. Do not leave the only administrator disabled.
- [ ] **Idle expiry:** leave a disposable form idle for more than 30 minutes with
  no navigation/polling in that session, then submit. Expect denial and no saved
  change; sign in again deliberately. The eight-hour absolute limit has automated
  coverage; a real eight-hour wait can be recorded separately. Native forms do
  not preserve drafts reliably, and file inputs must be selected again.

### Content and public-page checks

- [ ] Navigate between Gallery, Shows, and YouTube videos. Exactly one editor is
  visible, the current link is marked, and browser Back/Forward works. Published
  lists include imported content and explicit removal controls.
- [ ] Add one disposable show with a valid date, one supported YouTube URL, and
  one small JPEG/PNG/WebP photo. Reload public pages in the signed-out browser and
  verify the show, video, thumbnail, and full-size image without deploying.
- [ ] Reject an invalid calendar date, unsupported video URL, and disguised
  non-image upload. Normal browser validation may prevent submission first.
  Do not disable authorization or CSRF/Origin protections to make a test pass.
- [ ] Remove only these disposable records using their confirmation controls.
  Reload public pages; removed original/thumbnail URLs must return 404. An old
  removal form must not remove a later record, and unrelated content must remain.
- [ ] From the home page, navigate to Shows, Past, and Future; reload Past directly.
  Check public photos, videos, CSS/WASM, audio, `/info`, `/privacy`, and
  `/health-check`. An empty Future list can be legitimate; browser panics and
  missing past shows are failures. Use a private window or hard reload after an
  asset update to avoid stale bundles.
- [ ] If SMTP delivery is required, send one deliberate contact message and confirm
  receipt. Otherwise record delivery as intentionally unavailable. Avoid repeated
  probes that consume the shared contact allowance.
- [ ] Observe deployed memory during a representative upload alongside ordinary
  browsing, and inspect logs for OOM/restarts. The local isolated upload test fits
  its 192 MiB allowance within a 256 MiB budget; it does not establish full-server
  production headroom. Record the actual Machine RAM and observed peak/limits.
- [ ] Review logs locally for unexpected failures and sensitive query/token values.
  Do not export HAR files, cookie panels, callback URLs, or credential prompts as
  evidence. Record only redacted errors and relevant statuses.

Record failures before proceeding; production acceptance remains incomplete until
required failures are resolved. For a redirect mismatch, compare the exact
production callback/client. For an application 403 after Google succeeds, check
local approval and start a fresh flow. For provider failures, inspect connectivity
and credential configuration without printing secrets or resetting the database.

## 4. Establish an off-machine backup and rehearse recovery

Do this after enrollment and before the restart/release tests. Retain the current
image digest and a private record of the intended enabled identity.

1. Announce a maintenance window and pause edits. Follow
   [the maintenance-window procedure](MAINTENANCE.md#fly-keep-a-console-available-while-http-is-stopped)
   on the existing volume-backed Machine. Backup requires HTTP and other database
   users to stop; a stopped Machine cannot provide the ordinary SSH console.
2. Create a new coordinated database/gallery backup using
   [Backups](MAINTENANCE.md#backups). Require a successful exit, `manifest.json`,
   and the final `COMPLETE` marker; a partial directory is not a backup.
3. Transfer the whole completed directory off the Machine and its volume. Run Fly
   as the normal workstation user and use a private user-owned destination such
   as `$HOME/.local/share/maj-backups`, outside the repository/build context.
   Remote root ownership does not require local `sudo fly` or weaker permissions.
4. Follow [backup verification](MAINTENANCE.md#verify-a-backup-before-using-it)
   on the downloaded copy. Record the backup time, source image/volume, destination,
   and verification result. A second copy on `/data` is not an off-machine backup.
5. Restore the normal HTTP entrypoint/command and check health/public content.
   Confirm that no temporary sleep command remains.
6. Follow [the isolated restore rehearsal](MAINTENANCE.md#rehearse-in-isolated-local-storage)
   in a new private local directory with the matching release binary/assets.
   Keep it loopback-only and remove production Google/SMTP credentials. Restore
   SQLite and its matching gallery together, then revoke all restored sessions
   before opening HTTP. Do not overwrite live production storage for this test.
7. Review restored approval records against the private intended-user record.
   Session revocation preserves approvals: disable any identity that should no
   longer be approved before exposing a restored system. Check content and image
   references, restart the isolated app, and record the recovery result.

- [ ] A verified off-machine backup exists and an isolated restore has succeeded.

## 5. Verify restart and release persistence

Keep the verified backup and previous image available. Plan downtime for the
single serving Machine; do not start a second independent catalog for this test.

1. Sign in and add a new uniquely named disposable show, video, and photo. Record
   their public appearance and the intended enabled identity.
2. Inspect the existing Machine's mount, permissions, and free space:

   ```fish
   fly ssh console --app maj-webpage --machine "$maj_machine_id"
   ```

   **Inside the Machine:**

   ```sh
   awk '$5 == "/data" { print }' /proc/self/mountinfo
   df -h /data
   stat -c '%a %u:%g %n' /data /data/gallery /data/staging /data/admin.sqlite3
   ```

   Expect a real `/data` mount, runtime-owned directories with mode `700`, and a
   database with mode `600`. Do not bypass startup checks or create an empty
   replacement database if any check fails.
3. Exit SSH, then restart the selected Machine from the workstation:

   ```fish
   fly machine restart "$maj_machine_id" --app maj-webpage
   fly status --app maj-webpage
   fly volumes list --app maj-webpage
   ```

   Require healthy startup, the same recorded volume ID, all three disposable
   records/images, and the same approval. Eligible sessions should survive;
   expected idle/absolute expiry is not data loss. Verify fresh sign-in too.
4. At the next planned fresh-image release, follow
   [the release checklist](MAINTENANCE.md#release-checklist) from start to finish,
   including its pre-upgrade backup and current security checks. If the browser
   clock fix has not yet been deployed, it can be that release. Use the same live
   volume and no catalog re-import. Verify the disposable content, gallery files,
   approval, and sign-in again afterward; record the resulting image and volume
   IDs. If the Machine ID changes, update `maj_machine_id` before further commands.
5. Remove the disposable records and confirm their removal publicly. Take another
   verified off-machine backup of the accepted clean state using the established
   maintenance procedure.

- [ ] Content, gallery files, and approval survive restart.
- [ ] Content, gallery files, and approval survive image replacement.

Until image replacement is tested, leave that item pending rather than treating
restart alone as proof of release persistence. An older image does not undo a
schema migration; recover a matching database/gallery using the maintenance guide
when rollback requires a data restore.

## 6. Close acceptance and establish routine operations

- [ ] Record the production login/edit/logout results and any checks not exercised.
  Confirm exactly one intended administrator is enabled after all tests.
- [ ] Record the accepted release, volume ID, verified backup, and successful
  restore rehearsal. Resolve required failures from the preceding steps.
- [ ] Assign backup responsibility, a daily maintenance window, protected
  off-machine storage, and retention. Start with seven daily/four weekly copies
  plus pre-upgrade backups if suitable. There is no built-in scheduler; verify
  scheduled jobs and transfers actually succeed. Keep several generations and
  define acceptable loss since the last backup.
- [ ] Establish disk/memory/health and backup checks through
  [Routine checks](MAINTENANCE.md#routine-checks). One Machine, one region, and one
  volume accept downtime during maintenance or host/regional failure; there is
  no database/gallery replication in this setup.
- [ ] Keep independent access to Fly and the Google project, account recovery
  methods, and dependency/advisory reviews current. Consult
  [the documented dependency decisions](README.md#coverage-and-dependency-checks)
  for review deadlines; an earlier successful audit is not a future release check.

The first administration setup is complete when the intended owner can manage
photos, shows, and videos, unauthorized requests remain denied, content survives
restart/release, and verified recovery works. Biography changes continue through
source edits and release.

Adding the second administrator is deferred. When needed, use
[Content, access, and credentials](MAINTENANCE.md#content-access-and-credentials):
verify that person's Google identity privately, grant it through the operator
command, and require a fresh sign-in. No new OAuth client or shared password is
needed; both approved users have the same content permissions.
