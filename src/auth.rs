//! Server-only session policy, login state, and administrator request guards.
//!
//! [`DatabaseManager`] owns persistence; this module creates protocol secrets,
//! configures browser cookies, and checks the current approval on each request.
//! Session cookies contain opaque IDs, while authentication data stays in SQLite.
//! Leptos server functions and ordinary Axum handlers use the same guards.
//!
//! Google callbacks verify identity before establishing an approved session.
//! Saving or consuming login state alone never grants administrator access.

mod editor;
mod google;
mod login;

use crate::{database::DatabaseManager, server_config::ServerConfig};
use axum::response::{IntoResponse, Response};
use http::{
    header::{CACHE_CONTROL, ORIGIN, REFERRER_POLICY},
    HeaderMap, StatusCode,
};
pub(crate) use login::routes as login_routes;
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, Session, SessionManagerLayer,
};
use tower_sessions_sqlx_store::SqliteStore;

const ADMIN_KEY: &str = "administrator";
const BROWSER_KEY: &str = "login_binding";
const IDLE_SECONDS: i64 = 30 * 60;
const ABSOLUTE_SECONDS: i64 = 8 * 60 * 60;

// Errors and their response conversions.

/// Public authorization failures with fixed HTTP statuses and generic messages.
/// Database and session errors are deliberately reduced to `Unavailable` so
/// responses cannot expose stored authentication data or backend diagnostics.
#[derive(Debug, thiserror::Error)]
#[error("Authentication request failed")]
pub enum AuthError {
    /// HTTP 401: no administrator session, or invalid/expired session timestamps.
    Unauthorized,
    /// HTTP 403: approval, generation, CSRF, or browser-bound login checks failed.
    Forbidden,
    /// HTTP 503: persistence or required server/request context is unavailable.
    Unavailable,
    /// HTTP 429: outstanding login transactions reached a browser or global limit.
    Busy,
}

impl AuthError {
    /// Converts a guard failure for a Leptos server function without exposing details.
    /// Sets the matching HTTP status when a response-options context is present;
    /// the serialized server-function error always uses a generic message.
    pub fn server_error(self) -> leptos::prelude::ServerFnError {
        let response = self.into_response();
        if let Some(options) = leptos::prelude::use_context::<leptos_axum::ResponseOptions>() {
            options.set_status(response.status());
        }
        leptos::prelude::ServerFnError::new("Authorization failed")
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Sign in required"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Access denied"),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication is temporarily unavailable",
            ),
            Self::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many sign-in attempts; retry later",
            ),
        };
        (
            status,
            [
                (CACHE_CONTROL, "no-store"),
                (REFERRER_POLICY, "no-referrer"),
            ],
            message,
        )
            .into_response()
    }
}

/// Discards database diagnostics at the authorization boundary.
impl From<sqlx::Error> for AuthError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

/// Treats failed session reads/writes as unavailable rather than granting access.
impl From<tower_sessions::session::Error> for AuthError {
    fn from(_: tower_sessions::session::Error) -> Self {
        Self::Unavailable
    }
}

// Public authentication data and entry points.

/// An explicitly approved issuer/subject identity loaded from the local database.
///
/// The issuer and subject identify the Google account; the label is only for
/// operators. Guards reload this record so disabling an account or changing its
/// generation invalidates sessions that were previously accepted.
#[derive(sqlx::FromRow, Serialize)]
pub struct Administrator {
    /// Stable local database ID, also stored in the administrator session.
    pub id: i64,
    /// Verified identity-provider issuer, paired with `subject` for lookup.
    pub issuer: String,
    /// Provider account identifier; an email address is not used as the identity.
    pub subject: String,
    /// Human-readable operator label, with no role in authorization.
    pub label: String,
    /// Whether this identity may currently establish and use an admin session.
    pub enabled: bool,
    /// Revocation counter; accepted sessions must carry the current value.
    pub session_generation: i64,
}

