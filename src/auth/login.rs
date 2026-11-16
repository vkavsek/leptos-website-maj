//! Server-rendered Google sign-in, approval guidance and administrator logout.
//!
//! `/auth/google/start` saves a browser-bound attempt before redirecting to Google.
//! The callback consumes that attempt once, verifies the provider identity, then
//! checks local approval before establishing an administrator session. An unapproved
//! account receives only a five-minute identity display for manual operator approval.
//!
//! Protocol/session checks live in the parent auth module; provider IO and token
//! verification live in `google`. These handlers coordinate them and render generic
//! failures. The server must install session middleware, database/config extensions
//! and `private_routes`, which adds privacy headers to errors and redirects as well.

use super::{
    google::{GoogleProvider, ISSUER},
    AdminSession, AuthError, ADMIN_KEY,
};
use crate::{database::DatabaseManager, server_config::ServerConfig};
use axum::{
    extract::{
        rejection::{FormRejection, QueryRejection},
        DefaultBodyLimit, Form, Query,
    },
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Router,
};
use http::{HeaderMap, HeaderValue, StatusCode};
use leptos::prelude::*;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::OnceCell;
use tower_sessions::Session;

const RECENT_IDENTITY: &str = "recent_google_identity";

/// Registers server-rendered auth pages with lazy provider initialization.
/// It has no startup network dependency and uses no browser-side secrets or scripts.
/// The returned router expects the server's session and private-route middleware.
pub(crate) fn routes(config: Arc<ServerConfig>) -> Router<leptos::config::LeptosOptions> {
    router(Arc::new(LoginService {
        config,
        provider: OnceCell::new(),
        attempts: Mutex::new(std::array::from_fn(|_| VecDeque::new())),
    }))
}

/// Maps internal failure categories to fixed messages without provider diagnostics or secrets.
pub(super) fn login_failure(error: AuthError) -> Response {
    let (status, message) = match error {
        AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Sign in to continue."),
        AuthError::Forbidden => (
            StatusCode::FORBIDDEN,
            "Sign-in could not be verified. Start again to retry.",
        ),
        AuthError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Google sign-in is temporarily unavailable. Please try again later.",
        ),
        AuthError::Busy => (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many sign-in attempts. Please wait a minute before trying again.",
        ),
    };
    page(
        status,
        "Sign-in",
        view! {
            <p>{message}</p>
            <a href="/admin">"Return to sign-in"</a>
        },
    )
}

/// Renders a script-free HTML response with a restrictive content security policy.
/// The surrounding `private_routes` middleware supplies cache/referrer privacy headers.
/// FIX: format
pub(super) fn page(status: StatusCode, title: &'static str, body: impl IntoView) -> Response {
    // Leptos escapes all dynamic text/attributes. No tokens or provider diagnostics enter scripts.
    let html = view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>{title}" - Maj Kavšek"</title>
                <style>
                    "body { font-family: system-ui, sans-serif; line-height: 1.5; margin: 1rem; } main { max-width: 45rem; margin-inline: auto; } code, li { overflow-wrap: anywhere; } button, input { font: inherit; min-height: 48px; padding: .5rem; box-sizing: border-box; max-width: 100%; } input:not([type=hidden]):not([type=checkbox]) { display: block; width: 100%; margin-block-end: 1rem; } input[type=checkbox] { min-height: auto; } label { display: inline-block; padding-block: .5rem; } form, section { margin-block: 1.5rem; } fieldset { min-width: 0; } a { display: inline-block; padding-block: .75rem; } img { max-width: 100%; height: auto; } :focus-visible { outline: 3px solid #1656b8; outline-offset: 3px; }"
                </style>
            </head>
            <body>
                <main>
                    <h1>{title}</h1>
                    {body}
                </main>
            </body>
        </html>
    }.to_html();
    (status, [(http::header::CONTENT_SECURITY_POLICY, "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'")], Html(html)).into_response()
}

/// Process-local provider and admission limits shared by every login request.
struct LoginService {
    config: Arc<ServerConfig>,
    provider: OnceCell<GoogleProvider>,
    /// Sliding windows: index 0 is login starts, index 1 is callbacks.
    attempts: Mutex<[VecDeque<Instant>; 2]>,
}

/// Untrusted callback fields; state is checked even when Google reports cancellation.
#[derive(Deserialize)]
struct Callback {
    state: String,
    code: Option<String>,
    error: Option<String>,
}

