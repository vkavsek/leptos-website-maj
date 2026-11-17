# Google authentication and administration setup

## Progress

- [x] Sections 1–3: deployment values, Google projects/consent, and OAuth clients
  completed, as confirmed by the owner.
- [x] Section 4.1: server configuration validation and the development Fish
  launcher implemented and tested.
- [x] Section 4.2: private layout and separation from public assets implemented.
- [x] Section 4.3: SQLite initialization, application migrations, and session schema implemented.
- [x] Section 5 foundation: schemas, session/context integration, guards, and all operator commands.
- [x] Section 6 implementation: Google sign-in and approved sessions, with local mock-provider tests.
- [x] Section 7 implementation: gallery, shows and video editors with live database loaders.
- [x] Section 8: local import, initial enrollment and approved sign-in completed,
  as confirmed by the owner.
- [ ] Section 9: automated regression/build checks and the extended manual Google
  checklist below; record actual results rather than assuming section 8 covers every case.
- [ ] Sections 10 onward: production persistence, upload memory
  measurement, deployment and recovery remain pending.
- [ ] Final pre-deployment step: install the production `GOOGLE_CLIENT_SECRET`
  in Fly secret storage. **It has not been added yet.** See steps 10–11.

This guide specifies the first administration release: Google OpenID Connect
(OIDC), one explicitly approved administrator, and editing for gallery photos,
shows, and YouTube videos. Another administrator can be approved later without
changing the authentication design. Both accounts will have the same permission.

**Biography editing is deferred.** Keep the biography in `src/routes/about.rs`;
updating it will continue to require a release. Do not add a biography editor,
database table, or mutation endpoint in this release.

**Implementation status:** this is a setup and implementation guide, not an
already installed authentication feature. Section 4.1's configuration handling
and development launcher exist, together with section 4.2's private directories
and section 4.3's initialized SQLite database. Section 5 now provides every listed
operator command, private schemas, session middleware, shared authorization/CSRF
guards, and Axum/Leptos server context. Sections 6–7 now provide Google sign-in and the three content workflows; public
loaders query live SQLite. Real-provider checks and production upload memory
measurement remain deployment prerequisites.
The `leptos-website-maj admin ...` commands work after building the current code;
production use still requires the planned deployment and release blockers to clear.
Google Console and Fly commands are existing tools, but this guide does not run
them or change the deployed application.

Provider documentation was checked on 2026-11-12. Console labels and dependency
versions can change; the linked official documentation is the reference.
Shell examples use **Bash** unless labeled **Fish**, on Linux, from the
repository root unless noted.
Build prerequisites remain in [BUILD.md](BUILD.md).

## 1. Choose the deployment values

**Status: complete, confirmed by the owner.**

Record these before creating credentials:

| Value               | Development                                  | Production                                       |
| ------------------- | -------------------------------------------- | ------------------------------------------------ |
| Application origin  | `http://localhost:8080`                      | `https://www.majkavsek.com`                      |
| Callback            | `http://localhost:8080/auth/google/callback` | `https://www.majkavsek.com/auth/google/callback` |
| Data directory      | `$HOME/.local/share/maj-admin-dev`           | `/data` on a Fly volume                          |
| Google OAuth client | Web client in the development project        | Web client in the production project             |
| Google client ID | `312770222166-qtkr1gn7e22jpofqtt31vhbq88bj3ppb.apps.googleusercontent.com` | `774712125801-lljgocauoic4un01lt1ml1hg27l10tep.apps.googleusercontent.com` |
| Google project ID | `maj-webpage-admin-development` | `maj-webpage-admin-production` |

Replace `YOUR_DOMAIN` with the canonical public hostname, including `www` if that
is the hostname you use. Use that same hostname throughout sign-in. `localhost`
and `127.0.0.1` are different browser hosts; use `localhost` for the development
sign-in flow described here.

The checked-in Fly app is `maj-webpage`, in region `ams`. Confirm these against
the deployment you intend to change; the repository does not establish the
current live machine count, volume attachments, or custom domain.

NOTE: Added fly status and changed the custom domain.

App
Name maj-webpage
Owner personal
Hostname maj-webpage.fly.dev
Image maj-webpage:deployment-01M4B2VKFNYD9A5T9FWY9SZBKK

Machines
PROCESS ID VERSION REGION STATE ROLE CHECKS LAST UPDATED
app 56830157ad0338 63 ams started 1 total, 1 passing 2026-11-09T06:54:57Z
app e7843d1ec41138 63 ams started 1 total, 1 passing 2026-11-09T09:07:26Z

Secure the initial administrator's Google account with MFA or a passkey, and keep
its recovery methods current. Website login and management access to the Google
Cloud project are separate permissions.

## 2. Create the Google project and consent configuration

**Status: complete, confirmed by the owner.** Project IDs are
`maj-webpage-admin-development` and `maj-webpage-admin-production`.