/// Values for a future authorization redirect after protocol state is persisted.
/// No Debug/serialization implementation is provided: state and nonce must not
/// accidentally appear in logs. The PKCE verifier remains in server storage.
pub struct PendingLogin {
    /// Random state sent to the provider and matched against its stored hash.
    pub state: CsrfToken,
    /// S256 challenge sent to the provider; its verifier stays server-side.
    pub challenge: PkceCodeChallenge,
    /// Expected ID-token nonce, retained server-side for later claim validation.
    pub nonce: SecretString,
}

/// Secrets recovered by consuming one valid, browser-bound login transaction.
/// Consumption alone does not verify identity; the callback must still exchange
/// the authorization code and validate the returned provider claims.
pub struct LoginTransaction {
    /// Server-held verifier for the authorization-code exchange.
    pub verifier: PkceCodeVerifier,
    /// Nonce that the verified ID token must contain.
    pub nonce: SecretString,
}

/// Establishes a session for an already verified, explicitly enabled identity.
///
/// The caller must verify the provider's identity claims before supplying issuer
/// and subject. This checks local approval, rotates the session ID, clears earlier
/// session data, and saves fresh timestamps, generation, and CSRF token.
/// Unknown/disabled identities return `Forbidden`; persistence errors return
/// `Unavailable`, so the caller must not report a successful login on failure.
pub async fn establish(
    session: &Session,
    database: &DatabaseManager,
    issuer: &str,
    subject: &str,
) -> Result<(), AuthError> {
    let administrator = database
        .enabled_administrator(issuer, subject)
        .await?
        .ok_or(AuthError::Forbidden)?;
    // Rotate the session ID and discard pre-login data to prevent session fixation.
    session.clear().await;
    session.cycle_id().await?;
    // A prior unapproved login may have given this browser a shorter fixed expiry.
    session.set_expiry(Some(Expiry::OnInactivity(Duration::seconds(IDLE_SECONDS))));
    let now = chrono::Utc::now().timestamp();
    session
        .insert(
            ADMIN_KEY,
            AdminSession {
                id: administrator.id,
                generation: administrator.session_generation,
                created_at: now,
                last_seen: now,
                csrf: CsrfToken::new_random().secret().clone(),
            },
        )
        .await?;
    session.save().await?;
    Ok(())
}

/// Builds session middleware using the manager's SQLite store and cookie policy.
///
/// Production uses a Secure `__Host-` cookie with no Domain and a root path.
/// Explicit local development uses a separate name and permits plain HTTP.
/// Both use HttpOnly, SameSite=Lax, and a 30-minute idle timeout; guards separately
/// enforce the eight-hour absolute administrator lifetime.
pub fn session_layer(
    database: &DatabaseManager,
    development: bool,
) -> SessionManagerLayer<SqliteStore> {
    SessionManagerLayer::new(database.session_store())
        .with_name(if development {
            "maj_admin_dev"
        } else {
            "__Host-maj_admin"
        })
        .with_secure(!development)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(Duration::seconds(IDLE_SECONDS)))
}

/// Applies [`require_admin`] using the Leptos database context and Axum session.
/// Missing context or session extraction fails closed as `Unavailable`.
/// Server functions should map failures with [`AuthError::server_error`].
pub async fn require_server_admin() -> Result<Administrator, AuthError> {
    let database =
        leptos::prelude::use_context::<DatabaseManager>().ok_or(AuthError::Unavailable)?;
    let session = leptos_axum::extract::<Session>()
        .await
        .map_err(|_| AuthError::Unavailable)?;
    require_admin(&session, &database).await
}

/// Applies [`require_mutation`] to a Leptos server function's request headers.
/// Reads the trusted application origin from server configuration, never from the
/// incoming request. Missing configuration, request parts, or session is unavailable.
pub async fn require_server_mutation() -> Result<Administrator, AuthError> {
    let database =
        leptos::prelude::use_context::<DatabaseManager>().ok_or(AuthError::Unavailable)?;
    let config = leptos::prelude::use_context::<std::sync::Arc<ServerConfig>>()
        .ok_or(AuthError::Unavailable)?;
    let parts =
        leptos::prelude::use_context::<http::request::Parts>().ok_or(AuthError::Unavailable)?;
    let session = leptos_axum::extract::<Session>()
        .await
        .map_err(|_| AuthError::Unavailable)?;
    require_mutation(&session, &database, &parts.headers, &config.app_origin).await
}