/// Native form alternative to the logout request's `x-csrf-token` header.
#[derive(Deserialize)]
struct Logout {
    csrf_token: String,
}

/// Short-lived server-side identity proof for operator approval, never an admin grant.
#[derive(Deserialize, Serialize)]
struct RecentIdentity {
    issuer: String,
    subject: String,
    verified_at: i64,
}

impl LoginService {
    /// Initializes the HTTP client on demand; missing credentials leave public pages usable.
    async fn provider(&self) -> Result<&GoogleProvider, AuthError> {
        if self.config.google.is_none() {
            return Err(AuthError::Unavailable);
        }
        self.provider
            .get_or_try_init(|| async { GoogleProvider::new() })
            .await
    }

    /// Admits at most 20 starts or 60 callbacks per rolling minute for this process.
    /// Accepted requests occupy a slot even if later validation fails. Restarting
    /// resets these windows; database transaction limits provide a separate bound.
    fn reserve(&self, callback: bool) -> Result<(), AuthError> {
        let now = Instant::now();
        let mut attempts = self.attempts.lock();
        let attempts = &mut attempts[usize::from(callback)];
        while attempts
            .front()
            .is_some_and(|at| now.duration_since(*at) >= Duration::from_secs(60))
        {
            attempts.pop_front();
        }
        // Global process limits supplement the persistent per-browser transaction limits.
        if attempts.len() >= if callback { 60 } else { 20 } {
            return Err(AuthError::Busy);
        }
        attempts.push_back(now);
        Ok(())
    }
}

/// Shares handler state and bounds extracted request bodies (not callback query strings).
fn router(service: Arc<LoginService>) -> Router<leptos::config::LeptosOptions> {
    Router::new()
        .route("/auth/google/start", get(start))
        .route("/auth/google/callback", get(callback))
        .route("/auth/identity", get(identity))
        .route("/auth/logout", post(logout))
        .route("/admin", get(admin))
        .layer(DefaultBodyLimit::max(2048))
        .layer(Extension(service))
        .merge(super::editor::routes())
}

/// Persists state, nonce and PKCE before exposing the authorization redirect.
/// Failed URL construction/timeout attempts to delete the abandoned transaction;
/// normal transaction expiry handles it if that best-effort deletion also fails.
async fn start(
    session: Session,
    Extension(database): Extension<DatabaseManager>,
    Extension(service): Extension<Arc<LoginService>>,
) -> Response {
    let result = async {
        service.reserve(false)?;
        let provider = service.provider().await?;
        session.remove::<RecentIdentity>(RECENT_IDENTITY).await?;
        let pending = super::begin_login(&session, &database).await?;
        let state_hash = super::hash(pending.state.secret());
        let redirect = tokio::time::timeout(
            Duration::from_secs(8),
            provider.authorization_url(&service.config, pending),
        )
        .await;
        match redirect {
            Ok(Ok(url)) => Ok(Redirect::to(&url).into_response()),
            _ => {
                let _ = database.delete_login_transaction(&state_hash).await;
                Err(AuthError::Unavailable)
            }
        }
    }
    .await;
    result.unwrap_or_else(login_failure)
}

