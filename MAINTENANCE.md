# Application maintenance

This is the operating guide for the portfolio and its private administration
pages. Build dependencies and test setup are in [BUILD.md](BUILD.md), runtime
variables are in [README.md](README.md#runtime-configuration), and application
design is in [ARCHITECTURE.md](ARCHITECTURE.md). Use
[AUTH_RECOMMENDATION.md](AUTH_RECOMMENDATION.md) for the remaining first-administrator
enrollment and production acceptance checks after the initial deployment.

- [Routine checks](#routine-checks)
- [Operator commands and maintenance windows](#operator-commands-and-maintenance-windows)
- [Backups and verification](#backups)
- [Restore and recovery](#restore-and-recovery)
- [Migrations, deployments, and rollback](#migrations-and-deployment)
- [Content, access, and credentials](#content-access-and-credentials)
- [Troubleshooting and cleanup](#troubleshooting-and-cleanup)

Replace `MACHINE_ID`, `BACKUP_NAME`, and example absolute paths with the recorded
values and operator-owned locations. Run workstation commands from the repository
root; commands marked as running inside the Machine use its Linux shell. Examples
labelled `bash` require Bash, including when the interactive shell is Fish.

Commands run from the repository root unless marked **inside the Machine**.
Examples labelled `bash` use **Bash**; the credential prompt labelled `fish` uses
Fish. Replace uppercase placeholders before
running commands. Production commands below target `maj-webpage`; check
`fly.toml` and the actual deployment before each maintenance window.

## Operating model and data to preserve

The intended production setup is **one serving Machine, one region, one volume**.
The checked-in configuration selects `ams`, volume name `maj_data`, mount `/data`,
and canonical origin `https://majkavsek.com`. A volume name is not its unique ID:
record the actual Machine and `vol_...` IDs. There is no application replication;
maintenance and regional/host failures can interrupt public browsing.

| Location | What it contains | Maintenance rule |
| --- | --- | --- |
| `/app/leptos-website-maj`, `/app/site` | Release executable and built assets | Replace together with a release; keep the previous image reference |
| `/data/admin.sqlite3` | Approvals, browser sessions, login transactions, shows, videos, gallery catalog, migration/import history | Preserve with its matching gallery; never copy only the live database file |
| `/data/admin.sqlite3-wal`, `-shm`, possibly `-journal` | SQLite working files | Leave under SQLite's control; never remove them to recover space |
| `/data/gallery/originals`, `/data/gallery/thumbnails` | Uploaded/imported image pairs | Back up and restore with the database |
| `/data/staging` | Temporary upload/import work | Private; excluded from application backups |
| `/data/admin.lock` | Cross-process maintenance lock | Leave in place; deleting it does not unlock a running process safely |
| `/data/backups` (if used) | Temporary local backup copies | Transfer completed copies off the volume; this directory alone is not disaster recovery |

Directories must be owned by the runtime user with mode `0700`; database and
other private files use `0600`. The current Docker image runs as root; recheck
ownership if that changes. Startup rejects unsafe existing permissions instead
of repairing them. Inspect the named path before correcting it; do not recursively
change ownership or permissions across an existing volume.

Keep runtime data, credentials, and backups outside public asset trees, Git, and
the Docker build context. On Fly, `MAJ_DATA_DIR` must be exactly `/data` on a
separate mounted filesystem. A second serving Machine with an independent volume
would have a different catalog and approval list; do not scale out this design
without a replication plan.

## Routine checks

| When | Check |
| --- | --- |
| After a release, restart, or configuration change | Health, public pages, admin sign-in, Machine/volume pairing, and recent logs |
| After content-editing sessions and before migrations or substantial deletions | Take and transfer a coordinated backup |
| Daily when production content is changing | Check backup success/age, disk space, and application failures; use a scheduled maintenance window for offline backups |
| Periodically and after recovery changes | Restore a backup into isolated storage and record the result |
| Before releases and after dependency changes | Run current security checks and review unresolved audit findings |
| During continuous development | Stop builds/watchers and run `just clean` daily |

From the operator workstation:

```sh
fly status --app maj-webpage
fly machine list --app maj-webpage
fly volumes list --app maj-webpage
fly logs --app maj-webpage
curl --fail --show-error https://majkavsek.com/health-check
```

`fly logs` streams until interrupted. The health endpoint is a liveness check:
HTTP 200 alone does not verify database reads, Google login, SMTP, or recovery.
Also load `/about_me`, `/shows`, and `/media`, open a gallery photo, and use
`/admin` from the canonical origin when testing sign-in. Do not submit real mail
or mutate production content as part of unattended health checks.

Select the recorded Machine explicitly:

```sh
fly ssh console --app maj-webpage --machine MACHINE_ID
```

**Inside the Machine**, inspect capacity and private storage:

```sh
df -h /data
du -sh /data/gallery /data/staging
stat -c '%a %u:%g %n' /data /data/gallery /data/staging /data/admin.sqlite3
awk '$5 == "/data" { print }' /proc/self/mountinfo
```

Leave room for the database, WAL, image conversion/staging, and local backup
copies. Extend the intended volume before it fills; Fly volumes can grow but
cannot shrink. Follow [Fly volume management](https://fly.io/docs/volumes/volume-manage/)
and check capacity inside the Machine afterward.

## Operator commands and maintenance windows

`just admin --help` lists local commands. Local recipes default to
`$HOME/.local/share/maj-admin-dev`; explicitly set an absolute `MAJ_DATA_DIR` to
select other data. The deployed executable accepts the same `admin ...`
arguments without Cargo, Just, Google/SMTP credentials, or an HTTP origin.

**Use the same executable version as the serving application. Every operator
command except `backup` initializes storage and runs pending application/session
migrations before performing its action, including `list-users`.** Do not use a
newly built binary for a casual production inspection before its schema upgrade
has been planned.

HTTP and ordinary identity commands take shared storage locks. Backup and import
need an exclusive lock and fail promptly if another application process is using
the root. Migration work and restoration also belong in a maintenance window;
the shared lock on `admin migrate` does not make a live schema change safe.

### Local or self-managed server

Stop the watcher/server and prevent its supervisor from restarting it. Ctrl+C,
SIGINT, and SIGTERM allow active requests up to 25 seconds to drain; wait for the
process to exit. Run maintenance as the runtime user, then restore the normal
startup command and verify health. Fly's checked-in shutdown allowance is 30 seconds.

### Fly: keep a console available while HTTP is stopped

A stopped Machine cannot provide a normal SSH console. For the current Docker
image, temporarily run a sleeping process on the **existing attached Machine**.
First record its image digest, entrypoint/command, restart policy, services,
autostop settings, environment configuration, and volume ID from Fly's Machine
configuration. Keep any saved configuration private and outside the repository.
Verify that the normal entrypoint is `/app/leptos-website-maj` with no arguments;
custom configurations need their own matching restoration commands.

From the workstation, during the announced downtime:

```sh
fly machine update MACHINE_ID --app maj-webpage \
  --entrypoint /bin/sleep --command infinity --skip-health-checks
fly ssh console --app maj-webpage --machine MACHINE_ID
```

This preserves the image and volume and intentionally fails the HTTP checks.
Ensure any configured autostop will not shut down the maintenance console, and
restore its previous setting afterward. Verify the mount and ownership in the
console before touching data. Backup/import's exclusive lock provides a further
check that the application is stopped. Do not kill PID 1 in an ordinary serving
console, destroy the Machine, or attach a second helper to the live volume.

After maintenance, exit SSH and restore the current image's normal entrypoint:

```sh
fly machine update MACHINE_ID --app maj-webpage \
  --entrypoint /app/leptos-website-maj --command ''
```

The empty command removes `infinity`; leaving it would pass an invalid argument to
the server. Check the resulting configuration against the recorded settings, then
repeat the health and persistence checks. Rehearse this procedure before relying
on it in production. Flags were checked against local Fly CLI help and the
[upstream update implementation](https://github.com/superfly/flyctl/blob/master/internal/command/machine/run.go).

## Backups

The application backup is an **offline database-and-gallery snapshot**. It opens
the existing database without running pending migrations, uses SQLite `VACUUM
INTO` to create a standalone database, copies gallery files, checks catalog file
references, and writes a SHA-256 checksum manifest. Staging and the lock file are
not included; unreferenced gallery files may be included.

1. Stop HTTP using the procedure above, and stop other operator commands.
2. Provision a private backup parent outside public assets/build context. Choose
   a new destination name; the command refuses to overwrite an existing directory.
3. Run the backup and require a successful exit. **Inside the Fly Machine**:

   ```sh
   install -d -m 700 /data/backups
   MAJ_DATA_DIR=/data /app/leptos-website-maj admin backup \
     --output /data/backups/BACKUP_NAME
   ```

   Locally, after stopping the local server:

   ```sh
   install -d -m 700 "$HOME/.local/share/maj-backups"
   env MAJ_DATA_DIR=/absolute/path/to/maj-data just admin backup \
     --output "$HOME/.local/share/maj-backups/BACKUP_NAME"
   ```

4. Transfer the **whole completed directory** to protected storage off the Machine
   and its volume. For example, from a Bash workstation shell with a new local
   destination, while the maintenance Machine is still reachable:

   ```bash
   umask 077
   install -d -m 700 "$HOME/.local/share/maj-backups"
   fly ssh sftp get --app maj-webpage --machine MACHINE_ID --recursive \
     /data/backups/BACKUP_NAME "$HOME/.local/share/maj-backups/BACKUP_NAME"
   ```

   Run Fly as your normal workstation user. The local parent directory above
   belongs to that user; mode `0700` restricts access to its owner, which does not
   have to be root. Remote `/data/backups` ownership is independent: Fly's SSH/SFTP
   user defaults to root on the Machine. Do not use `sudo fly` or weaken the remote
   backup permissions to make a local destination writable. Keep the local backup
   outside the repository/build context. These commands also work in Fish.

   See [Fly recursive SFTP download](https://fly.io/docs/flyctl/ssh-sftp-get/).
   Verify the downloaded directory as described below, encrypt retained backups,
   and restrict access: they contain approved identities and browser sessions.
5. Restart HTTP and check health. Record the backup time, application commit/image,
   volume ID, transfer destination, verification result, and last restore test.

A valid backup contains `admin.sqlite3`, both `gallery/` trees, `manifest.json`
with format version 1, and `COMPLETE`. The completion marker is written last and
contains the manifest's hash. A directory without a valid marker is incomplete;
keep failed attempts separate and investigate before removing only their dedicated
destination. A successful local backup with a failed off-machine transfer is not
a successful disaster-recovery backup.

Scheduling, encryption, off-machine transfer, and retention are operator tasks;
there is no built-in backup scheduler. A reasonable starting retention is seven
daily copies and four weekly copies, plus pre-upgrade copies until the release is
proven. Adjust for storage and acceptable content loss. Offline backups briefly
interrupt this single-Machine site; account for that in any automation.

Fly snapshots supplement application backups. Check their actual retention and
completion; they can miss recent writes and do not prove a coordinated application
restore. See [Fly volume snapshots](https://fly.io/docs/volumes/snapshots/).
Never count a second directory on the same volume as an independent backup.

### Verify a backup before using it

Use a trusted backup source. Checksums detect corruption or incomplete transfer;
someone able to replace both files and checksums can forge them. The following
read-only check requires Python 3.11+ on the recovery workstation, not in the
production Docker image. Run it on an extracted/private backup directory and
stop if any check fails.

```bash
maj_backup="$HOME/.local/share/maj-backups/BACKUP_NAME"
python3 - "$maj_backup" <<'PY'
import hashlib
import json
import sqlite3
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve(strict=True)
actual = set()
for path in root.rglob("*"):
    if path.is_symlink() or not (path.is_file() or path.is_dir()):
        raise SystemExit("Backup contains a link or non-regular entry")
    if path.is_file():
        actual.add(path.relative_to(root).as_posix())

manifest_bytes = (root / "manifest.json").read_bytes()
if (root / "COMPLETE").read_text().strip() != hashlib.sha256(manifest_bytes).hexdigest():
    raise SystemExit("Invalid completion marker")
manifest = json.loads(manifest_bytes)
if manifest["version"] != 1 or "admin.sqlite3" not in manifest["files"]:
    raise SystemExit("Unsupported or incomplete manifest")
if actual != set(manifest["files"]) | {"manifest.json", "COMPLETE"}:
    raise SystemExit("Missing or unexpected backup files")
for name, expected in manifest["files"].items():
    with (root / name).open("rb") as file:
        if hashlib.file_digest(file, "sha256").hexdigest() != expected:
            raise SystemExit("File checksum mismatch")
for directory in ("gallery/originals", "gallery/thumbnails"):
    if not (root / directory).is_dir():
        raise SystemExit("Missing gallery directory")

with sqlite3.connect((root / "admin.sqlite3").as_uri() + "?mode=ro", uri=True) as db:
    if db.execute("PRAGMA integrity_check").fetchall() != [("ok",)]:
        raise SystemExit("SQLite integrity check failed")
    if db.execute("PRAGMA foreign_key_check").fetchall():
        raise SystemExit("SQLite foreign key check failed")
    if db.execute("SELECT 1 FROM sqlite_master WHERE name = 'gallery_images'").fetchone():
        for original, thumbnail in db.execute("SELECT original_file, thumbnail_file FROM gallery_images"):
            for directory, name in (("originals", original), ("thumbnails", thumbnail)):
                if f"gallery/{directory}/{name}" not in manifest["files"]:
                    raise SystemExit("Catalog references a missing image")
print("Backup checks passed")
PY
```

This verifies transfer consistency and database/file references; the isolated
restore and application checks below are still required.

## Restore and recovery

There is **no `admin restore` command**. Restore a verified database and its
matching gallery together, keeping the old data available for investigation.
Restoring loses all edits made after the chosen backup. Recover hosting access
and provisioned secrets separately; they are not in the application backup.

### Rehearse in isolated local storage

1. Verify the backup above. Retain its matching release executable/image and
   migration sources; do not start an arbitrary newer binary against it yet.
2. As the intended local runtime user, choose a new absolute directory outside
   the repository/public roots. These **Bash** commands refuse an existing target:

   ```bash
   umask 077
   maj_backup="$HOME/.local/share/maj-backups/BACKUP_NAME"
   maj_restore="$HOME/.local/share/maj-recovery/NEW_NAME"
   install -d -m 700 "$HOME/.local/share/maj-recovery"
   mkdir -m 700 -- "$maj_restore"
   install -m 600 -- "$maj_backup/admin.sqlite3" "$maj_restore/admin.sqlite3"
   cp -R -- "$maj_backup/gallery" "$maj_restore/gallery"
   ```

   Run each step only after the previous one succeeds. Do not copy old WAL/SHM,
   staging, or lock files into the new root. Confirm private ownership/modes.
3. Using the backup's matching release binary, invalidate restored sessions before
   opening HTTP; administrator approvals remain intact:

   ```bash
   MAJ_DATA_DIR="$maj_restore" /path/to/matching/leptos-website-maj admin revoke-all-sessions
   MAJ_DATA_DIR="$maj_restore" /path/to/matching/leptos-website-maj admin list-users
   ```

   This also clears pending logins. If testing an upgrade, take a copy of this
   recovered data first, then run the new version's migration against that copy.
4. Run the matching application/assets on loopback in explicit development mode,
   with production Google/SMTP credentials removed. Use the local run instructions
   in [README.md](README.md#runtime-configuration), selecting the recovered root;
   `just run` is suitable only when its debug binary matches the recovery version.
5. Check gallery images, shows/videos, approval records, and rejection of old
   sessions. Restart, verify the recovered content again, and record the result.
   Keep this rehearsal inaccessible from the public network.

### Restore production

Enter the offline maintenance configuration and verify the recorded volume before
making changes. Preserve a coordinated backup of the current state if possible;
if corruption prevents it, preserve the stopped database **and all companions**
with the gallery for investigation. Do not overwrite the only remaining copy.

Transfer and verify the chosen backup privately. With **all database users stopped**,
move the existing database, its companions, gallery, and staging into a uniquely
named private quarantine directory on the volume. Leave `/data/admin.lock` in
place. Install the recovered standalone database and matching gallery at their
normal paths under `/data`, with the runtime owner and private modes; the
application recreates staging. Ensure no previous database's WAL/SHM/journal is
left beside the recovered database.

Use the matching release image in maintenance mode and run **inside the Machine**:

```sh
MAJ_DATA_DIR=/data /app/leptos-website-maj admin revoke-all-sessions
MAJ_DATA_DIR=/data /app/leptos-website-maj admin list-users
```

Only then restore HTTP and verify public content, fresh administrator sign-in,
health, and persistence after a restart. Keep the original backup/quarantined
state until recovery is accepted. If using a replacement volume, provision its
owner/modes and deliberately attach that volume; confirm its unique ID afterward.
Do not change Fly's data root to a recovery subdirectory: its mount guard requires
exactly `/data`.

## Migrations and deployment

### Dependency and toolchain updates

Update small groups of dependencies and review both `Cargo.toml` and `Cargo.lock`.
For a specific release within the manifest's allowed range, use
`cargo update -p CRATE --precise VERSION`; major upgrades also need a reviewed
manifest change. Rebuild and test both server and browser targets using BUILD.md.
Run `just security` with current advisory data, and recheck any scoped advisory
exception when its dependency or usage changes. Keep build tools aligned with
the documented build requirements and test a toolchain update before deploying it.

### Create and apply a schema change

1. Create a migration with `just migration-add descriptive_name` (SQLx CLI setup
   is in BUILD.md). Review the SQL and its effect on existing rows and IDs.
2. Never edit or remove an applied migration. Add a new migration for corrections;
   checksum/version mismatches stop startup. Do not edit SQLx's history table to
   hide a mismatch.
3. Rebuild: migrations are embedded in the executable, and `build.rs` tracks their
   source directory. Test a fresh database and an upgrade of recovered existing
   data before releasing.
4. With the local HTTP server stopped, apply deliberately to the intended root:

   ```sh
   env MAJ_DATA_DIR=/absolute/path/to/maj-data just db-migrate
   ```

   `just db-migrate` uses the local debug build. On the deployed release, use
   `MAJ_DATA_DIR=/data /app/leptos-website-maj admin migrate` inside the offline
   maintenance console instead. Require success before restarting HTTP.

Normal startup runs application migrations and the session library's schema setup
before listening. Migration failure keeps HTTP closed. Do not migrate during a
Docker build or Fly `release_command`: the latter runs on a temporary Machine
without this application's persistent volume.
See [Fly release command behavior](https://fly.io/docs/reference/configuration/#the-deploy-section).

### Release checklist

1. Review the diff, migrations, configuration changes, and [AUDIT.md](AUDIT.md).
   Run `just check`, `just security`, `just test-shutdown`, and browser checks
   appropriate to the change; use `just coverage --jobs 2` for non-trivial native
   logic. A failed advisory fetch is not a clean audit. Resolve release blockers
   rather than suppressing them. SQLite now uses the pinned upstream fix and Paste
   has a reviewed temporary maintenance exception; transitive Chrono has a separate temporary maintenance exception (review by 2026-12-17). Recheck the Git pin and advisory review deadlines in README.md.
   After image-library, upload-limit or processing changes, also run
   `just test-upload-memory` on Linux with GNU time. It checks a 192 MiB isolated
   upload allowance within a 256 MiB budget; it does not measure the full server
   under production traffic. Check deployed memory when validating a new Machine size.
2. Record the current image digest/commit and volume ID. Check disk space,
   production origin and Google callback registration, required secrets, and
   Machine count. `fly secrets list --app maj-webpage` lists names/status without
   showing values; it does not prove a secret belongs to the correct client.
3. Take and verify an off-machine pre-upgrade backup. Keep content editing paused
   through migration/deployment. Preserve the previous image and compatible data.
4. Restore the recorded normal Machine startup configuration after backup, then
   deploy during the maintenance window; the new server migrates before HTTP:

   ```sh
   just deploy --ha=false
   fly status --app maj-webpage
   fly machine list --app maj-webpage
   fly volumes list --app maj-webpage
   ```

   [The deploy `--ha=false` option](https://fly.io/docs/flyctl/deploy/) prevents
   spare creation; it does not remove existing replicas. Require one intended
   serving Machine and the recorded volume. Do not use a second empty volume or
   a parallel old/new server to avoid the planned downtime.
5. Check health, migration/startup logs, public content, canonical-origin sign-in,
   and editors. Verify that saved records/photos and approvals survived; restart
   the Machine and verify again when validating a new persistence setup. Confirm
   no temporary sleep entrypoint remains. Record the release and backup results.

### Update for the browser clock fix

The Jiff browser-clock fix changes the hydration bundle. It requires a new image
and uses the existing database, approvals and volume; no content re-import or
volume provisioning is needed. Retain the previous image and verified backup.

From the updated repository, run in Fish:

```fish
just deploy --ha=false --strategy immediate --config fly.toml
fly status --app maj-webpage
fly machine list --app maj-webpage
fly volumes list --app maj-webpage
```

This builds a new image from the current source and performs the planned
single-Machine cutover, with a brief interruption. The earlier `maj_release_image`
variable identifies the previous image; this command builds the updated source.
Confirm the recorded existing volume remains attached. Then open a fresh private
browser window or force-reload so cached JavaScript/WASM is refreshed. Navigate
Home → Shows → Past → Future, reload Past directly, and verify the browser console
has no Jiff clock panic or `unreachable` error. An empty Future list can be correct
when all saved dates are past. Continue the normal production acceptance checklist.

### Rollback

An image rollback does **not** reverse SQLite migrations. An older binary may also
reject an applied migration absent from its embedded history, even if its queries
appear compatible. Test the old image against an isolated copy of the upgraded
database before choosing an image-only rollback. Use the prior image digest and
matching configuration, not an unrecorded floating tag.

If compatibility is unproven, keep HTTP stopped and either fix forward or restore
the pre-upgrade database/gallery together with the matching release. Session
revocation is required after data restoration. State explicitly which post-backup
edits will be lost; keep the failed state for investigation.

## Content, access, and credentials

Use `/admin` on the canonical origin to add/remove photos, shows, and YouTube
videos. Changes appear on subsequent public loads without a release. Deletion
is permanent from the editor; recover individual data through an isolated backup
restore and re-entry/re-upload if necessary. Biography, discography, audio, and
other static changes still require source edits and a release.

`admin import-content --source /app/seed` is an **initial, offline, one-time import**
into an empty catalog, not a recurring sync. It rejects initialized content.
Likewise, `just import-content ARCHIVE.zip` updates bundled source JSON; it does
not update the live database. Never re-import on every boot or release.

To manage approved users, obtain the verified issuer/subject from that person's
`/auth/identity` page after Google sign-in and confirm ownership privately.
Google Console Test users and email addresses do not grant application access.
In the selected production console, using the serving version:

```sh
/app/leptos-website-maj admin list-users
/app/leptos-website-maj admin grant-google \
  --issuer https://accounts.google.com --subject VERIFIED_SUBJECT --label owner
/app/leptos-website-maj admin revoke-google \
  --issuer https://accounts.google.com --subject VERIFIED_SUBJECT
/app/leptos-website-maj admin revoke-all-sessions
```

Use the action needed, not the entire block in sequence. Both administrators have
the same content permissions. Revocation disables the chosen identity and
invalidates its sessions; re-enabling requires a fresh sign-in. Global session
revocation preserves approvals. Website logout also revokes all existing sessions
for the signing-out administrator across browsers/devices, without disabling the
identity or affecting other administrators. Use operator revocation when an
identity must be disabled or sessions must be ended without access to a browser.

Store `GOOGLE_CLIENT_SECRET` and `PORKMAIL_PWD` in Fly secrets; origin, data path,
and Google client ID are ordinary configuration. Restart/redeploy after credential
changes because configuration is loaded once. Missing SMTP credentials disable
contact delivery while other pages remain available; production Google settings
are required for startup. Secret rotation alone does not invalidate local sessions.

Google availability affects new sign-ins; public browsing and existing eligible
local sessions do not require a live Google connection. Diagnose provider outages
without resetting the database or weakening local approval checks.

For a Google secret rotation, use this Fish prompt and stdin, with `fish_trace`
disabled. Enter the secret at the masked prompt to keep it out of command history:

```fish
read --silent --prompt-str 'Production Google client secret: ' maj_google_secret
printf '\n'
printf 'GOOGLE_CLIENT_SECRET=%s\n' "$maj_google_secret" | \
  fly secrets import --stage --app maj-webpage
set --erase maj_google_secret
```

[Staging the secret](https://fly.io/docs/flyctl/secrets-import/) avoids an immediate
deployment; install it with the planned release, then verify sign-in. Rotate SMTP
through the same protected mechanism using `PORKMAIL_PWD`. For a compromise,
revoke affected identities/sessions as well, secure the Google/provider accounts,
and review hosting access. Keep a separate recovery path to Fly and the Google
project if the only administrator loses their Google account.

## Troubleshooting and cleanup

| Symptom | First action |
| --- | --- |
| Startup refuses storage/mount | Confirm the actual `/data` mount, volume ID, runtime owner, and the exact path named in the error; preserve existing data |
| Migration failure/checksum mismatch | Keep the server stopped; compare the release's immutable migrations and backup history, then fix forward or restore a compatible pair |
| Backup/import reports storage busy | Stop HTTP, watchers, and other operator commands; do not remove `admin.lock` |
| Content disappears after deploy | Check Machine count and volume attachment before creating/importing anything; an empty new database is not recovery |
| Google login fails | Compare canonical origin/callback/client pairing, account approval, cookies, system time, and outbound HTTPS; use the real-login guide |
| Content mutation is forbidden | Refresh the form/sign-in and use the canonical origin; do not disable Origin/CSRF checks |
| Contact mail fails | Check provisioned SMTP credentials and provider availability; failures/timeouts consume the shared ten-attempt hourly allowance |
| Disk full or upload/write failure | Pause edits, inspect `df`/`du`, transfer backups, and expand storage; do not delete SQLite companion files |
| Repeated cleanup warnings | Investigate database access/capacity; expired authentication data is normally cleaned every five minutes |

Deleted images and interrupted uploads may leave private unreferenced files.
There is no orphan-cleanup command. Under an offline maintenance lock/window,
back up first and compare gallery files with catalog references before removing
only proven orphans. Inspect staging only when no upload/import can be writing.
Do not empty gallery or staging indiscriminately. Routine manual database
compaction or hand-editing session rows is not required.

For development disk use, stop watchers/builds and save needed coverage reports
outside `target/`, then run `just clean` daily. This removes compiled artifacts,
generated assets, and coverage output; rebuild before running again. It preserves
runtime data only when that data is kept outside build-output directories.

Keep a short private operations log: releases/images, schema changes, backup and
transfer results, restore drills, credential rotations, volume/Machine changes,
and incidents. This guide describes procedures; it does not establish that the
production backup schedule, restore drill, Google configuration, or Fly maintenance
workflow has already been exercised.