/// Returns the current approved administrator for an authenticated request.
///
/// Rejects inconsistent timestamps, 30-minute idle expiry, and eight-hour absolute
/// expiry, flushing the invalid session. It then reloads approval and generation;
/// a missing/disabled identity or a generation mismatch also flushes the session.
/// Success refreshes idle activity without extending the absolute lifetime.
pub async fn require_admin(
    session: &Session,
    database: &DatabaseManager,
) -> Result<Administrator, AuthError> {
    let mut authenticated: AdminSession = session
        .get(ADMIN_KEY)
        .await?
        .ok_or(AuthError::Unauthorized)?;
    let now = chrono::Utc::now().timestamp();
    // Reject inconsistent timestamps before subtraction, then enforce both expiry limits.
    if authenticated.created_at < 0
        || authenticated.created_at > authenticated.last_seen
        || authenticated.last_seen > now
        || now - authenticated.created_at >= ABSOLUTE_SECONDS
        || now - authenticated.last_seen >= IDLE_SECONDS
    {
        session.flush().await?;
        return Err(AuthError::Unauthorized);
    }

    // Reload approval so revocation takes effect even for an already loaded session.
    let administrator = database.administrator(authenticated.id).await?;
    let administrator = match administrator {
        Some(admin) if admin.enabled && admin.session_generation == authenticated.generation => {
            admin
        }
        _ => {
            session.flush().await?;
            return Err(AuthError::Forbidden);
        }
    };

    // Refresh idle activity without extending the absolute session lifetime.
    authenticated.last_seen = now;
    session.insert(ADMIN_KEY, authenticated).await?;
    Ok(administrator)
}

/// Requires current approval, exact Origin, and a session-bound CSRF token.
///
/// Call before a state-changing operation, including logout. `origin` must come
/// from validated server configuration. Duplicate/missing headers are rejected;
/// the `x-csrf-token` value is compared with the stored token in constant time.
/// A failed mutation check never authorizes its caller to perform the operation.
pub async fn require_mutation(
    session: &Session,
    database: &DatabaseManager,
    headers: &HeaderMap,
    origin: &str,
) -> Result<Administrator, AuthError> {
    let administrator = require_admin(session, database).await?;
    require_csrf(session, headers, origin).await?;
    Ok(administrator)
}

/// Creates and persists a browser-bound login attempt before an authorization redirect.
///
/// Generates state, an S256 PKCE pair, and nonce. Supplies hashes of state and
/// browser binding to the manager; the verifier and nonce stay server-side for
/// callback validation. Records expire after five
/// minutes, with at most three per browser and 128 globally; a full store is `Busy`.
/// The browser session is saved before returning. If saving it fails, this
/// function attempts best-effort removal of the unusable transaction and returns `Unavailable`.
pub async fn begin_login(
    session: &Session,
    database: &DatabaseManager,
) -> Result<PendingLogin, AuthError> {
    // Reuse this browser's binding while giving every login attempt fresh protocol secrets.
    let binding = match session.get::<String>(BROWSER_KEY).await? {
        Some(value) => value,
        None => CsrfToken::new_random().secret().clone(),
    };
    let state = CsrfToken::new_random();
    let nonce = CsrfToken::new_random();
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    if !database
        .try_insert_login_transaction(
            &hash(state.secret()),
            &hash(&binding),
            verifier.secret(),
            nonce.secret(),
        )
        .await?
    {
        return Err(AuthError::Busy);
    }

    // Save browser state only after accepting the attempt, before any future redirect.
    session.insert(BROWSER_KEY, &binding).await?;
    if session.save().await.is_err() {
        // Remove abandoned protocol state when the matching browser session cannot be saved.
        let _ = database
            .delete_login_transaction(&hash(state.secret()))
            .await;
        return Err(AuthError::Unavailable);
    }
    Ok(PendingLogin {
        state,
        challenge,
        nonce: nonce.secret().clone().into(),
    })
}