/// Consumes the attempt before any provider exchange, so later failures require a new login.
/// Verified, enabled identities receive a rotated admin session; other verified
/// identities receive a separate short-lived record containing only approval details.
async fn callback(
    session: Session,
    Extension(database): Extension<DatabaseManager>,
    Extension(service): Extension<Arc<LoginService>>,
    query: Result<Query<Callback>, QueryRejection>,
) -> Response {
    let result = async {
        service.reserve(true)?;
        let Query(query) = query.map_err(|_| AuthError::Forbidden)?;
        // Consume state even on consent cancellation; no callback can replay a valid attempt.
        let transaction = super::consume_login(&session, &database, &query.state).await?;
        if let Some(error) = query.error {
            if query.code.is_some() {
                return Err(AuthError::Forbidden);
            }
            if error != "access_denied" {
                return Err(AuthError::Unavailable);
            }
            return Ok(page(
                StatusCode::BAD_REQUEST,
                "Sign-in cancelled",
                view! {
                    <p>"Google sign-in was cancelled or declined. You can try again."</p>
                    <a href="/auth/google/start">"Try Google sign-in again"</a>
                },
            ));
        }
        let code = query
            .code
            .filter(|code| !code.is_empty() && code.len() <= 4096)
            .ok_or(AuthError::Forbidden)?;
        let provider = service.provider().await?;
        let subject = tokio::time::timeout(
            Duration::from_secs(12),
            provider.exchange(&service.config, code, transaction),
        )
        .await
        .map_err(|_| AuthError::Unavailable)??;

        // Establish rechecks enabled approval and rotates IDs; verified strangers get no admin data.
        match super::establish(&session, &database, ISSUER, &subject).await {
            Ok(()) => {
                tracing::info!(
                    action = "google_login",
                    outcome = "approved",
                    "Administrator signed in"
                );
                Ok(Redirect::to("/admin").into_response())
            }
            Err(AuthError::Forbidden) => {
                // Discard any earlier privileges before saving the unapproved identity.
                session.clear().await;
                session.cycle_id().await?;
                // Expire the identity-bearing record itself, independently of page access.
                session.set_expiry(Some(tower_sessions::Expiry::AtDateTime(
                    tower_sessions::cookie::time::OffsetDateTime::now_utc()
                        + tower_sessions::cookie::time::Duration::minutes(5),
                )));
                session
                    .insert(
                        RECENT_IDENTITY,
                        RecentIdentity {
                            issuer: ISSUER.into(),
                            subject,
                            verified_at: chrono::Utc::now().timestamp(),
                        },
                    )
                    .await?;
                session.save().await?;
                Ok(Redirect::to("/auth/identity").into_response())
            }
            Err(error) => Err(error),
        }
    }
    .await;
    result.unwrap_or_else(login_failure)
}

/// Displays the verified issuer/subject only to the browser that just signed in.
/// Recheck the five-minute deadline on every view, independently of session expiry;
/// showing these details neither grants access nor creates an administrator record.
async fn identity(session: Session) -> Response {
    let result = async {
        let recent = session.get::<RecentIdentity>(RECENT_IDENTITY).await?.ok_or(AuthError::Unauthorized)?;
        let now = chrono::Utc::now().timestamp();
        if recent.verified_at < 0 || recent.verified_at > now || now - recent.verified_at >= 300 {
            session.remove::<RecentIdentity>(RECENT_IDENTITY).await?;
            return Err(AuthError::Unauthorized);
        }
        Ok(page(StatusCode::FORBIDDEN, "Account approval required", view! {
            <p>
                "Google verified this account, but it is not approved for administration. Share these values privately with the site operator, then sign in again after approval."
            </p>
            <dl>
                <dt>"Issuer"</dt>
                <dd>
                    <code>{recent.issuer}</code>
                </dd>
                <dt>"Subject"</dt>
                <dd>
                    <code>{recent.subject}</code>
                </dd>
            </dl>
            <p>"This identity display expires five minutes after verification."</p>
            <a href="/auth/google/start">"Sign in again"</a>
        }))
    }.await;
    result.unwrap_or_else(login_failure)
}

/// Requires current approval, the configured Origin and a session-bound CSRF token.
/// Flush the server-side session before redirecting so its cookie cannot be reused.
async fn logout(
    session: Session,
    Extension(database): Extension<DatabaseManager>,
    Extension(service): Extension<Arc<LoginService>>,
    mut headers: HeaderMap,
    form: Result<Form<Logout>, FormRejection>,
) -> Response {
    let result = async {
        let administrator = super::require_admin(&session, &database).await?;
        // Native HTML forms carry the same session-bound token in a hidden body field.
        // API callers can keep using the normal header; all callers need the exact Origin.
        if !headers.contains_key("x-csrf-token") {
            let Form(form) = form.map_err(|_| AuthError::Forbidden)?;
            headers.insert(
                "x-csrf-token",
                HeaderValue::from_str(&form.csrf_token).map_err(|_| AuthError::Forbidden)?,
            );
        }
        super::require_csrf(&session, &headers, &service.config.app_origin).await?;
        session.flush().await?;
        tracing::info!(
            action = "logout",
            actor = administrator.id,
            outcome = "success",
            "Administrator signed out"
        );
        Ok(Redirect::to("/admin").into_response())
    }
    .await;
    result.unwrap_or_else(login_failure)
}