1. Open the [Google Cloud Console](https://console.cloud.google.com/) and create
   separate development and production projects, for example
   `maj-webpage-admin-development` and `maj-webpage-admin-production`. Keep the correct project
   selected when configuring each environment.
2. In each project, open **Google Auth Platform**, then **Branding**. Enter an
   application name, support contact, and project contact. Complete the requested
   domain and policy information using the site's real information.
3. Set **Audience** to **External** for a personal Google account. Internal is
   intended for accounts in the project's Google organization.
4. Configure the initial testing audience in **each project**:

   - Open **Google Auth Platform → Audience** and find **Publishing status**.
     Leave it set to **Testing** during setup. Do not click **Publish app** yet;
     production readiness is covered in step 10. A project named
     `maj-webpage-admin-production` can still have a publishing status of Testing.
   - Find **Test users**, click **Add users**, enter the Google account email of
     the person who will initially administer the website, and save. Use the
     account they will select at sign-in; it need not be the account that owns
     the Cloud project. Add only that initial account for now.
   - Repeat in the other project if the same person will test both environments.
     Test-user entries belong to individual projects, not to all your projects.
   - If the Console does not offer the Test users control, continue with the
     identity-only scopes below; this list is not required for that flow.

5. Configure the sign-in permissions, called **scopes**, in each project:

   - Open **Google Auth Platform → Data Access**, then **Add or remove scopes**.
     Select these two entries, using their identifiers to distinguish them:
     `openid` and `https://www.googleapis.com/auth/userinfo.email`.
   - If an entry is missing from the list, use **Manually add scopes** to enter
     the missing identifier, then **Add to table**. Do not enter `openid email`
     as one Console scope; it represents two separate permissions.
   - Click **Update**, then **Save** if the page offers it. Check that both
     entries appear under the non-sensitive scopes. Leave `userinfo.profile`
     unselected; no sensitive or restricted scopes are needed for this design.
   - Repeat for the other project; scope configuration is separate for each.

   `openid` enables OpenID Connect sign-in and an ID token containing Google's
   stable account identifier (`sub`). The email permission supplies the account's
   email address and `email_verified` claim. It does not allow reading or sending
   Gmail messages. Use email to help identify the account during enrollment;
   website access still depends on the locally approved issuer and subject,
   rather than an email match.

   In the server implementation, request exactly the space-separated scopes
   `openid email` when starting login. Google accepts `email` as the OpenID
   Connect equivalent of the longer Console identifier above. Declaring scopes
   in the Console does not configure the Rust client's authorization request;
   configure both, and validate the returned ID token as described in step 6.
   See [Google's scope configuration instructions](https://support.google.com/cloud/answer/15549135?hl=en)
   and [OpenID Connect scope definitions](https://developers.google.com/identity/openid-connect/openid-connect).

   Gallery uploads are stored by this website, and the video editor stores
   YouTube links. Neither needs access to Google Photos, Drive, or a YouTube
   account. Do not add those APIs or permissions for these editors.

Follow Google's [consent setup instructions](https://developers.google.com/workspace/guides/configure-oauth-consent)
and [audience documentation](https://support.google.com/cloud/answer/15549945?hl=en).
Normally, Testing limits authorization to listed test users and consent expires
after seven days. Google's exception for identity-only requests such as our
`openid email` scopes removes those restrictions and the Testing warning. Other
Google accounts can therefore complete Google sign-in even while Testing is
selected. **The Test users list is not the website's administrator access
control.** Adding an email there does not grant website access: explicitly approve
the verified Google identity locally in step 8, using the access check from step
6. Google sign-in alone must never grant administration rights.

Google's [OAuth policies](https://developers.google.com/identity/protocols/oauth2/policies)
call for separate projects for deployment tiers. Use one web client per project
here. The private, known-user audience can qualify for personal-use exceptions to
some production requirements; do not assume either that public deployment
automatically requires sensitive-scope verification or that all branding
requirements are waived.

## 3. Register development and production OAuth clients

**Status: complete, confirmed by the owner.** Keep each client ID and secret
associated with its project; the project ID itself is not an OAuth client ID.

An OAuth client identifies this website to Google. Create one for development
and one for production, each in its own project. These clients are shared by the
people signing in: adding a second administrator later does not require another
OAuth client.

Create the development client first:

1. Select `maj-webpage-admin-development` in the Console's project selector. Open
   **Google Auth Platform → Clients**, then click **Create client**. Complete
   section 2 first if Google asks you to configure the Auth Platform.
2. For **Application type**, choose **Web application**. Although the site uses
   WASM in the browser, the Rust server will exchange the authorization code.
3. For **Name**, enter `maj-admin-web-development`. This identifies the credential
   in the Console; it does not change the consent-screen application name or the
   Rust package name.
4. Leave **Authorized JavaScript origins** empty. This design uses browser
   redirects and a server callback, without Google's browser JavaScript SDK.
5. Under **Authorized redirect URIs**, click **Add URI** and enter exactly
   `http://localhost:8080/auth/google/callback`. This is the server endpoint that
   receives Google's response, including the temporary authorization code.
   The homepage or `/admin` URL is not the callback.
6. Click **Create**. Store the resulting client ID and client secret as described
   below, with a clear development label.

Switch to `maj-webpage-admin-production` and repeat those steps with these values:

| Field | Development | Production |
| ----- | ----------- | ---------- |
| Application type | Web application | Web application |
| Client name | `maj-admin-web-development` | `maj-admin-web-production` |
| Authorized JavaScript origins | Leave empty | Leave empty |
| Authorized redirect URI | `http://localhost:8080/auth/google/callback` | `https://www.majkavsek.com/auth/google/callback` |

Keep only the matching environment's callback on each client. The production
hostname follows section 1; change both sections together if the canonical domain
changes. Creating a client does not publish the application or approve an
administrator. See [Google's client creation instructions](https://developers.google.com/workspace/guides/create-credentials).

For each client, keep its credentials together in a password manager or another
protected secret store, labeled with the project and environment:

- **Client ID** identifies the application, usually ending in
  `.apps.googleusercontent.com`. Supply it as `GOOGLE_CLIENT_ID` in that
  environment. It is ordinary configuration, not the Google account email or
  project ID.
- **Client secret** authenticates the server during the code exchange. Supply
  it as the server-only `GOOGLE_CLIENT_SECRET`. Google documents that newly
  created secrets are shown only at creation, so save it before dismissing the
  dialog. If it is unavailable later, create a replacement using the client's
  secret-management controls. See [Google's credential management guidance](https://support.google.com/cloud/answer/15549257?hl=en).

Use the ID and secret from the same client. Development configuration is covered
in step 8 and production secret installation in step 10. Do not put a secret or
downloaded credentials JSON in the repository, public assets, browser code, or
issue reports. The manual environment configuration in this guide does not
require downloading a credentials file.

Check the callback character for character before continuing. HTTPS is required
for production; HTTP is allowed for localhost development. `localhost` differs
from `127.0.0.1`, and `www.majkavsek.com` differs from `majkavsek.com`. Keep the
port and path shown above, without an extra trailing slash. Once implemented,
the server derives this callback from `MAJ_APP_ORIGIN` plus
`/auth/google/callback`; the resulting URL must match the client's registered
URI or Google returns `redirect_uri_mismatch`. Google notes that URI changes can
take five minutes to several hours to propagate. See
[Google's redirect URI rules](https://support.google.com/cloud/answer/15549257?hl=en)
and [the server-side OpenID Connect flow](https://developers.google.com/identity/openid-connect/openid-connect).

At this stage, both clients should appear in their respective projects with
different client IDs. The website will only be able to complete sign-in after
the callback, configuration, and access checks in steps 4–6 are implemented;
registering a callback does not create a route in the Rust application.

## 4. Define application configuration and persistent storage

**Sections 4.1–4.3 are implemented. Production persistence setup remains in section 10.** The server validates
configuration and prepares private filesystem storage, but does not yet initialize
Google login. SQLite initialization and migrations now run before HTTP startup.

### 4.1. Supply the server configuration

**Status: implemented.** `src/server_config.rs` validates settings before the
HTTP listener is opened. `src/serve.rs` shares the configuration as a server-only
Axum extension; no credentials are serialized into Leptos resources.

Use these four environment variables; do not hard-code credentials in Rust:

| Setting | Development value | Production value | Purpose |
| ------- | ----------------- | ---------------- | ------- |
| `MAJ_APP_ORIGIN` | `http://localhost:8080` | `https://www.majkavsek.com` | Canonical browser-facing origin, without a trailing slash |
| `GOOGLE_CLIENT_ID` | Unset normally; development client's ID for login tests | Production client's ID | Identify the matching OAuth client from section 3 |
| `GOOGLE_CLIENT_SECRET` | Unset normally; enter development secret interactively for login tests | Production client's secret | Authenticate the server to Google during the code exchange |
| `MAJ_DATA_DIR` | `$HOME/.local/share/maj-admin-dev` after shell expansion | `/data` | Absolute path containing the database and uploaded gallery files |

Only `GOOGLE_CLIENT_SECRET` is confidential. For production, put the origin,
client ID, and data path under `[env]` in `fly.toml`, and install the secret
through Fly's secret store as described in step 10. A local shell variable does
not configure the deployed Fly Machine.

For regular local development, leave **both Google credential variables unset**.
Set `APP_ENVIRONMENT=development` and a loopback origin. Public pages and current
file-backed content reads work; no login or administration routes exist yet.
When those routes are implemented, absent credentials must keep them unavailable
and must never bypass authorization. The origin and absolute data path are
required now; the database is added in subsequent steps. `APP_ENVIRONMENT` accepts
only `development` or `production`; without it, debug builds use development and
release builds use production. Docker already sets `production`. Missing Google
credentials never change a production process into development mode.

Do not add an active `GOOGLE_CLIENT_SECRET` placeholder to `.cargo/config.toml`.
A dummy string cannot authenticate with Google, and should be rejected as invalid
configuration rather than silently switching modes. Keep the credential absent
until testing sign-in. Cargo's `[env]` supplies values to Cargo-launched processes
and is not a secret store; its defaults normally yield to existing environment
variables, but `force = true` would override an interactively supplied value.
See [Cargo's environment configuration](https://doc.rust-lang.org/cargo/reference/config.html#env).
No change to `.cargo/config.toml` is needed for this recommendation.

For an explicit Google login test, run the **Fish development launcher**:

```fish
just dev-auth
```

The launcher defaults to the client ID recorded above for
`maj-webpage-admin-development`. An existing `GOOGLE_CLIENT_ID` environment
variable can override that public default. Copy its client secret from your
password manager and paste it at the masked prompt.
**Do not paste a real secret into the script file.** The launcher
clears the current clipboard after reading input, then calls the justfile's shared
watch recipe with the secret in its environment. It also clears after empty input or
EOF, and refuses to launch if clearing fails or no clipboard tool is available.

Clipboard support requires `wl-copy` for Wayland (`wl-clipboard` package), `xclip`
for X11, or `powershell.exe` for Windows-hosted WSL. WSL uses the Windows clipboard
rather than assuming a Linux display clipboard matches the terminal. These are
selected automatically. Clearing the current clipboard does not erase saved
clipboard history or cloud-synced copies; disable that history for secrets or use
your password manager's clipboard protections.

The launcher requires `just`, Fish 3.7 or newer, and the development build
dependencies in BUILD.md. It uses `http://localhost:8080` and defaults to
`$HOME/.local/share/maj-admin-dev`, creating that directory privately. An existing
`MAJ_DATA_DIR` can select another absolute development directory. The secret is
not stored in command history, a file, or a persistent Fish variable; it is
discarded when the launcher exits. Google sign-in is implemented in steps 5–6; real-provider verification remains pending.
Do not use Fish's persistent `set --universal` for credentials.
See [Fish private mode](https://fishshell.com/docs/3.7/cmds/fish.html)
and [Fish read options](https://fishshell.com/docs/3.7/cmds/read.html).

These are runtime settings for the server process. Do not supply the secret as a
Docker build argument, copy it into the image, or embed it in Leptos resources or
the browser WASM bundle. Reuse `secrecy` for the loaded secret. Keep development
and production credentials and data directories separate.

The existing contact form's `PORKMAIL_PWD` is also read once at startup and stored
as an optional `SecretString` in this same `ServerConfig`. Missing or empty SMTP
credentials keep the public site available but disable sending; they are not
required for Google login. Contact handlers borrow the configured password, and
password changes require a restart. Continue storing the production SMTP password
in Fly secrets, never in `fly.toml` or the development launcher.

At production HTTP startup, require all four values and reject empty or
placeholder values. In explicit loopback development, allow both Google
credentials to be absent as described above; if either is provided, require both
to be nonempty, non-placeholder values. Report which setting needs attention
without printing its contents or dumping the environment. Parse `MAJ_APP_ORIGIN`
as a URL: reject credentials,
query strings, fragments, and paths beyond the origin. Require HTTPS in
production; allow HTTP only for explicit loopback development origins. Use the
same origin for callback generation and the mutation Origin checks in step 6.

For example, the production origin produces
`https://www.majkavsek.com/auth/google/callback`. Fly forwards requests to the
server on port 8080, but the public origin remains HTTPS without that internal
port. Never derive it from incoming Host or forwarded headers. Successful login
returns to the fixed `/admin` path; do not accept an arbitrary return URL.

Operator commands are an exception to the HTTP configuration requirements:
`admin ...` commands need `MAJ_DATA_DIR`, but must not require Google credentials
or network access. They must open the same database as the server they manage.

### 4.2. Separate built assets from persistent data

**Status: implemented.** `src/storage.rs` prepares and validates the layout before
the HTTP listener is opened. The database path is reserved as an empty private
file; step 4.3 now initializes SQLite and its session-store schema before serving.

Keep `LEPTOS_SITE_ROOT` pointing at built assets. In the current Docker image,
`LEPTOS_SITE_ROOT=site` resolves to `/app/site` because the working directory is
`/app`. Preserve this arrangement. The proposed production layout is:

```text
/app/site/                       built CSS, WASM, images, fonts, and music
/data/admin.sqlite3              administrators, sessions, and editable records
/data/gallery/originals/         published full-size gallery images
/data/gallery/thumbnails/        matching WebP thumbnails
/data/staging/                   incomplete imports and uploads; never served
```

In development, use the same layout beneath `MAJ_DATA_DIR` rather than `/data`.
The database holds approved identities, login transactions, sessions, shows,
video IDs, and gallery metadata. Image bytes live in the gallery directories;
records reference generated filenames. Biography text remains in Rust.

On first initialization, create the directories with access restricted to the
runtime user and authorized operators. Use private directories (mode `0700`)
and private database files (mode `0600`); a restrictive creation mask such as
`0077` also covers SQLite's companion files. Ownership must match the user
running the application and its administration commands. That user needs write
access to the directory as well as the database file.

The server sets umask `0077` before starting Tokio. Existing data must already
belong to the runtime user with the modes above; startup rejects unsafe paths
rather than silently changing ownership/permissions or truncating a database.
Root aliases are canonicalized before use. Symlinked managed subdirectories and
database symlinks/hard links are rejected. Existing
SQLite `-wal`, `-shm`, and `-journal` files are checked too. To prepare a new
dedicated root as the runtime user, use:

```bash
install -d -m 700 /path/to/dedicated/maj-data
```

Before adopting an existing root, inspect its contents, ownership, and permissions;
correct only the dedicated application paths named by any startup error. Keep
runtime data outside the Docker build context.

Never expose `/data` with a static directory handler or set
`LEPTOS_SITE_ROOT=/data`. Serve published gallery images through the controlled
lookup described in step 7. The database, sessions, backups, and staging uploads
must not become downloadable. Keep staging and final gallery files on the same
volume so publication can use atomic renames.

Startup rejects overlap between the data root and either the configured static
root or this project's `public/` source tree, checking canonical paths and aliases.
Public asset trees must be symlink-free, including dangling links. Keep built
assets immutable while running; only those assets are exposed by the static
handler. Gallery publication uses step 7's controlled catalog lookup.
A filesystem-device check enforces that gallery/staging share the data root's
filesystem. Tests cover private modes, repeat-safe setup, preservation of data,
unsafe links, and HTTP isolation.

### 4.3. Initialize SQLite before accepting requests

**Status: implemented.** `DatabaseManager` in `src/database.rs` owns one private
SQLx pool for application and session storage. Embedded migrations create an empty `administrators` table with
a unique `(issuer, subject)` and disabled-by-default approval records. The session
library initializes `tower_sessions` on the same pool. No login or approval UI is
enabled by this step; section 5 now adds content/login-transaction tables.
Fly startup checks `/data` is mounted on a separate filesystem before storage is
created. The volume itself still needs provisioning in section 10.

**Unresolved release blocker:** current `just audit`/`just deny` checks report
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
for bundled `libsqlite3-sys` 0.37.0. SQLx 0.9.0 caps that dependency below 0.38.0,
but patched releases start at 0.39.0. Do not deploy this database integration until
a supported patched dependency path is available and verified. Private directory
checks limit exposure to attacker-supplied database/WAL files; they do not fix the
vulnerability. No exception or suppression was added. The older `paste`
maintenance advisory remains too.

Use `just db-migrate` for local schema administration (defaults to the development
data directory, or respects `MAJ_DATA_DIR`). It invokes the existing executable's
`admin migrate` command and requires no Google, SMTP, or HTTP-origin configuration.
The deployed binary accepts the same command with `MAJ_DATA_DIR` set. It uses the
same private-path checks, connection options, migrations, and session setup as HTTP
startup, then exits without listening. Unknown commands fail explicitly.
Use `just migration-add descriptive_name` with sqlx-cli 0.9 to create a new SQL
migration. Rebuild after changes; migrations are embedded and tracked by `build.rs`.
Never edit an already applied migration; take a coordinated backup before upgrades.

Use one SQLite database at `MAJ_DATA_DIR/admin.sqlite3`; a separate database
server is unnecessary for this initial release. Both the content queries and
the session store must use this file. Initialize them in this order:

1. Validate the data path and verify that its required storage is available and
   writable. In production, verify the Fly volume is mounted at `/data`; the
   existence of a directory named `/data` alone does not establish persistence.
2. Open the database using consistent SQLx connection options for the HTTP
   server and operator commands. Start with foreign keys enabled, WAL journal
   mode, `synchronous=FULL`, and a five-second busy timeout. Apply the
   connection-specific options to every pooled connection. WAL permits readers
   alongside a writer; it still allows only one writer at a time. Keep write
   transactions short and return a useful error if contention exceeds the limit.
3. Run versioned application migrations and the session store's schema setup
   before HTTP startup. Create an empty approval table; do not grant the first
   login access. Back up existing data before a release that changes its schema.
4. Start serving only after local configuration, storage, and schema setup
   succeed. A missing mount, failed migration, or unreadable database must stop
   startup with a clear error. Do not fall back to an in-memory database or an
   unauthenticated administration mode.

Use SQLx connection options rather than relying on a one-off SQLite shell
command. `FULL` with WAL requests durable commits; it does not protect against
loss of the underlying volume. See [SQLx SQLite configuration](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html),
[SQLite settings](https://www.sqlite.org/pragma.html), and
[WAL behaviour](https://www.sqlite.org/wal.html).
Use sqlx-cli for database management.

SQLite can create `admin.sqlite3-wal` and `admin.sqlite3-shm` beside the database.
Leave their management to SQLite; do not delete them while it is running or
copy only the live main file as a backup. Use the coordinated backup procedure
in step 13. Database transactions also do not atomically include image files;
follow the staging/publication sequence in step 7.

## 5. Implement the server integration before running enrollment commands

**Status: foundation and operator commands implemented.** This step prepares the
server/database integration used by sections 6–7. It does not enable Google login,
callback routes, content mutations, gallery serving, or the three-section editor.
Those workflows remain the next implementation steps. `/admin` and `/api/admin`
already reject unauthenticated requests, and private namespaces preserve no-store
headers. Existing public pages continue loading their built content.

Keep authentication and database dependencies behind the existing `ssr` feature.
Neither the Google secret nor token-processing code belongs in the WASM bundle.
Reuse the existing `secrecy` dependency for the client secret; do not expose auth
state through serialized Leptos resources or derive unredacted debug output for
credentials and login transactions. Restrict database/data-directory access to
the runtime user and authorized operators.

Use these libraries rather than implementing OIDC or session generation:

| Library                                                                                                                        | Role                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| [`openidconnect`](https://docs.rs/openidconnect/latest/openidconnect/)                                                         | Google authorization-code flow, PKCE, discovery, and ID-token validation |
| [`tower-sessions`](https://docs.rs/tower-sessions/latest/tower_sessions/)                                                      | Opaque browser session IDs and server-side session data                  |
| [`tower-sessions-sqlx-store`](https://docs.rs/tower-sessions-sqlx-store/latest/tower_sessions_sqlx_store/) with SQLite enabled | Persistent session store                                                 |
| [`sqlx`](https://docs.rs/sqlx/latest/sqlx/) with SQLite/Tokio support                                                          | Application queries and migrations                                       |

The foundation uses `tower-sessions` 0.15, the SQLx session store 0.16, SQLx 0.9,
and Clap 4 for the operator interface. OAuth2 5 provides secure state/nonce
generation and PKCE; no hand-written random-token or PKCE implementation is used.
OpenID Connect token verification is deferred to section 6, when that library is
actually needed. Recheck its resolved crypto dependencies/advisories then; the
foundation does not retain an unused RSA/token-verification dependency.
At the documentation check, the relevant OpenID Connect release family was 4.1.
The store's
[manifest](https://docs.rs/crate/tower-sessions-sqlx-store/0.16.0/source/Cargo.toml)
uses SQLx 0.9 and session-core 0.15. Resolve compatible features, check
advisories/MSRVs, and commit the resulting lockfile when adding later integration.
The section 4.3 SQLite release blocker remains unresolved; no advisory exception
has been added.

### Code changes

This table is the roadmap across sections 5–7. The foundation is present;
callback/token verification work belongs to 6 and editor/content HTTP work to 7.

| Location                                                 | Required change                                                                                    |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src/main.rs`                                            | Parse operator subcommands before HTTP startup; validate configuration and initialize the database |
| New server-only `src/auth.rs`                            | OIDC callbacks, session policy, and shared administrator/CSRF guards                               |
| `src/database.rs`                                        | Application SQL, database initialization, transactions, and session-store construction              |
| New server-only content module and migrations            | Content import, safe gallery file operations, and schema definitions                               |
| `src/serve.rs`                                           | Session middleware, auth routes, upload handler, gallery serving, and cache rules                  |
| `src/app.rs` and a new admin route                       | Protected administration page with three sections                                                  |
| Shows, Media, and gallery loaders                        | Read editable content from the database instead of immutable files/process caches                  |
| `Cargo.toml`, `src/lib.rs`, Docker/runtime configuration | SSR-only dependencies, module exports, persistent data configuration                               |
| Rust/browser tests and documentation                     | Cover the new access checks, storage, workflows, and operator commands                             |

Implemented modules are `src/database.rs` (`DatabaseManager` and application SQL),
`src/cli.rs` (Clap/operator commands), `src/auth.rs`
(sessions, browser-bound transactions, and guards), `src/content.rs`
(offline import/backup), and the new private-schema migration. The production
router supplies the same manager/configuration/session to Axum handlers and Leptos
server functions. Native tests exercise that actual middleware/context stack;
the test-only probe endpoints are absent from production builds.

Pass database/auth state through Axum state and the Leptos server context as
needed. Install session middleware on callback, admin, and mutation requests.
Verify that Leptos server functions receive the same session context as ordinary
Axum handlers.

### Database and operator interface

Create an administrator table with a local ID, verified issuer, subject, display
label, enabled flag, and session-generation number. Enforce a unique constraint on
`(issuer, subject)`. Begin with **zero enabled administrators**; an empty database
must not enroll the first person who happens to sign in.

The foundation also creates short-lived login-transaction records, the library's session table,
and tables for shows, video IDs, and gallery records. Do not create a biography
table. Keep login transactions bound to the initiating browser session and expire
them after five minutes. Consume them atomically so concurrent/replayed callbacks
cannot reuse the same transaction.

Implement these operator subcommands in the existing executable. Unknown
subcommands must exit with an error rather than accidentally start the server.
Commands use `MAJ_DATA_DIR`, run migrations when needed, and do not need to contact
Google or load the client secret:

| Proposed command                                                     | Contract                                                                                                        |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `admin list-users`                                                   | Show locally approved identities and their enabled status                                                       |
| `admin grant-google --issuer ISSUER --subject SUBJECT --label LABEL` | Explicitly approve one identity; idempotent for an already enabled identity                                     |
| `admin revoke-google --issuer ISSUER --subject SUBJECT`              | Disable the identity and increment its session generation                                                       |
| `admin revoke-all-sessions`                                          | Increment every administrator's session generation                                                              |
| `admin import-content --source PATH`                                 | One-time import of shows, videos, originals, and thumbnails; refuse to overwrite an initialized content catalog |
| `admin backup --output PATH`                                         | Produce a consistent database-and-gallery backup without credentials in its console output                      |

All commands in that table are now available, along with `admin migrate`.
Use `just admin --help` for Clap's help and `just admin SUBCOMMAND ...` locally;
the recipe defaults to the development data directory and removes provider/SMTP
credentials. Results go to stdout (`list-users` is JSON), diagnostics to stderr.
Issuer must be `https://accounts.google.com`; supply the verified subject from
the later login flow, not an email. Subject/label validation and parameterized SQL
protect the local interface. No initial identity is automatically granted access.

Import and backup are **offline maintenance commands**. Stop the HTTP process
before running either one, and restart it afterward. Shared locks on `admin.lock`
allow HTTP and identity operations to coexist; exclusive maintenance locks refuse
to proceed while either is active. Keep the lock file in place. Backup opens the
existing database without applying pending migrations so a pre-upgrade snapshot
really precedes the upgrade. Production maintenance must retain the correct
volume and binary image; see steps 11 and 13.

Import accepts one validated catalog: JSON/calendar dates, HTTPS YouTube IDs,
JPEG/PNG/WebP originals and 480x270 WebP thumbnails. Limits are 2 MiB per JSON,
10000 shows, 1000 videos/photos, and 20 MiB/40 million pixels per image, with
decoder allocation bounded to 256 MiB. It stages all files privately, uses opaque
stored filenames, and commits catalog rows plus a singleton import marker only
after publication. Repeat imports are refused. Private orphans after a crash
are possible; no partially committed catalog is exposed.

Backups use `VACUUM INTO`, private gallery copies, a SHA-256 manifest, and a final
`COMPLETE` marker. Provision the private parent directory first and choose a new
output directory outside served paths/build context. Missing referenced files or
links cause failure; a failed output without a valid completion marker cannot be
restored. Native tests restore a snapshot into isolated storage, verify matching
files/rows, and invalidate restored sessions. Off-machine transfer, encryption,
scheduling, and actual production recovery remain step 13's operational work.

Re-enabling a revoked identity must also advance its generation, so old sessions
cannot become valid again. Validate arguments and use parameterized SQL. Approval
commands require shell/hosting access, not an unauthenticated web endpoint.

## 6. Implement and verify the login flow

**Status: implemented and tested with an isolated mock provider.** The five routes
below are wired into the production router. Real Google consent/redirect checks
in development and production remain required before deployment; automated tests
never use actual credentials or contact Google. Content editors remain section 7.

Implementation uses `oauth2` for code exchange and `openidconnect` 4.1 for RS256
signature, issuer, audience, expiry, and nonce validation. Application checks
add issuance-time, not-before, expiration ordering, subject, and authorized-party
policy. The RSA dependency's private-key timing advisory has a documented
public-key-only exception in both security tools; see
[the assessment and review conditions](README.md#rsa-advisory-applicability).
Only `openid` is requested; access/refresh tokens are not retained.

Provider HTTP uses no redirects, three-second connection/five-second request
timeouts, and 64 KiB response limits. Discovery validates fixed Google endpoints;
`http-cache-semantics` determines metadata/key freshness from HTTP headers with a
one-hour cap and no heuristic lifetime. Non-cacheable responses are discarded
after use; responses without usable freshness are immediately stale. Unknown
key refresh is limited to once a minute and failed lookups back off for 30 seconds.
Login starts and callbacks also have process-wide 20/minute and 60/minute limits.

To approve the initial account, visit `/admin`, sign in, and privately copy the
issuer/subject from the five-minute `/auth/identity` page. Use the existing shell
approval command against that server's data directory, then sign in again. The
unapproved flow creates no approval record or administrator session. The native
logout form POSTs its session-bound CSRF token; APIs can use the existing header.

Use these proposed routes:

| Route                       | Behaviour                                                                                                                |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `GET /auth/google/start`    | Create a short-lived login transaction and redirect to Google                                                            |
| `GET /auth/google/callback` | Consume the transaction, validate identity, and check local approval                                                     |
| `GET /auth/identity`        | Display only the current browser's recently verified identity for manual enrollment; give unapproved accounts HTTP `403` |
| `POST /auth/logout`         | Validate CSRF protection, invalidate the local session, and clear the cookie                                             |
| `GET /admin`                | Render the editor only for an enabled administrator; otherwise display/redirect to login                                 |

The callback exchanges the returned authorization code on the server. Use the
library's asynchronous HTTP client, with redirects disabled and bounded request
timeouts. Use the fixed Google issuer `https://accounts.google.com`; never discover
a provider from user-supplied URLs. See the
[`openidconnect` flow and HTTP-client guidance](https://docs.rs/openidconnect/latest/openidconnect/).

Before creating an administrator session, verify:

1. State matches an unexpired transaction for this browser and is consumed once.
2. The authorization code is exchanged using that transaction's PKCE verifier.
3. Token signature, allowed algorithm, issuer, audience/client ID, expiry, and
   nonce pass the library's validation; apply any additional audience checks its
   API requires.
4. The verified `(issuer, sub)` has an enabled local approval record.

Do not treat a decoded JWT, an access token, a claimed email, or a client-supplied
user ID as proof of authorization. Google describes the identity checks in its
[server verification guide](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token);
the [OIDC specification](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation)
defines the token-validation rules. Google `sub` is the stable account identifier;
store the trusted configured issuer alongside it.

For an approved account, rotate the browser's session ID, issue a fresh CSRF
token, and store only the local administrator ID, its generation, and session
timing information. Discard the login transaction and unused Google tokens. Do
not request offline access or retain refresh tokens for website administration.

For an unapproved identity, create **no administrator session**. Temporarily retain
only the verified identity needed for `/auth/identity`, then redirect there to
remove the authorization code from the browser's current URL. Expire that
information after five minutes. Do not save every rejected identity as a new user.
The page must never display someone else's identity or any token.

Handle consent cancellation and provider failures with a useful login error.
Keep Google key/discovery retrieval bounded and cached for rotation; an outage
must deny new logins without taking the public portfolio offline. Existing valid
local sessions do not need a Google request on every page load. Do not make HTTP
startup depend on a successful Google discovery request: initialize that client
lazily or from valid cached metadata, and return a retryable login failure if it
is unavailable. Persist the pre-login session and transaction before redirecting
the browser; a failed database write must not start a login with missing state.

### Session, authorization, and cache policy

The section 5 foundation already enforces these cookie/guard defaults; this
section must connect the validated login/logout flow to them:

- Production cookie: `__Host-maj_admin`, `Secure`, `HttpOnly`, `SameSite=Lax`,
  `Path=/`, and no Domain attribute.
- HTTP loopback development: a separate cookie name such as `maj_admin_dev`,
  with `Secure=false`. Never use the insecure exception in production.
- Five-minute login/enrollment transaction lifetime; 30-minute admin idle timeout;
  eight-hour absolute session lifetime. Enforce both session limits server-side;
  a sliding cookie expiry alone does not enforce the absolute limit.
- Check the administrator's enabled flag and generation from the database on
  every protected request. This makes revocation effective on subsequent requests.
- Invalidate the local session on logout using the session library. Website logout
  does not sign the browser out of its Google account.

Use the library's [session APIs](https://docs.rs/tower-sessions/latest/tower_sessions/struct.Session.html)
for rotation and logout. See [OWASP session guidance](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
for the cookie and lifetime considerations.
Set SameSite explicitly: the session middleware's default is Strict, whereas
this Google GET callback needs Lax to carry the pre-login cookie across the
redirect. See the [session middleware settings](https://docs.rs/tower-sessions/latest/tower_sessions/service/struct.SessionManagerLayer.html).

Every create/delete/upload operation needs a shared server-side `require_admin`
check and CSRF verification, including direct requests that bypass the page.
Use session-bound CSRF tokens plus exact origin checks on mutations, including
logout. No state changes through GET. Missing/expired admin sessions should yield
HTTP `401` on APIs; disabled access or failed CSRF checks should yield `403`.
SameSite is an additional protection, not the entire CSRF implementation. See
[OWASP CSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).

If mutations use Leptos `#[server]` functions, put them under `/api/admin` and call
the guard in each function. The current `.leptos_routes(...)` integration registers
server functions; protecting only `/admin` does not protect those endpoints.
The multipart upload handler needs the same guard before processing a body.

Change the current blanket response cache mapper: preserve `no-store` for auth,
identity, admin, and mutation responses, including redirects and errors. Public
content responses must revalidate after changes; versioned static assets can stay
cached. Do not let release-mode `max-age=3600` overwrite private cache headers.
Use `Referrer-Policy: no-referrer` on auth/enrollment responses so callback data
cannot be passed to another page through a Referer header.

Rate-limit login starts and callbacks, bound outstanding login transactions, and
clean up expired transactions/sessions. Log action names, local actor IDs, target
IDs, and outcomes as needed; never log secrets, codes, state/nonce values, cookies,
tokens, upload bodies, or raw provider errors. The current request logger uses
paths; keep it from expanding to full callback query strings.

## 7. Implement the three content workflows

**Status: implemented locally.** The authenticated `/admin` page uses
`Referrer-Policy: same-origin` so native POST forms retain their Origin header;
other private responses keep `no-referrer`. Exact Origin and CSRF checks remain required.
Native forms at `/admin` add/remove photos,
shows and YouTube videos. Public loaders query the live catalogs and gallery
handlers enforce publication. Uploads accept 10 MiB, 12 megapixels and at most
6000 pixels per side; decoder allocations are limited to 96 MiB, with one processing
job at a time. Full-size JPEG/PNG outputs and 480×270 WebP thumbnails are re-encoded
without metadata. Names are normalized and duplicate names rejected; content IDs
are never reused after deletion. See README's content-maintenance section.

Import the bundled catalog before serving existing content with this build.
Validation uses isolated local data. Measure peak upload memory on the production
Machine before deployment; the conservative limits are not a production measurement.

The first admin page has **Gallery**, **Shows**, and **YouTube videos** only.

### Gallery

1. Require login and CSRF verification before accepting an upload. Set both
   request-byte and decoded-pixel limits, based on the largest supported photo
   and the production machine's measured memory budget. Bound processing
   concurrency; one upload-processing job at a time is sufficient initially.
2. Accept JPEG, PNG, and WebP uploads. Use a maintained decoder to inspect actual
   bytes, re-encode to supported output, and strip unnecessary metadata. Uploaded
   SVG, HTML, archives, and client-supplied filesystem paths are outside this flow.
3. Generate a safe lowercase filename, replacing spaces with underscores before
   storage and rejecting collisions without overwriting existing files. Generate
   a 480×270 WebP thumbnail, keeping a full-size
   gallery version. Complete both files in staging before publishing a record.
4. Atomically move finished files into their final directories on the same volume,
   then commit the gallery record. If publication fails, clean up unreferenced
   files; a crash should leave an orphan, not a published broken image.
5. On confirmed removal, remove the catalog record first, then clean up its files.
   List and serve only published records so unreferenced files are not public.

These checks follow [OWASP upload guidance](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html).
Preserve existing public URL shapes where practical:
`/img/about_pics/<filename>` and `/img/about_thumbnails/<filename>.webp`.
Dedicated handlers must look up a published record and serve its known file;
reject a missing record with `404` rather than falling through to old built files.
This also preserves the current thumbnail URL convention and exact-match viewer
navigation. Keep path-segment encoding for imported filenames.

### Shows and videos

Give each database record a stable local ID for deletion. Preserve the existing
optional show fields and validate real calendar dates using the existing Chrono
approach. Store dates consistently and retain chronological public sorting.

For videos, accept recognized YouTube URL forms, extract and validate the video
ID, and construct the embed URL on the server. Match allowed hostnames exactly;
do not accept arbitrary iframe HTML or fetch arbitrary submitted URLs.

Refactor the existing show/video/gallery loaders to read the live database. Remove
their process-lifetime `OnceCell` caches for editable data. Initially, query these
small tables directly rather than adding another cache-invalidation system. Reload
the relevant admin/public resource after a successful mutation. Biography data
and rendering remain in Rust, with no admin API. Public visitors already viewing a
page will see new content on their next page load/resource reload; live push
updates are not part of this release.

## 8. Build, configure, and enroll the initial user locally

**Status: complete, confirmed by the owner.** Keep the steps below for setup on
another development machine; do not re-import an initialized catalog.

***STATUS***: done
**Run this step after the local code in 4.1–4.3 and 5–7 is implemented.** Section
Production setup stays pending until sections 10–11. Before starting the HTTP server,
perform the one-time content import using the same `MAJ_DATA_DIR`:

```bash
just admin import-content --source public
```

Skip this command for a catalog already imported; it intentionally refuses to
replace live edits. If the server is running, stop it for import and restart it
afterward. Start the development
server using `just dev-auth` as described in section
4.1. Use the actual development client ID; do not put the secret in a command
literal, Cargo configuration, or a persistent Fish variable.

Open `http://localhost:8080/admin`, start sign-in, and choose the intended account.
Before approval, `/auth/identity` should report no admin access and display the
verified issuer and subject. Check the Google account shown is the intended one.

In another Bash terminal, using that same development data directory:

```bash
export MAJ_DATA_DIR="$HOME/.local/share/maj-admin-dev"
./target/debug/leptos-website-maj admin grant-google \
  --issuer 'https://accounts.google.com' \
  --subject 'REPLACE_WITH_VERIFIED_GOOGLE_SUBJECT' \
  --label 'owner'
./target/debug/leptos-website-maj admin list-users
```

Sign in again to obtain an administrator session. Verify there is exactly **one**
enabled administrator and that all existing shows, videos, photos, and thumbnails
were imported. The import must validate all source records and prepare gallery
files before committing the catalog; on failure it must not expose partial data.
Record import completion and reject subsequent imports that would replace live
edits. Never re-import automatically on each deployment.

## 9. Run focused tests before deployment

### 9.1. Automated checks and what they prove

**Verification record (2026-11-16):** `just check` passed (78 Rust tests plus
Python/Fish checks), the complete server/WASM/styles build passed, the authenticated
Chromium/Firefox workflows passed, and all eight real-process shutdown/storage/
import/backup checks passed. Native LLVM coverage is **80.71%**. Fresh cargo-audit
and cargo-deny still fail on the existing SQLite vulnerability and unmaintained
`paste`; no new advisory exception was added. The public/audio Chromium/Firefox
suite passed 71 cases, with one high-zoom caption assertion failing at 99.94%
intersection and two fixture-only editor cases skipped. That assertion now uses
the adjacent caption test's 99% subpixel tolerance; its targeted rerun passed in
both browsers. The editor cases run separately through `just test-admin-browser`.
WebKit was not run. The extended real-Google checklist remains unconfirmed.
Section 8's successful local enrollment is complete.

Run from the repository root. None of these commands needs Google/SMTP secrets;
provider tests use signed fixture tokens, a loopback fake provider and temporary
private databases. The authenticated browser harness creates test-only sessions in
isolated storage; no production login bypass exists.

```bash
just check
just build
just test-admin-browser
just test-shutdown
just coverage --skip-clean --jobs 2
just security
```

`just check` checks formatting, strict Clippy for SSR and browser/WASM, all Rust
tests, and Python/Fish script checks. `just build` builds the complete server,
hydration bundle and styles. `just test-admin-browser` exercises native forms in
Chromium and Firefox, including removing imported records, single-editor navigation,
Back/Forward, uploads, public lookups and logout. The test harness requires installed
Playwright browsers (`just e2e-setup`). `just test-shutdown` also exercises startup,
private storage, operator import/backup and runtime locks using real local processes.
Coverage reports go to `target/coverage/`; `--skip-clean` reuses the instrumented
build for this follow-up run. Use `just clean` at a natural daily stopping point.

If `just security` stops at cargo-audit, run `just deny` separately as well. A failed
security check is not a successful release gate. The existing SQLite advisory and
unmaintained `paste` remain visible; the documented RSA exception applies only to
the reviewed public-key verification path. Resolve the SQLite release blocker
before deploying; do not add exceptions to get a green status.

| Area | Existing evidence / required follow-up |
| --- | --- |
| Configuration | Rust tests reject missing/partial production credentials and invalid origins; loopback public mode works without credentials. |
| Login protocol | Fake-provider tests cover browser/state binding, expiry, replay, PKCE, nonce, signature/claim failures, provider response bounds and rotation/backoff. |
| Approval/session | Tests cover unknown/disabled identities, ID rotation, idle/absolute expiry, generations and logout. Real-provider account selection is checked manually below. |
| Authorization/CSRF | Direct native form submissions and guarded APIs reject anonymous/revoked users, wrong/missing CSRF and foreign Origins; tests bypass the UI deliberately. |
| Privacy | Router/handler tests assert no-store on private errors/redirects; authenticated `/admin` uses same-origin referrers for native form Origin, other private responses use no-referrer. Manually inspect real-provider logs/cookies below. |
| Content | Import is repeat-safe. Imported records survive database reopen, appear in their Published list, and can be removed. IDs are not reused, so stale removal forms cannot delete replacement records. |
| Uploads | Tests cover byte/pixel limits, corrupt data, traversal names, filename collisions, JPEG/PNG/WebP decoding, EXIF rotation, metadata stripping, publication failure rollback and deleted/orphan file denial. Production peak memory and crash/disk failure exercises remain deployment work. |
| Persistence/recovery | Tests reopen durable SQLite data, restore a coordinated database/gallery backup and invalidate old sessions. Production Machine/release persistence still requires sections 10–11. |

Keep the existing public-page, audio and gallery browser regressions as well:
start an isolated server as described in BUILD.md, import into its data directory
before starting HTTP, then run `just e2e --project=chromium --project=firefox`.
WebKit needs its system libraries installed before its result counts as tested.
Never point destructive editor tests at the owner's working catalog.

### 9.2. Prepare a real Google test safely

COMPLETE

Section 8 confirms the basic local sign-in/enrollment. This checklist additionally
checks consent cancellation, another account, session isolation and native edits.
Record the browser/version, date, application revision and pass/fail for each case.
Leave anything not performed marked **not tested**. Automated fixture tests cannot
verify Google Console configuration, the selected Google account or real browser
cookies.

1. Use the **development** project `maj-webpage-admin-development` and its Web
   client from section 3. Keep the browser origin exactly `http://localhost:8080`
   and registered redirect exactly `http://localhost:8080/auth/google/callback`.
   Do not switch to `127.0.0.1` midway: it is a different cookie origin. Google
   requires the redirect URI to match the registered value; see
   [Google's server-flow documentation](https://developers.google.com/identity/protocols/oauth2/web-server#redirect-uri).
2. Run `just dev-auth`. Paste the **development** client secret into its hidden
   prompt; the Fish launcher clears the current clipboard before starting Cargo.
   Do not type it in a shell command, add it to Cargo config, screenshot it, or
   share DevTools exports. Saved clipboard history is separate from the cleared
   current clipboard. Use the same `MAJ_DATA_DIR` as section 8.
3. Use Firefox first (then repeat the core sign-in/edit/logout in another installed
   browser). Open `/admin` in a fresh private window. An existing Google session may
   skip account selection or consent; this is normal. Verify the selected account
   on Google's page before continuing. If needed, use a separate browser profile
   already signed into the intended account rather than changing the app's scopes.
4. Keep an ordinary browser/profile open separately for unauthenticated public
   checks. Two private windows in one browser may share cookies; use a genuinely
   separate profile/browser to test isolation. The development server remains
   loopback-only. Use uniquely named disposable content for mutation tests; do not
   remove real shows, gallery photos or videos just to check a button.
5. If using a second Google account, ensure the development Google consent policy
   permits it (add it as a Test user where required). **Do not grant it local admin
   approval.** Google accepting an account is separate from this app authorizing it.

Google's [OIDC flow and claim documentation](https://developers.google.com/identity/openid-connect/openid-connect)
is the provider reference. This application requests only `openid`, identifies an
account by verified issuer plus subject, and checks local approval separately;
an email address is not the enrollment key. It does not need YouTube or Drive API
access for the content editors.

### 9.3. Real Google sign-in and account isolation checklist

tested on: FIREFOX: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:160.0) Gecko/20100101 Firefox/160.0

- [x] **Anonymous access:** `/admin` shows sign-in, without editor forms. Direct
  `/admin?section=shows` or `/admin?section=videos` does not expose published admin
  lists. `/auth/identity` without a verified recent identity returns 401.
- [x] **Approved account:** start from the website's Sign in with Google link,
  complete Google's flow with the owner account, and arrive on `/admin`. The
  page shows the approved local label and Gallery; no code/token remains in the
  resulting URL. Compare the cookie's value locally before/after sign-in if a
  pre-login cookie exists: the session ID must change. Do not record its value.
- [x] **Cookies/privacy:** inspect cookie attributes in Firefox Storage. Local
  `maj_admin_dev` is HttpOnly, SameSite=Lax and Path=/; Secure is deliberately off
  on loopback HTTP. Production later uses `__Host-maj_admin` with Secure, Path=/
  and no Domain. `/admin` is no-store with same-origin referrers; auth responses
  and errors are no-store with no-referrer. No browser-side Google client secret
  or ID/access token should appear in HTML, browser storage or scripts.
- [/] **Cancellation:** sign out of the app, start a fresh attempt and decline or
  cancel on Google's page **if Google offers that control**. A returned
  `access_denied` should show the cancellation page and grant no session. Closing
  the Google tab alone produces no callback; verify the app remains signed out.
  If prior consent bypasses the screen, record cancellation as not exercised and
  use a fresh permitted account/profile if available. Do not assume a missing
  consent page means the application failed.
  NOTE: Google doesn't offer that control
- [x] **Unapproved account:** in a separate browser/profile start a new attempt
  using the unapproved Google account. After Google verifies it, expect
  `/auth/identity` with HTTP 403 and only that account's issuer/subject. It must
  not show editors or another browser's identity. `/admin` remains denied, and
  after five minutes `/auth/identity` no longer shows the identity. Do not grant
  this account; an account Google rejects before returning to the app does not
  test local approval.
- [x] **Fresh attempts/replay:** always begin again at `/auth/google/start` after
  an error. Automated tests cover replay and wrong-browser state; do not paste
  real callback URLs/codes into tickets, terminals, logs or another browser just
  to reproduce them. A reloaded used callback must not create another session.
- [x] **Logout:** from an authenticated editor click Sign out; expect the sign-in
  page. Reload `/admin?section=shows` and use Back then reload: neither can restore
  access. Public content stays visible. Application logout does not sign the
  browser out of Google, so a later new sign-in may skip Google prompts.

### 9.4. Real editing, expiry and restart checks

- [x] Switch between Gallery, Shows and YouTube videos. Exactly one editor is
  displayed; the current link is marked and Back/Forward work. Each Published
  list includes existing imported content, with an explicit removal checkbox.
- [x] Add one disposable show (including a leap-day date), one recognized YouTube
  URL and one small JPEG/PNG/WebP photo with a unique filename. Use the public
  pages in the separate browser to verify the new records and the thumbnail/full
  image. An invalid calendar date, disguised/non-image photo and unsupported
  URL must not publish; normal browser constraints may prevent submission first.
- [x] Confirm and remove **only the disposable records**. Check the published
  lists and freshly loaded public pages; the removed original and thumbnail
  URLs must return 404. Existing unrelated records remain. A second submit of an
  old removal form must not remove any later record.
- [x] For expiry, use disposable unsaved form text. Leave an authenticated form
  idle for more than 30 minutes without polling or navigating in that session,
  then submit. Expect denial and no change. The eight-hour absolute limit is
  covered in automated tests; a real eight-hour wait can be recorded separately.
  After sign-in, re-enter the disposable data deliberately. **Unsaved native forms
  are not a persistent draft feature:** navigation/auth expiry can lose text and
  always requires reselecting a file. Browser Back/autofill is not guaranteed.
  Keep substantial text in a local draft before testing expiry; never bypass the
  guard to preserve a submission.
- [x] For revocation, keep one disposable edit form open, then use section 12's
  revoke command for a test identity/session. Its next submission must fail with
  no content change. Restore the intended owner's approval deliberately and
  sign in again; do not leave the owner accidentally disabled after testing.
- [x] Before restart testing, save one uniquely named disposable record and note
  its public appearance. Stop the watcher/server, run `just build`, then restart
  with `just dev-auth` and the **same** data directory. Confirm the record remains
  on the public page and in Published; remove it afterward. Never reset SQLite
  or rerun the initial import as a substitute for checking persistence.
- [x] Check local logs without exporting secrets: requests should log paths and
  statuses, never callback query strings, authorization codes, state/nonce,
  session/CSRF values, Google tokens or client secrets. Login and mutation logs
  may contain the local administrator ID and action. Do not attach HAR files,
  full cookie panels, callback URLs or credential prompts to the evidence.

If something fails, record the step, browser and HTTP status with a redacted
message. For `redirect_uri_mismatch`, recheck the exact development callback and
client; for an app 403, restart the flow and check the approved issuer/subject;
for 503, check provider connectivity and the secret supplied to the launcher.
Google may reject an account before the callback for consent policy reasons;
that is different from this application's approval denial. Finish the extended
manual checklist before marking section 9 fully complete.

## 10. Prepare Fly persistence and production configuration

**Status: pending production setup and verification.** Complete section 9's
required checks and resolve the dependency release blocker before carrying out
this production setup. Section 4 implements local configuration, private storage,
and SQLite startup checks; this section attaches that storage to a persistent Fly
volume. The checked-in `fly.toml` now specifies the `maj_data` mount at `/data`;
the actual volume attachment and production persistence still require verification.

Work through 10.1–10.4 to choose and prepare the intended volume and plan migrations.
Section 10.5 contains the acceptance checks to perform **after deployment in
section 11**; read them now so you know what evidence to collect. Section 10.6
covers capacity/recovery. Complete 10.7's production secret import as the final
configuration action before deploying. Mark production persistence verified only
after the restart, fresh-release, and isolated restore checks pass.

### 10.1. Decide what survives a release

Keep built assets separate from persistent runtime paths:

| Location | Contents | Release behaviour |
| --- | --- | --- |
| `/app/site` | Built CSS/WASM, music, fonts, and initial source content | Replaced with the image |
| `/data/admin.sqlite3` | Approved identities, sessions, and editable records | Retained on the volume; migrated in place |
| `/data/gallery/originals` and `/data/gallery/thumbnails` | Published image files | Retained on the volume |
| `/data/staging` | Incomplete uploads/imports | Private working space on the same volume |

Setting `MAJ_DATA_DIR=/data` selects a path; the `[[mounts]]` configuration attaches
persistent storage there. Fly's root filesystem is ephemeral, so `/tmp`,
`/app/site`, and a plain directory named `/data` cannot hold the live catalog.
The Docker image carries code and built assets; it must not contain a production
database, uploads, or backups.

Use **one serving Machine attached to one data volume** initially. Each volume
attaches to one Machine, lives in one region/on one host, and has no automatic
replication to other volumes. A second independent database would split approvals,
sessions, and content between requests. This design accepts downtime during
releases or host failure. More availability requires a separate replication plan.
See [Fly Volumes](https://fly.io/docs/volumes/overview/).

### 10.2. Inventory the current Machines and volumes

Inspect current state instead of relying on section 1's
older record of two Machines:

```bash
fly status --app maj-webpage
fly machine list --app maj-webpage
fly volumes list --app maj-webpage
```

Record the intended serving Machine ID, its region, and the intended volume's
name, ID, size, and attachment. `maj_data` is a reusable volume **name**; a value
such as `vol_...` identifies one particular volume. Record the ID so a similarly
named empty volume cannot be mistaken for the live catalog.

If there are multiple serving Machines, deliberately retain the one with the
intended data and retire the other serving replicas before enabling editing.
Inspect and back up any existing state before changing that topology. A fresh
database on each replica is not a migration or a backup of the intended database.
Step 11's `--ha=false` prevents spare creation during deploy; it does not remove
existing replicas.

Create a volume only if no suitable one exists, using the app's intended region
(`ams` currently):

```bash
fly volumes create maj_data --region ams --size 1 --app maj-webpage
```

This 1 GiB example is a starting value, not a measured capacity recommendation:
account for originals, thumbnails, staging space during uploads/imports,
SQLite/WAL growth, and headroom. See
[volume creation](https://fly.io/docs/flyctl/volumes-create/).

### 10.3. Configure the mount and prepare its permissions

Merge these settings into `fly.toml`, replacing the domain placeholder and
preserving existing ports, health checks, SMTP configuration, and shutdown settings:

```toml
[env]
PRIMARY_REGION = 'ams'
MAJ_APP_ORIGIN = 'https://YOUR_DOMAIN'
MAJ_DATA_DIR = '/data'
GOOGLE_CLIENT_ID = '774712125801-lljgocauoic4un01lt1ml1hg27l10tep.apps.googleusercontent.com'

[[mounts]]
source = 'maj_data'
destination = '/data'
processes = ['app']
```

`source` selects the volume name, not its ID. The region and available volumes
also affect attachment, so verify the resulting Machine/volume ID pairing after
deployment. `processes` matches this repository's existing `app` service group.
See [Fly mount configuration](https://fly.io/docs/reference/configuration/#the-mounts-section).

The mounted root must belong to the server's effective user and have mode `0700`.
The current Dockerfile uses the base image's root user (UID 0); this is separate
from your local Linux account. Recheck ownership requirements if the runtime
image later selects another UID. Startup refuses a root with incorrect ownership
or permissions and will not silently change them.

For a **new, dedicated, unattached volume**, prepare its root before booting the
new application. Otherwise a permission failure can leave the application exiting
before you can inspect it through SSH. One option is a temporary maintenance shell
attached to the specific new volume. This creates a temporary billable Machine;
run it only during the planned production setup, using the recorded volume ID:

```bash
fly machine run --app maj-webpage --region ams \
  --volume REPLACE_WITH_VOLUME_ID:/data --shell debian:bookworm-slim
```

Inside that temporary Linux shell, inspect the mount, owner, and directory listing:

```bash
id
awk '$5 == "/data" { print }' /proc/self/mountinfo
stat -c '%a %u:%g %n' /data
ls -la /data
```

Confirm this is the intended new application volume. As its intended runtime user,
set the dedicated root's mode, verify it, and leave the temporary shell:

```bash
install -d -m 700 /data
stat -c '%a %u:%g %n' /data
exit
```

The `--shell` workflow removes the helper Machine on exit. Check `fly machine list`
and `fly volumes list` afterward: the helper should be gone and the volume should
still exist, ready for the serving Machine. Do not attach a helper concurrently
to a volume already used by a running Machine. For existing data, inspect and
back up first; correct only the application paths named in startup errors.
The application creates its gallery/staging directories and database on startup.
See [Fly Machine run options](https://fly.io/docs/flyctl/machine-run/).

### 10.4. Keep migration work on the volume-backed Machine

On each release, the existing executable validates configuration, checks that
`/data` is mounted on a separate filesystem, validates private storage, opens
SQLite, and runs application/session schema setup before opening the listener.
The mount check establishes availability; the recorded volume ID and a restore
test establish that you selected the intended data and can recover it.

Back up the current source content before the initial catalog migration. For an
existing catalog, take a coordinated off-machine backup before schema-changing releases. Stop
editing during the maintenance window and avoid overlapping old/new application
versions against a changing schema. Let the normal startup sequence migrate the
existing file. `admin migrate` is also available for deliberate operator use on
the correctly mounted Machine; it must run from the intended release binary.

Keep migration execution out of the Docker build and Fly's `release_command`.
The latter uses a temporary Machine without the attached volume, so it cannot
migrate this application's persistent database. See
[Fly release commands](https://fly.io/docs/reference/configuration/#the-deploy-section).

Import bundled content once in steps 8/11. A routine deploy must preserve edits,
gallery files, approved users, and sessions still valid under the session policy.
Do not re-import built content at every boot. Redeploying an older image also
does not undo database migrations; check schema compatibility before rolling back,
and use the recovery procedure in step 13 when a restore is required.

Do not bake the database or secrets into the Docker image. Ensure the data root
is writable by the runtime user and cannot be served by the static file handler.

### 10.5. Verify persistence after the deployment in section 11

**Perform these checks after section 11 deploys and imports the catalog.**
Do not deploy early just to complete this subsection.

After the planned deployment, repeat the Machine/volume inventory and confirm
there is exactly one intended serving Machine. Use an explicitly selected SSH
session to inspect that Machine:

```bash
fly ssh console --app maj-webpage --select
```

In its Linux shell, verify the actual mount, permissions, and available capacity:

```bash
awk '$5 == "/data" { print }' /proc/self/mountinfo
df -h /data
stat -c '%a %u:%g %n' /data /data/gallery /data/staging /data/admin.sqlite3
```

Expect owned directories with mode `700` and a database with mode `600`.
Successful HTTP startup confirms schema setup finished; merely finding a file
called `admin.sqlite3` does not prove its initialization or persistence.
See [Fly SSH selection](https://fly.io/docs/flyctl/ssh-console/).

Check filesystem persistence independently of the content editors:

1. In that remote shell, create a uniquely named private probe in staging:

   ```bash
   maj_probe_path=$(mktemp /data/staging/persistence-check.XXXXXX)
   printf 'persistence-check\n' > "$maj_probe_path"
   printf '%s\n' "$maj_probe_path"
   ```

   Record the printed path before exiting SSH. `mktemp` avoids replacing an
   existing upload. This file remains private and is not a database backup.

2. From the local terminal, restart the recorded serving Machine:

   ```bash
   fly machine restart REPLACE_WITH_MACHINE_ID --app maj-webpage
   ```

   Reconnect to that Machine and check the recorded probe's contents and the
   database's presence. Confirm the volume ID/attachment is unchanged.

3. Perform the planned fresh-image release in step 11, then repeat the checks on
   the resulting serving Machine. Verify the recorded volume ID even if the
   Machine ID changes. Remove only the known probe file after both checks pass.

4. Save disposable show/video/gallery changes through the editors and
   verify it survives both operations too. Verify approved identities and eligible
   sessions are preserved; an expected session expiry is not data loss.

The probe checks filesystem persistence. The edited-record check and a successful
restore into an isolated data directory are additional acceptance requirements
before relying on production editing. See
[Machine restart](https://fly.io/docs/flyctl/machine-restart/) and step 13.

### 10.6. Monitor capacity and recovery

Inspect `df -h /data` and, when useful, `du -sh /data/gallery /data/staging` on the
serving Machine. Include staged uploads and WAL growth when assessing free space.
Extend the recorded volume before it fills; Fly supports growth, not shrinking.
Use the [volume management instructions](https://fly.io/docs/volumes/volume-manage/)
and verify the resulting capacity inside the Machine.

Keep coordinated database/gallery backups off the volume and test recovery using
step 13. Automatic Fly snapshots supplement that process; they can lag recent
writes and are not proof that an application backup restores correctly. Restore
tests must not overwrite the live volume. Never delete live SQLite companion files
to reclaim space or copy just the live main file as a backup.

If mounting, permissions, migration, or writes fail, stop and investigate the
recorded volume and startup error. Do not work around a missing mount by creating
an empty directory, dropping the database, or disabling checks. Request handlers
must report failed writes instead of reporting unsaved changes as successful.

### 10.7. Review Google production configuration and stage its secret

Before launch, review the Google project's Audience publishing controls and
complete any branding/domain verification the Console requests. Moving to In
production does not grant anyone administration rights in this application;
the local approval table remains authoritative. Identity-only Testing behaviour
is described in step 2, rather than relying on a presumed seven-day expiry.

**Pending: the production secret has not been added to Fly.** As the final
configuration step before the deployment in step 11, retrieve the secret for the
OAuth client in `maj-webpage-admin-production`. The project ID is not the client
ID. Import its secret without putting the value in command history, and stage it
for the new release:

```bash
read -r -s -p 'Production Google client secret: ' maj_google_secret
printf '\n'
printf 'GOOGLE_CLIENT_SECRET=%s\n' "$maj_google_secret" | \
  fly secrets import --stage --app maj-webpage
unset maj_google_secret
```

The value passes to Fly through stdin; do not enable shell tracing. See the
[secret import command](https://fly.io/docs/flyctl/secrets-import/).

## 11. Deploy, import content, and approve the production account

After deployment and import, complete section 10.5's restart/release persistence
checks and the isolated restore test in section 13. Only then mark production
persistence verified in section 10.

**Before running `just deploy`, finish step 10's pending production secret import.**
Use `fly secrets list --app maj-webpage` to check that `GOOGLE_CLIENT_SECRET`
exists without displaying its value. Ensure it belongs to the same OAuth client
as the configured production `GOOGLE_CLIENT_ID`. Do not deploy this configuration
change with missing credentials: the new server rejects that startup.

The Docker build must include the implemented operator subcommands in the existing
server executable. Run schema migrations on the machine with the volume mounted,
before opening the HTTP listener. Fly build/release-command environments do not
provide the mounted volume needed for these data migrations.

```bash
just deploy --ha=false
fly status --app maj-webpage
fly machine list --app maj-webpage
```

`--ha=false` prevents deploy from creating spare machines; it does not remove
pre-existing replicas. Confirm the actual machine/volume arrangement. See
[deploy options](https://fly.io/docs/flyctl/deploy/) and
[volume lifecycle limits](https://fly.io/docs/volumes/overview/).

Import before exposing the first new release publicly. Plan a short maintenance
window for the offline import; public content now reads the live database. Do not
automatically restore bundled content over an initialized database.

On the single intended data Machine, use a maintenance console with the HTTP
process stopped and the intended release image/volume attached. Section 13
explains that requirement. Run the import once inside that console, then restore
the normal server entrypoint and check health:

```bash
MAJ_DATA_DIR=/data /app/leptos-website-maj admin import-content --source /app/site
```

Check the import result and public gallery/show/video pages before allowing edits.
Then visit the canonical production `/admin`, sign in with the intended Google
account, and obtain its verified identity from `/auth/identity`. Use the verified
production result even if the same account was enrolled in development.

```bash
fly ssh console --app maj-webpage -C \
  '/app/leptos-website-maj admin grant-google --issuer https://accounts.google.com --subject REPLACE_WITH_VERIFIED_GOOGLE_SUBJECT --label owner'
fly ssh console --app maj-webpage -C \
  '/app/leptos-website-maj admin list-users'
```

These operator commands are implemented. Identity commands can run alongside HTTP
and are invoked through Fly's existing
[SSH console](https://fly.io/docs/flyctl/ssh-console/). If there is any ambiguity
about the target machine, select the one with the intended `/data` volume rather
than running approval/import commands against an arbitrary replica.

Sign in again. Confirm exactly one approved account, all three editor sections,
rejection of direct unauthorized mutations, correct secure cookies/cache headers,
and successful logout. Add/remove disposable content, restart the machine, and
confirm the saved state survives. Public content must not need a rebuild after
these edits. Check that a direct Fly hostname or alternate domain cannot bypass
authentication or confuse callback/session origins.

## 12. Add a second administrator later

No second account is created or pre-approved in the initial release.

1. Have the intended additional user secure their Google account and sign in to
   the canonical production site. Until approved, they receive no admin access.
2. Obtain that account's verified issuer/subject from its own `/auth/identity`
   page. Confirm its ownership out of band before granting access.
3. Run `admin grant-google` through the same Fly SSH command, with that subject
   and a distinct label such as `second-admin`.
4. Check `admin list-users`: there should now be two enabled identities.
5. Have the second user sign in again and verify their access. Both users have
   the same content permissions; the web UI does not manage administrator grants.

A new Google OAuth client, shared password, or website release is not needed to
add an approved identity. Removing one uses `admin revoke-google` with the same
issuer/subject. Confirm their next protected request fails. Requests that already
committed before revocation are not rolled back.

## 13. Backups, recovery, and routine maintenance

Use [MAINTENANCE.md](MAINTENANCE.md) for the ongoing operating procedures,
backup verification, restore steps, and deployment/rollback checklist.

**The offline `admin backup` command is implemented; production scheduling and
recovery validation remain pending.** It uses an exclusive cross-process file
lock, `VACUUM INTO`, and private copies of gallery files. It refuses to proceed
while the HTTP process or identity commands hold a shared lock. It opens the
existing database without applying pending migrations, so it can create a true
pre-upgrade snapshot. Keep backup destinations outside served paths/build context.
See [SQLite's backup documentation](https://www.sqlite.org/backup.html).

Output is a new directory containing `admin.sqlite3`, `gallery/`, `manifest.json`
with SHA-256 checksums, and a final `COMPLETE` marker containing the manifest hash.
It refuses existing destinations and verifies that all referenced gallery files
were copied. A failed directory without a valid completion marker is incomplete
and cannot be restored. Automatic encryption/off-machine transfer is not provided.

For local work, stop `just dev`/`just run`, provision a private backup parent outside
the repository, then use `just admin backup --output /absolute/private/unique-name`.
Restart the server after maintenance. On Fly, do not run backup/import in an
ordinary SSH session while the server is serving requests: the lock will reject
it. A stopped Machine cannot provide a normal SSH console either.

During a planned window in steps 10–11, retain the existing volume/image and put
the attached Machine in a maintenance configuration with a running shell/sleep
process instead of HTTP. For the current Debian image, the temporary entrypoint
can be `/bin/sleep` with command `infinity`; record the original image, entrypoint,
command, and restart policy before changing them, then restore them after the
operation. Verify this hosting procedure during deployment preparation. Do not
destroy the serving Machine or detach its volume as a shortcut. This is distinct
from section 10.3's helper for a new, unattached volume.

Inside the selected volume-backed maintenance console, as the runtime user,
provision the dedicated backup parent and create a uniquely named snapshot:

```bash
install -d -m 700 /data/backups
MAJ_DATA_DIR=/data /app/leptos-website-maj admin backup --output /data/backups/pre-migration-YYYY-MM-DD
```

Replace the date placeholder with a unique date/name, inspect the command's
success result, and transfer the completed directory to the chosen off-machine
backup destination. Do not count this local staging copy as the off-machine copy.

Schedule daily backups to a separate protected storage destination and keep
several generations. Take another backup before migrations or substantial
deletions. A second copy on the same volume is not the off-machine backup. Fly
snapshots can supplement this but are not the entire backup plan. Backups contain
administrator/session information and should be encrypted and access-controlled.

Test restoration to a separate development data directory before launch. Restore
the database and its matching gallery files together, verify all referenced files,
then run `admin revoke-all-sessions` on the restored database before exposing it.
Do not restore production session cookies into a publicly reachable test system.
Record the restore procedure and the last successful restore test.

Keep approval/revocation available through hosting access so losing a Google
account does not remove the operator's recovery path. If the only Google account
becomes unusable, verify a replacement account and explicitly grant it through
the operator command, then revoke the old identity. There is no public recovery
or automatic administrator-enrollment endpoint.

Keep Google project contacts and dependencies current. Google documents removal
of OAuth clients unused for six months; occasional admin sign-in and project
monitoring matter for an infrequently edited site. Rotate a compromised client
secret, and separately revoke local sessions: changing the Google secret alone
does not invalidate this application's existing sessions. See
[Google's credential and lifecycle policies](https://developers.google.com/identity/protocols/oauth2/policies).

## 14. Troubleshooting and completion criteria

| Symptom                                   | Check                                                                                                                      |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `redirect_uri_mismatch`                   | Compare the registered callback with the fixed origin, client environment, hostname, port, path, and slash                 |
| Google succeeds, website denies access    | Verify the local approval record and issuer/subject; a Test users entry or matching email is not a grant                   |
| Login loops or state fails                | Use the canonical hostname; check cookie settings, login expiry, a shared persistent store, and competing sign-in attempts |
| Browser reports CSRF failure              | Refresh the login/form session; check exact origin and token handling, not an authorization bypass                         |
| Edits vanish after deploy                 | Check `/data` mounting, runtime permissions, actual machine count, and accidental re-import of bundled content             |
| Edits are saved but public pages stay old | Check remaining `OnceCell` uses, HTTP caching, and resource reloads                                                        |
| Google is unavailable                     | Show a retryable login error; keep public pages and existing eligible local sessions usable                                |
| Logout/revocation appears ineffective     | Check the server-side session/generation guard and private cache headers                                                   |

The release is ready when the single approved user can manage photos, shows, and
videos; unapproved/direct/forged requests cannot; data survives deployment; and a
backup has been restored successfully. Biography changes still use the existing
edit/build/release workflow. Link this guide from the runtime documentation when
the feature is implemented and update the documented contracts to the actual
commands, configuration, routes, and tested versions.