/// Atomically consumes unexpired state belonging to this browser session.
///
/// Missing browser binding, overlong state, expiry, replay, and mismatched state or
/// browser return `Forbidden`. A successful delete returns the verifier and nonce
/// once; callbacks cannot retry the same state after later validation/exchange failure.
pub async fn consume_login(
    session: &Session,
    database: &DatabaseManager,
    state: &str,
) -> Result<LoginTransaction, AuthError> {
    if state.len() > 256 {
        return Err(AuthError::Forbidden);
    }
    let binding: String = session
        .get(BROWSER_KEY)
        .await?
        .ok_or(AuthError::Forbidden)?;
    let record = database
        .consume_login_transaction(&hash(state), &hash(&binding))
        .await?;
    let (verifier, nonce) = record.ok_or(AuthError::Forbidden)?;
    Ok(LoginTransaction {
        verifier: PkceCodeVerifier::new(verifier),
        nonce: nonce.into(),
    })
}

/// Guards `/admin` and `/api/admin` namespaces and marks private responses uncacheable.
///
/// GET/HEAD requests require current approval; other methods also require Origin
/// and CSRF checks. The native editor POST checks Origin here and its first multipart
/// CSRF field in the handler, before reading file data. `/auth` requests pass through for future login handlers, which
/// must perform their own protocol checks. All three namespaces receive no-store,
/// no-referrer, and Vary: Cookie headers, including errors and redirects. The
/// authenticated entry page uses same-origin referrers so native POST Origin is not null.
/// This middleware must run inside session middleware and the required extensions.
pub async fn private_routes(
    session: Session,
    axum::Extension(database): axum::Extension<DatabaseManager>,
    axum::Extension(config): axum::Extension<std::sync::Arc<ServerConfig>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path();

    let admin = path == "/admin"
        || path.starts_with("/admin/")
        || path == "/api/admin"
        || path.starts_with("/api/admin/");
    let private = admin || path == "/auth" || path.starts_with("/auth/");

    // The HTML entry page renders its own login prompt; API namespaces still fail with 401.
    let entry_page =
        path == "/admin" && matches!(*request.method(), http::Method::GET | http::Method::HEAD);
    let mut response = if admin && !entry_page {
        // Reads need approval; other methods additionally require Origin and CSRF checks.
        // The native editor checks its first multipart CSRF field before reading file data.
        let editor_form = path == "/admin/content" && *request.method() == http::Method::POST;
        let access = if editor_form {
            match require_origin(request.headers(), &config.app_origin) {
                Ok(()) => require_admin(&session, &database).await,
                Err(error) => Err(error),
            }
        } else if matches!(*request.method(), http::Method::GET | http::Method::HEAD) {
            require_admin(&session, &database).await
        } else {
            require_mutation(&session, &database, request.headers(), &config.app_origin).await
        };
        match access {
            Ok(_) => next.run(request).await,
            Err(error) => error.into_response(),
        }
    } else {
        next.run(request).await
    };

    // Apply privacy headers after downstream handling, including errors and redirects.
    if private {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, "no-store".parse().unwrap());
        // no-referrer makes native forms send Origin: null. Keep exact Origin checks
        // by allowing same-origin referrers only on the authenticated form page.
        let policy = if entry_page && response.status().is_success() {
            "same-origin"
        } else {
            "no-referrer"
        };
        response
            .headers_mut()
            .insert(REFERRER_POLICY, policy.parse().unwrap());
        response
            .headers_mut()
            .append(http::header::VARY, "Cookie".parse().unwrap());
    }
    response
}

// Private session data.

/// Private data under `ADMIN_KEY` in the server-side session, never in the cookie.
/// `last_seen` controls idle expiry; `created_at` fixes the absolute expiry even
/// when activity refreshes the session. Approval is checked against the database.
#[derive(Serialize, Deserialize)]
struct AdminSession {
    /// Local administrator ID used to reload the current approval record.
    id: i64,
    /// Approval generation captured at login; a mismatch means revocation.
    generation: i64,
    /// Login time in UTC Unix seconds, unchanged by later activity.
    created_at: i64,
    /// Last accepted request time in UTC Unix seconds, for idle-expiry checks.
    last_seen: i64,
    /// Random token required in the header of each authenticated mutation.
    csrf: String,
}