/// Renders the protected landing page or a sign-in prompt with the denial status.
/// Approval is reloaded on each request; database/session failures render an error.
async fn admin(
    session: Session,
    Extension(database): Extension<DatabaseManager>,
    Extension(service): Extension<Arc<LoginService>>,
) -> Response {
    match super::require_admin(&session, &database).await {
        Ok(administrator) => {
            let data = match session.get::<AdminSession>(ADMIN_KEY).await {
                Ok(Some(data)) => data,
                _ => return login_failure(AuthError::Unavailable),
            };
            let editors = match super::editor::content(&database, &data.csrf).await {
                Ok(editors) => editors,
                Err(error) => return login_failure(error),
            };
            page(
                StatusCode::OK,
                "Administration",
                view! {
                    <p>"Signed in as "{administrator.label}"."</p>
                    {editors}
                    <form action="/auth/logout" method="post">
                        <input type="hidden" name="csrf_token" value=data.csrf />
                        <button type="submit">"Sign out"</button>
                    </form>
                    <p>
                        <a href="/">"Back to the portfolio"</a>
                    </p>
                },
            )
        }
        // Bind either denial variant as `error` to share the page while preserving 401/403.
        Err(error @ (AuthError::Unauthorized | AuthError::Forbidden)) => page(
            if matches!(error, AuthError::Forbidden) {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::UNAUTHORIZED
            },
            "Administration sign-in",
            view! {
                {if service.config.google.is_some() {
                    view! {
                        <p>
                            <a href="/auth/google/start">"Sign in with Google"</a>
                        </p>
                    }
                        .into_any()
                } else {
                    view! { <p>"Google sign-in is unavailable on this server."</p> }.into_any()
                }}
                <p>
                    <a href="/">"Back to the portfolio"</a>
                </p>
            },
        ),
        Err(error) => login_failure(error),
    }
}

// ###################################
// ->     TESTS
// ###################################
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::google::tests::{claims, config, token, MockGoogle};
    use axum::{
        body::{to_bytes, Body},
        middleware,
    };
    use http::{header::*, Request};
    use secrecy::ExposeSecret;
    use serde_json::json;
    use tower::ServiceExt;

    fn service(config: Arc<ServerConfig>, provider: GoogleProvider) -> Arc<LoginService> {
        let cell = OnceCell::new();
        assert!(cell.set(provider).is_ok());
        Arc::new(LoginService {
            config,
            provider: cell,
            attempts: Mutex::new(std::array::from_fn(|_| VecDeque::new())),
        })
    }

    fn test_app(database: &DatabaseManager, service: Arc<LoginService>) -> Router {
        let config = service.config.clone();
        router(service)
            .layer(middleware::from_fn(super::super::private_routes))
            .layer(Extension(config))
            .layer(Extension(database.clone()))
            .layer(super::super::session_layer(database, true))
            .with_state(
                leptos::config::LeptosOptions::builder()
                    .output_name("test")
                    .build(),
            )
    }

    fn cookie(session: &Session) -> String {
        format!("maj_admin_dev={}", session.id().unwrap())
    }
    fn response_cookie(response: &Response) -> String {
        response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .into()
    }
    fn private(response: &Response) {
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[REFERRER_POLICY], "no-referrer");
        assert!(response.headers()[VARY]
            .to_str()
            .unwrap()
            .contains("Cookie"));
    }
    async fn body(response: Response) -> String {
        String::from_utf8(
            to_bytes(response.into_body(), 65536)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn start_persists_bound_state_before_redirect_and_uses_identity_only_scope() {
        let (root, database) = crate::test_support::database().await;
        let mock = MockGoogle::new().await;
        let app = test_app(
            &database,
            service(Arc::new(config(root.path())), mock.provider()),
        );
        let response = app
            .oneshot(
                Request::get("/auth/google/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        private(&response);
        let browser_cookie = response_cookie(&response);
        let url = url::Url::parse(response.headers()[LOCATION].to_str().unwrap()).unwrap();
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["scope"], "openid");
        assert_eq!(query["code_challenge_method"], "S256");
        assert!(!query.contains_key("access_type"));
        assert!(!query.contains_key("client_secret"));
        let app = test_app(
            &database,
            service(Arc::new(config(root.path())), mock.provider()),
        );
        let response = app
            .oneshot(
                Request::get(format!(
                    "/auth/google/callback?state={}&error=access_denied",
                    query["state"]
                ))
                .header(COOKIE, browser_cookie)
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        private(&response);
        assert!(body(response).await.contains("cancelled"));
        assert!(mock.requests().is_empty());
        assert!(database
            .consume_login_transaction(&super::super::hash(&query["state"]), "wrong-browser")
            .await
            .unwrap()
            .is_none());
        database.close().await;
    }

    #[tokio::test]
    async fn approved_callback_rotates_session_and_logout_requires_origin_and_csrf() {
        let (root, database) = crate::test_support::database().await;
        database
            .grant_administrator(ISSUER, "test-user", "Owner <safe>")
            .await
            .unwrap();
        let session = super::super::test_session(&database);
        let pending = super::super::begin_login(&session, &database)
            .await
            .unwrap();
        let old_cookie = cookie(&session);
        let mock = MockGoogle::new().await;
        mock.reply(StatusCode::OK,json!({"access_token":"unused-test-token","token_type":"Bearer","id_token":token(&claims(pending.nonce.expose_secret()),"test-key")}));
        let app = test_app(
            &database,
            service(Arc::new(config(root.path())), mock.provider()),
        );
        let callback = format!(
            "/auth/google/callback?state={}&code=test-code",
            pending.state.secret()
        );
        let response = app
            .clone()
            .oneshot(
                Request::get(&callback)
                    .header(COOKIE, &old_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        private(&response);
        assert_eq!(response.headers()[LOCATION], "/admin");
        let new_cookie = response_cookie(&response);
        assert_ne!(new_cookie, old_cookie);
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/admin/not-a-page")
                    .header(COOKIE, &old_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .clone()
            .oneshot(
                Request::get("/admin")
                    .header(COOKIE, &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[REFERRER_POLICY], "same-origin");
        let html = body(response).await;
        assert!(html.contains("Owner &lt;safe&gt;"));
        let csrf = html
            .split("name=\"csrf_token\" value=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        for (origin, token) in [
            ("https://attacker.invalid", csrf),
            ("http://localhost:8080", "wrong"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/auth/logout")
                        .header(COOKIE, &new_cookie)
                        .header(ORIGIN, origin)
                        .header("x-csrf-token", token)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            private(&response);
        }
        let response = app
            .clone()
            .oneshot(
                Request::get("/auth/logout")
                    .header(COOKIE, &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        private(&response);
        let response = app
            .clone()
            .oneshot(
                Request::post("/auth/logout")
                    .header(COOKIE, &new_cookie)
                    .header(ORIGIN, "http://localhost:8080")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(format!("csrf_token={csrf}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        private(&response);
        assert!(response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0"));
        let response = app
            .clone()
            .oneshot(
                Request::get("/admin")
                    .header(COOKIE, &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .clone()
            .oneshot(
                Request::get(callback)
                    .header(COOKIE, old_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let session = super::super::test_session(&database);
        super::super::establish(&session, &database, ISSUER, "test-user")
            .await
            .unwrap();
        let current = session
            .get::<AdminSession>(ADMIN_KEY)
            .await
            .unwrap()
            .unwrap();
        let response = app
            .oneshot(
                Request::post("/auth/logout")
                    .header(COOKIE, cookie(&session))
                    .header(ORIGIN, "http://localhost:8080")
                    .header("x-csrf-token", current.csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(mock.requests().len(), 1);
        database.close().await;
    }

    #[tokio::test]
    async fn unapproved_identity_is_private_temporary_escaped_and_never_an_administrator() {
        let (root, database) = crate::test_support::database().await;
        let session = super::super::test_session(&database);
        let pending = super::super::begin_login(&session, &database)
            .await
            .unwrap();
        let mut values = claims(pending.nonce.expose_secret());
        values["sub"] = json!("<unapproved>");
        let mock = MockGoogle::new().await;
        mock.reply(StatusCode::OK,json!({"access_token":"unused-test-token","token_type":"Bearer","id_token":token(&values,"test-key")}));
        let app = test_app(
            &database,
            service(Arc::new(config(root.path())), mock.provider()),
        );
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/auth/google/callback?state={}&code=test-code",
                    pending.state.secret()
                ))
                .header(COOKIE, cookie(&session))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        private(&response);
        assert_eq!(response.headers()[LOCATION], "/auth/identity");
        let cookie_value =
            tower_sessions::cookie::Cookie::parse(response.headers()[SET_COOKIE].to_str().unwrap())
                .unwrap();
        assert!((1..=300).contains(&cookie_value.max_age().unwrap().whole_seconds()));
        let new_cookie = response_cookie(&response);
        let response = app
            .clone()
            .oneshot(
                Request::get("/auth/identity")
                    .header(COOKIE, &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        private(&response);
        let html = body(response).await;
        assert!(html.contains("&lt;unapproved&gt;"));
        assert!(!html.contains("unused-test-token"));
        assert!(database.administrators().await.unwrap().is_empty());
        for path in ["/admin", "/auth/identity"] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!body(response).await.contains("unapproved"));
        }
        let response = app
            .oneshot(
                Request::get("/api/admin/any")
                    .header(COOKIE, new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // Timestamp enforcement is independent of SQLite session-record cleanup.
        let recent = RecentIdentity {
            issuer: ISSUER.into(),
            subject: "expired-user".into(),
            verified_at: chrono::Utc::now().timestamp() - 300,
        };
        let session = super::super::test_session(&database);
        session.insert(RECENT_IDENTITY, recent).await.unwrap();
        session.save().await.unwrap();
        let response = identity(session).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        database.close().await;
    }

    #[tokio::test]
    async fn malformed_cancelled_and_failed_callbacks_never_exchange_unbound_codes() {
        let (root, database) = crate::test_support::database().await;
        let mock = MockGoogle::new().await;
        let config = Arc::new(config(root.path()));
        let svc = service(config.clone(), mock.provider());
        let app = test_app(&database, svc.clone());
        for query in [
            "code=secret-test-code",
            "state=wrong&code=secret-test-code",
            "state=x&state=y&code=secret-test-code",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("/auth/google/callback?{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            private(&response);
            let text = body(response).await;
            assert!(!text.contains("secret-test-code"));
        }
        assert!(mock.requests().is_empty());
        let session = super::super::test_session(&database);
        let pending = super::super::begin_login(&session, &database)
            .await
            .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/auth/google/callback?state={}&error=server_error",
                    pending.state.secret()
                ))
                .header(COOKIE, cookie(&session))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(mock.requests().is_empty());
        let session = super::super::test_session(&database);
        let pending = super::super::begin_login(&session, &database)
            .await
            .unwrap();
        mock.reply(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error_description":"private provider diagnostic"}),
        );
        let response = app
            .oneshot(
                Request::get(format!(
                    "/auth/google/callback?state={}&code=secret-test-code",
                    pending.state.secret()
                ))
                .header(COOKIE, cookie(&session))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        private(&response);
        let html = body(response).await;
        assert!(!html.contains("private provider diagnostic"));
        assert!(matches!(
            super::super::consume_login(&session, &database, pending.state.secret()).await,
            Err(AuthError::Forbidden)
        ));
        database.close().await;
    }

    #[tokio::test]
    async fn provider_outage_cleans_pending_state_and_returns_bounded_retry_errors() {
        let (root, database) = crate::test_support::database().await;
        let svc = service(
            Arc::new(config(root.path())),
            crate::auth::google::tests::unavailable_provider(),
        );
        let app = test_app(&database, svc.clone());
        let response = app
            .clone()
            .oneshot(
                Request::get("/auth/google/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        private(&response);
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM login_transactions")
            .fetch_one(database.test_pool())
            .await
            .unwrap();
        assert_eq!(pending, 0);
        for _ in 1..20 {
            svc.reserve(false).unwrap();
        }
        let response = app
            .clone()
            .oneshot(
                Request::get("/auth/google/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        private(&response);
        for _ in 0..60 {
            svc.reserve(true).unwrap();
        }
        let response = app
            .oneshot(
                Request::get("/auth/google/callback?state=fake&code=fake")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        private(&response);
        database.close().await;
    }

    #[test]
    fn process_limits_are_bounded_and_recover_after_the_window() {
        let root = crate::test_support::directory();
        let service = LoginService {
            config: Arc::new(config(root.path())),
            provider: OnceCell::new(),
            attempts: Mutex::new(std::array::from_fn(|_| VecDeque::new())),
        };
        for (callback, limit) in [(false, 20), (true, 60)] {
            for _ in 0..limit {
                service.reserve(callback).unwrap();
            }
            assert!(matches!(service.reserve(callback), Err(AuthError::Busy)));
            let mut attempts = service.attempts.lock();
            for at in &mut attempts[usize::from(callback)] {
                *at = Instant::now() - Duration::from_secs(61);
            }
            drop(attempts);
            assert!(service.reserve(callback).is_ok());
        }
    }
}