// Internal policy and helpers.

/// Checks mutation headers after the caller has established current administrator approval.
async fn require_csrf(
    session: &Session,
    headers: &HeaderMap,
    origin: &str,
) -> Result<(), AuthError> {
    // Require unambiguous headers before comparing the browser's session-bound token.
    require_origin(headers, origin)?;
    if headers.get_all("x-csrf-token").iter().count() != 1 {
        return Err(AuthError::Forbidden);
    }
    let supplied = headers
        .get("x-csrf-token")
        .ok_or(AuthError::Forbidden)?
        .as_bytes();
    let stored: AdminSession = session
        .get(ADMIN_KEY)
        .await?
        .ok_or(AuthError::Unauthorized)?;
    // A constant-time comparison avoids revealing a matching token prefix through timing.
    if !bool::from(supplied.ct_eq(stored.csrf.as_bytes())) {
        return Err(AuthError::Forbidden);
    }
    Ok(())
}

/// Reject cross-origin native forms before consuming any submitted content.
fn require_origin(headers: &HeaderMap, origin: &str) -> Result<(), AuthError> {
    if headers.get_all(ORIGIN).iter().count() != 1
        || headers.get(ORIGIN).and_then(|value| value.to_str().ok()) != Some(origin)
    {
        return Err(AuthError::Forbidden);
    }
    Ok(())
}

/// Hashes high-entropy protocol bindings for login-transaction database lookups.
fn hash(value: &str) -> String {
    crate::encode_hex(&Sha256::digest(value.as_bytes()))
}

// ###################################
// ->     TESTS
// ###################################
/// Constructs a session using the same persistent store without a test login bypass.
#[cfg(test)]
pub(crate) fn test_session(database: &DatabaseManager) -> Session {
    Session::new(
        None,
        std::sync::Arc::new(database.session_store()),
        Some(Expiry::OnInactivity(Duration::minutes(30))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[tokio::test]
    async fn native_axum_sessions_use_sqlite_and_environment_specific_cookie_policy() {
        use axum::{body::Body, routing::get, Router};
        use tower::ServiceExt;
        let (_root, database) = crate::test_support::database().await;
        for development in [false, true] {
            let app = Router::new()
                .route(
                    "/touch",
                    get(|session: Session| async move {
                        session.insert("test", 1).await.unwrap();
                        StatusCode::OK
                    }),
                )
                .layer(session_layer(&database, development));
            let response = app
                .oneshot(http::Request::get("/touch").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let cookie = tower_sessions::cookie::Cookie::parse(
                response.headers()[http::header::SET_COOKIE]
                    .to_str()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                cookie.name(),
                if development {
                    "maj_admin_dev"
                } else {
                    "__Host-maj_admin"
                }
            );
            assert_eq!(cookie.secure().unwrap_or(false), !development);
            assert_eq!(cookie.http_only(), Some(true));
            assert_eq!(cookie.same_site(), Some(SameSite::Lax));
            assert_eq!(cookie.path(), Some("/"));
            assert_eq!(cookie.domain(), None);
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM tower_sessions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            2
        );
        sqlx::query(
            "UPDATE tower_sessions SET
                 expiry_date = '2000-01-01 00:00:00'",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        database.cleanup_expired_auth().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM tower_sessions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            0
        );
        database.close().await;
    }

    #[tokio::test]
    async fn guards_deny_unknown_users_and_revoke_cached_sessions() {
        let (_root, database) = crate::test_support::database().await;
        let session = test_session(&database);
        let issuer = "https://accounts.google.com";
        assert!(matches!(
            require_admin(&session, &database).await,
            Err(AuthError::Unauthorized)
        ));
        assert!(matches!(
            establish(&session, &database, issuer, "123").await,
            Err(AuthError::Forbidden)
        ));
        database
            .grant_administrator(issuer, "123", "owner")
            .await
            .unwrap();
        establish(&session, &database, issuer, "123").await.unwrap();
        let old_id = session.id().unwrap();
        let old_csrf: AdminSession = session.get(ADMIN_KEY).await.unwrap().unwrap();
        assert_eq!(
            require_admin(&session, &database).await.unwrap().subject,
            "123"
        );
        establish(&session, &database, issuer, "123").await.unwrap();
        assert_ne!(session.id().unwrap(), old_id);
        assert_ne!(
            session
                .get::<AdminSession>(ADMIN_KEY)
                .await
                .unwrap()
                .unwrap()
                .csrf,
            old_csrf.csrf
        );
        database.revoke_administrator(issuer, "123").await.unwrap();
        database
            .grant_administrator(issuer, "123", "owner")
            .await
            .unwrap();
        assert!(matches!(
            require_admin(&session, &database).await,
            Err(AuthError::Forbidden)
        ));
        assert!(session
            .get::<AdminSession>(ADMIN_KEY)
            .await
            .unwrap()
            .is_none());
        database.close().await;
    }

    #[tokio::test]
    async fn idle_absolute_and_invalid_timestamps_expire_server_side() {
        let (_root, database) = crate::test_support::database().await;
        let issuer = "https://accounts.google.com";
        database
            .grant_administrator(issuer, "123", "owner")
            .await
            .unwrap();
        let session = test_session(&database);
        let now = chrono::Utc::now().timestamp();
        for (created, seen) in [
            (now - 3600, now - IDLE_SECONDS),
            (now - ABSOLUTE_SECONDS, now),
            (now + 60, now),
            (i64::MIN, now),
        ] {
            establish(&session, &database, issuer, "123").await.unwrap();
            let mut data = session
                .get::<AdminSession>(ADMIN_KEY)
                .await
                .unwrap()
                .unwrap();
            data.created_at = created;
            data.last_seen = seen;
            session.insert(ADMIN_KEY, data).await.unwrap();
            assert!(matches!(
                require_admin(&session, &database).await,
                Err(AuthError::Unauthorized)
            ));
        }
        database.close().await;
    }

    #[tokio::test]
    async fn mutation_requires_exact_origin_and_session_bound_csrf() {
        let (_root, database) = crate::test_support::database().await;
        let session = test_session(&database);
        database
            .grant_administrator("https://accounts.google.com", "123", "owner")
            .await
            .unwrap();
        establish(&session, &database, "https://accounts.google.com", "123")
            .await
            .unwrap();
        let token = session
            .get::<AdminSession>(ADMIN_KEY)
            .await
            .unwrap()
            .unwrap()
            .csrf;
        let origin = "http://localhost:8080";
        let mut headers = HeaderMap::new();
        assert!(matches!(
            require_mutation(&session, &database, &headers, origin).await,
            Err(AuthError::Forbidden)
        ));
        headers.insert(ORIGIN, origin.parse().unwrap());
        headers.insert("x-csrf-token", "wrong".parse().unwrap());
        assert!(matches!(
            require_mutation(&session, &database, &headers, origin).await,
            Err(AuthError::Forbidden)
        ));
        headers.insert("x-csrf-token", token.parse().unwrap());
        assert!(require_mutation(&session, &database, &headers, origin)
            .await
            .is_ok());
        for wrong in [
            "null",
            "https://evil.test",
            "http://localhost:8080/",
            "http://localhost:8080.evil.test",
        ] {
            headers.insert(ORIGIN, wrong.parse().unwrap());
            assert!(matches!(
                require_mutation(&session, &database, &headers, origin).await,
                Err(AuthError::Forbidden)
            ));
        }
        headers.insert(ORIGIN, origin.parse().unwrap());
        headers.append(ORIGIN, origin.parse().unwrap());
        assert!(matches!(
            require_mutation(&session, &database, &headers, origin).await,
            Err(AuthError::Forbidden)
        ));
        database.close().await;
    }

    #[tokio::test]
    async fn login_transactions_are_browser_bound_expiring_and_consumed_once() {
        let (_root, database) = crate::test_support::database().await;
        let session = test_session(&database);
        let other = test_session(&database);
        let pending = begin_login(&session, &database).await.unwrap();
        assert!(matches!(
            consume_login(&session, &database, &"x".repeat(257)).await,
            Err(AuthError::Forbidden)
        ));
        begin_login(&other, &database).await.unwrap();
        assert!(matches!(
            consume_login(&other, &database, pending.state.secret()).await,
            Err(AuthError::Forbidden)
        ));
        assert!(matches!(
            consume_login(&session, &database, "wrong").await,
            Err(AuthError::Forbidden)
        ));
        let (first, second) = tokio::join!(
            consume_login(&session, &database, pending.state.secret()),
            consume_login(&session, &database, pending.state.secret())
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        let record = first.or(second).unwrap();
        assert_eq!(record.nonce.expose_secret(), pending.nonce.expose_secret());
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(&record.verifier).as_str(),
            pending.challenge.as_str()
        );
        let expired = begin_login(&session, &database).await.unwrap();
        sqlx::query(
            "UPDATE login_transactions SET
                 expires_at = unixepoch() - 1",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        assert!(matches!(
            consume_login(&session, &database, expired.state.secret()).await,
            Err(AuthError::Forbidden)
        ));
        database.cleanup_expired_auth().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM login_transactions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            0
        );
        database.close().await;
    }

    #[tokio::test]
    async fn login_transaction_limits_do_not_create_sessions_when_full() {
        let (_root, database) = crate::test_support::database().await;
        let session = test_session(&database);
        for _ in 0..3 {
            begin_login(&session, &database).await.unwrap();
        }
        assert!(matches!(
            begin_login(&session, &database).await,
            Err(AuthError::Busy)
        ));
        sqlx::query(
            "WITH RECURSIVE n(x) AS (
                 SELECT 1
                 UNION ALL
                 SELECT x + 1
                 FROM n
                 WHERE x < 125
             )
             INSERT INTO login_transactions
             SELECT 'test-' || x, 'test', 'verifier', 'nonce', unixepoch() + 300
             FROM n",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        let new_browser = test_session(&database);
        assert!(matches!(
            begin_login(&new_browser, &database).await,
            Err(AuthError::Busy)
        ));
        assert!(new_browser.id().is_none());
        assert!(new_browser
            .get::<String>(BROWSER_KEY)
            .await
            .unwrap()
            .is_none());
        database.close().await;
    }

    #[tokio::test]
    async fn failed_session_persistence_removes_the_pending_login_transaction() {
        let (_root, database) = crate::test_support::database().await;
        sqlx::query(
            "CREATE TRIGGER reject_session
             BEFORE INSERT ON tower_sessions
             BEGIN
                 SELECT RAISE(ABORT, 'test session write failure');
             END",
        )
        .execute(database.test_pool())
        .await
        .unwrap();
        assert!(matches!(
            begin_login(&test_session(&database), &database).await,
            Err(AuthError::Unavailable)
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM login_transactions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)
                 FROM tower_sessions",
            )
            .fetch_one(database.test_pool())
            .await
            .unwrap(),
            0
        );
        database.close().await;
    }

    #[tokio::test]
    async fn database_errors_fail_closed_and_private_errors_do_not_leak_details() {
        let error = AuthError::from(sqlx::Error::Protocol("private backend diagnostic".into()));
        assert_eq!(error.to_string(), "Authentication request failed");
        assert!(std::error::Error::source(&error).is_none());
        let (_root, database) = crate::test_support::database().await;
        let session = test_session(&database);
        database
            .grant_administrator("https://accounts.google.com", "123", "owner")
            .await
            .unwrap();
        establish(&session, &database, "https://accounts.google.com", "123")
            .await
            .unwrap();
        database.close().await;
        assert!(matches!(
            require_admin(&session, &database).await,
            Err(AuthError::Unavailable)
        ));
        let response = AuthError::Unavailable.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[REFERRER_POLICY], "no-referrer");
    }
}
