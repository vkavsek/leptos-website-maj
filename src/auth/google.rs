//! Google identity verification and bounded provider requests.
//!
//! The login handlers persist browser-bound state before asking this module for an
//! authorization URL. On callback, they consume that state and pass its PKCE
//! verifier and nonce to [`GoogleProvider::exchange`]. A successful exchange returns
//! the verified Google subject; local administrator approval remains the caller's job.
//!
//! `oauth2` builds the authorization/code-exchange requests; `openidconnect` verifies
//! the ID token. Provider URLs are pinned, redirects are disabled, and response
//! bodies/timeouts are bounded. Only public discovery metadata and signing keys are
//! cached. Provider errors are reduced to [`AuthError`] without exposing token bodies.

use super::{AuthError, LoginTransaction, PendingLogin};
use crate::server_config::ServerConfig;
use chrono::Utc;
use http::{header::CACHE_CONTROL, HeaderValue};
use http_cache_semantics::{CacheOptions, CachePolicy};
use jsonwebtoken::{
    decode_header,
    jwk::{AlgorithmParameters, JwkSet, KeyAlgorithm, KeyOperations, PublicKeyUse},
    Algorithm,
};
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    AuthType, AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, EndpointNotSet,
    EndpointSet, ExtraTokenFields, RedirectUrl, Scope, StandardRevocableToken,
    StandardTokenResponse, TokenUrl,
};
use openidconnect::{
    core::{
        CoreGenderClaim, CoreIdTokenVerifier, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    },
    AdditionalClaims, IdToken, IssuerUrl, JsonWebKeySet, Nonce,
};
use secrecy::ExposeSecret;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::Mutex;

pub(super) const ISSUER: &str = "https://accounts.google.com";
const DISCOVERY: &str = "https://accounts.google.com/.well-known/openid-configuration";
const AUTHORIZATION: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const JWKS: &str = "https://www.googleapis.com/oauth2/v3/certs";
const RESPONSE_BYTES: usize = 64 * 1024;

/// A shared provider contains public cached metadata/keys, never account tokens.
/// Construction does not contact Google; failed lookups back off without blocking startup.
pub(super) struct GoogleProvider {
    http: reqwest::Client,
    // Fixed in production; tests replace it with an isolated local discovery endpoint.
    discovery_url: String,
    cache: Mutex<Cache>,
}

impl GoogleProvider {
    /// Builds the shared HTTP client without network IO.
    /// Each request has a three-second connection and five-second total timeout;
    /// login handlers additionally bound the complete authorization/exchange operation.
    pub(super) fn new() -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| AuthError::Unavailable)?;
        Ok(Self {
            http,
            discovery_url: DISCOVERY.into(),
            cache: Mutex::new(Cache::default()),
        })
    }

    /// Builds an identity-only redirect using an already persisted login attempt.
    /// State binds the callback to the browser, PKCE binds the code exchange to the
    /// server-held verifier, and nonce binds the returned ID token to this attempt.
    pub(super) async fn authorization_url(
        &self,
        config: &ServerConfig,
        pending: PendingLogin,
    ) -> Result<String, AuthError> {
        let metadata = self.metadata().await?;
        let (url, _) = client(config, &metadata)?
            .authorize_url(|| pending.state)
            // Only identity is needed: no profile/email scopes, offline access, or refresh tokens.
            .add_scope(Scope::new("openid".into()))
            .set_pkce_challenge(pending.challenge)
            .add_extra_param("nonce", pending.nonce.expose_secret())
            .url();
        Ok(url.into())
    }

    /// Exchanges a one-use code and returns its verified `sub` account identifier.
    /// The caller must first consume the matching browser-bound transaction.
    /// Tokens are used only within this call and are never placed in the cache.
    /// Invalid grants/identity proofs return `Forbidden`; provider or configuration
    /// failures return `Unavailable`. Success alone does not authorize administration.
    pub(super) async fn exchange(
        &self,
        config: &ServerConfig,
        code: String,
        transaction: LoginTransaction,
    ) -> Result<String, AuthError> {
        let metadata = self.metadata().await?;
        let http = self.http.clone();
        let token = client(config, &metadata)?
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(transaction.verifier)
            .request_async(&move |request: oauth2::HttpRequest| {
                token_request(http.clone(), request)
            })
            .await
            .map_err(|error| match error {
                oauth2::RequestTokenError::ServerResponse(response)
                    if response.error() == &oauth2::basic::BasicErrorResponseType::InvalidGrant =>
                {
                    AuthError::Forbidden
                }
                _ => AuthError::Unavailable,
            })?;

        let id_token = &token.extra_fields().id_token;
        // Header fields only select a fixed provider key; they never choose URLs or algorithms.
        if id_token.len() > 16384 {
            return Err(AuthError::Forbidden);
        }

        let header = decode_header(id_token).map_err(|_| AuthError::Forbidden)?;
        if header.alg != Algorithm::RS256 {
            return Err(AuthError::Forbidden);
        }
        let kid = header
            .kid
            .filter(|kid| !kid.is_empty() && kid.len() <= 256)
            .ok_or(AuthError::Forbidden)?;

        let keys = self.keys(&metadata, &kid).await?;
        verify(
            id_token,
            &keys,
            &kid,
            config,
            transaction.nonce.expose_secret(),
        )
    }

    /// Reuses fresh discovery metadata or fetches and checks every required endpoint.
    /// Failed or unexpected discovery responses trigger the shared 30-second backoff.
    async fn metadata(&self) -> Result<Metadata, AuthError> {
        // Hold the async mutex through refresh so concurrent logins share one provider fetch.
        let mut cache = self.cache.lock().await;
        if cache.retry_at.is_some_and(|at| at > Instant::now()) {
            return Err(AuthError::Unavailable);
        }
        if let Some(cached) = &cache.metadata {
            if cached.expires_at > Instant::now() {
                return Ok(cached.value.clone());
            }
        }
        // Pin discovery and every endpoint to Google; untrusted inputs cannot redirect provider IO.
        let result = self.document::<Metadata>(&self.discovery_url).await;
        match result {
            Ok(cached)
                if cached.value.issuer == ISSUER
                    && cached.value.authorization_endpoint == AUTHORIZATION
                    && cached.value.token_endpoint == TOKEN
                    && cached.value.jwks_uri == JWKS =>
            {
                let metadata = cached.value.clone();
                cache.metadata = (cached.expires_at > Instant::now()).then_some(cached);
                Ok(metadata)
            }
            _ => {
                cache.retry_at = Some(Instant::now() + Duration::from_secs(30));
                Err(AuthError::Unavailable)
            }
        }
    }

    /// Returns fresh keys, refreshing early for an unknown key ID to allow rotation.
    /// Unknown-ID refreshes of a fresh cache are limited to once per minute; expired
    /// or absent keys still require a fetch. Stale keys are never a failure fallback.
    /// A missing ID after refresh is rejected by `verify`, without another fetch.
    async fn keys(&self, metadata: &Metadata, kid: &str) -> Result<JwkSet, AuthError> {
        let mut cache = self.cache.lock().await;
        let now = Instant::now();
        if cache.retry_at.is_some_and(|at| at > now) {
            return Err(AuthError::Unavailable);
        }
        let expired = cache
            .keys
            .as_ref()
            .is_none_or(|keys| keys.expires_at <= now);
        let missing = cache
            .keys
            .as_ref()
            .is_some_and(|keys| keys.value.find(kid).is_none());
        // Refresh an unknown kid for rotation, at most once a minute across all browsers.
        if expired || (missing && cache.rotation_retry_at.is_none_or(|at| at <= now)) {
            cache.rotation_retry_at = Some(now + Duration::from_secs(60));
            match self.document::<JwkSet>(&metadata.jwks_uri).await {
                Ok(keys) if !keys.value.keys.is_empty() && keys.value.keys.len() <= 32 => {
                    // A non-cacheable response can satisfy this lookup without being retained.
                    let value = keys.value.clone();
                    cache.keys = (keys.expires_at > Instant::now()).then_some(keys);
                    return Ok(value);
                }
                _ => {
                    cache.retry_at = Some(Instant::now() + Duration::from_secs(30));
                    return Err(AuthError::Unavailable);
                }
            }
        }
        cache
            .keys
            .as_ref()
            .map(|keys| keys.value.clone())
            .ok_or(AuthError::Unavailable)
    }

    /// Fetches a bounded JSON document and records its HTTP freshness deadline.
    /// An immediately stale document can serve this call; callers decide whether
    /// to retain it after checking its contents and deadline.
    async fn document<T: DeserializeOwned + Send>(
        &self,
        url: &str,
    ) -> Result<Cached<T>, AuthError> {
        let request = self
            .http
            .get(url)
            .build()
            .map_err(|_| AuthError::Unavailable)?;
        let mut response = self
            .http
            .execute(request.try_clone().ok_or(AuthError::Unavailable)?)
            .await
            .map_err(|_| AuthError::Unavailable)?;
        if !response.status().is_success() {
            return Err(AuthError::Unavailable);
        }
        let lifetime = cache_lifetime(&request, &mut response, SystemTime::now());
        let bytes = bounded_body(response).await?;
        let value = serde_json::from_slice(&bytes).map_err(|_| AuthError::Unavailable)?;
        Ok(Cached {
            value,
            expires_at: Instant::now() + lifetime,
        })
    }
}

/// Only the discovery fields needed by this application; all must match pinned URLs.
#[derive(Clone, Deserialize)]
struct Metadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

/// A decoded public document with a monotonic deadline for local reuse.
struct Cached<T> {
    value: T,
    expires_at: Instant,
}

/// Shared across browsers within one server process; never contains account tokens.
#[derive(Default)]
struct Cache {
    metadata: Option<Cached<Metadata>>,
    keys: Option<Cached<JwkSet>>,
    /// Provider-failure backoff shared by discovery and key requests.
    retry_at: Option<Instant>,
    /// Earliest extra refresh for an unknown key ID while cached keys are fresh.
    rotation_retry_at: Option<Instant>,
}

/// Retains the ID token from the OAuth response while keeping it out of Debug output.
#[derive(Deserialize, Serialize)]
struct IdTokenFields {
    id_token: String,
}

impl fmt::Debug for IdTokenFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IdTokenFields([REDACTED])")
    }
}
impl ExtraTokenFields for IdTokenFields {}

/// The optional JWT not-before claim supplements the library's OIDC standard claims.
#[derive(Debug, Deserialize, Serialize)]
struct TokenClaims {
    nbf: Option<i64>,
}

impl AdditionalClaims for TokenClaims {}

/// OAuth client with authorization/token endpoints set and unused endpoints unset.
type GoogleClient = Client<
    BasicErrorResponse,
    StandardTokenResponse<IdTokenFields, BasicTokenType>,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

/// Typed OIDC claims plus `nbf`; verification below accepts only RS256-signed tokens.
type GoogleIdToken = IdToken<
    TokenClaims,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;

/// Builds requests from validated configuration and pinned discovery metadata.
/// The callback comes from the configured application origin, never request headers.
fn client(config: &ServerConfig, metadata: &Metadata) -> Result<GoogleClient, AuthError> {
    let google = config.google.as_ref().ok_or(AuthError::Unavailable)?;
    Ok(Client::new(ClientId::new(google.client_id.clone()))
        .set_client_secret(ClientSecret::new(
            google.client_secret.expose_secret().into(),
        ))
        .set_auth_type(AuthType::RequestBody)
        .set_auth_uri(
            AuthUrl::new(metadata.authorization_endpoint.clone())
                .map_err(|_| AuthError::Unavailable)?,
        )
        .set_token_uri(
            TokenUrl::new(metadata.token_endpoint.clone()).map_err(|_| AuthError::Unavailable)?,
        )
        .set_redirect_uri(
            RedirectUrl::new(config.google_callback()).map_err(|_| AuthError::Unavailable)?,
        ))
}

/// Verifies the signature and identity claims against the selected Google public key.
/// The OIDC library checks issuer, audience, expiry and nonce. Application policy
/// additionally restricts key usage, authorized party (`azp`), timestamps and subject.
/// Issuance allows 60 seconds of clock skew; expiry and optional `nbf` allow none.
fn verify(
    token: &str,
    keys: &JwkSet,
    kid: &str,
    config: &ServerConfig,
    nonce: &str,
) -> Result<String, AuthError> {
    let google = config.google.as_ref().ok_or(AuthError::Unavailable)?;
    let key = keys.find(kid).ok_or(AuthError::Forbidden)?;
    if !matches!(key.algorithm, AlgorithmParameters::RSA(_))
        || key
            .common
            .key_algorithm
            .is_some_and(|alg| alg != KeyAlgorithm::RS256)
        || key
            .common
            .public_key_use
            .as_ref()
            .is_some_and(|usage| usage != &PublicKeyUse::Signature)
        || key
            .common
            .key_operations
            .as_ref()
            .is_some_and(|ops| !ops.contains(&KeyOperations::Verify))
    {
        return Err(AuthError::Forbidden);
    }
    let key: CoreJsonWebKey =
        serde_json::from_value(serde_json::to_value(key).map_err(|_| AuthError::Forbidden)?)
            .map_err(|_| AuthError::Forbidden)?;
    // RSA is used only for public-key verification; no private key or shared secret is supplied.
    let verifier = CoreIdTokenVerifier::new_public_client(
        ClientId::new(google.client_id.clone()),
        IssuerUrl::new(ISSUER.into()).map_err(|_| AuthError::Unavailable)?,
        JsonWebKeySet::new(vec![key]),
    )
    .set_allowed_algs([CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256])
    // Additional audiences require the matching authorized party checked below.
    .set_other_audience_verifier_fn(|_| true);
    let token: GoogleIdToken = token.parse().map_err(|_| AuthError::Forbidden)?;
    let claims = token
        .claims(&verifier, &Nonce::new(nonce.into()))
        .map_err(|_| AuthError::Forbidden)?;

    // Keep application policy beyond the library's signature, issuer, audience, expiry and nonce checks.
    let now = Utc::now().timestamp();
    let issued_at = claims.issue_time().timestamp();
    let expires_at = claims.expiration().timestamp();
    let subject = claims.subject().as_str();
    let authorized_party = claims.authorized_party().map(|party| party.as_str());
    if (claims.audiences().len() > 1 && authorized_party != Some(&google.client_id))
        || authorized_party.is_some_and(|party| party != google.client_id)
        || expires_at <= now
        || issued_at < 0
        || issued_at > now.saturating_add(60)
        || expires_at <= issued_at
        || claims
            .additional_claims()
            .nbf
            .is_some_and(|nbf| nbf < 0 || nbf > now)
        || subject.is_empty()
        || subject.len() > 255
        || !subject.bytes().all(|c| c.is_ascii_graphic())
    {
        return Err(AuthError::Forbidden);
    }
    Ok(subject.into())
}

/// Adapts the OAuth transport to our no-redirect, timed and size-bounded HTTP client.
/// Preserve status/headers so `oauth2` can classify protocol errors such as invalid_grant.
async fn token_request(
    http: reqwest::Client,
    request: oauth2::HttpRequest,
) -> Result<oauth2::HttpResponse, AuthError> {
    let (parts, body) = request.into_parts();
    let response = http
        .request(parts.method, parts.uri.to_string())
        .headers(parts.headers)
        .body(body)
        .send()
        .await
        .map_err(|_| AuthError::Unavailable)?;
    let mut result = http::Response::builder().status(response.status());
    *result.headers_mut().ok_or(AuthError::Unavailable)? = response.headers().clone();
    result
        .body(bounded_body(response).await?)
        .map_err(|_| AuthError::Unavailable)
}

/// Reads at most 64 KiB, rejecting oversized bodies even without Content-Length.
async fn bounded_body(mut response: reqwest::Response) -> Result<Vec<u8>, AuthError> {
    if response
        .content_length()
        .is_some_and(|size| size > RESPONSE_BYTES as u64)
    {
        return Err(AuthError::Unavailable);
    }
    let mut bytes = Vec::new();
    // Bound streamed bodies as well as Content-Length, including chunked provider responses.
    while let Some(chunk) = response.chunk().await.map_err(|_| AuthError::Unavailable)? {
        if bytes.len() + chunk.len() > RESPONSE_BYTES {
            return Err(AuthError::Unavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Computes shared-cache freshness, capped at one hour including upstream age.
/// No heuristic or immutable default extends a response without explicit freshness.
/// Zero means use once without retention; expired entries are fetched again rather
/// than conditionally revalidated. Header normalization is local to provider responses.
fn cache_lifetime(
    request: &reqwest::Request,
    response: &mut reqwest::Response,
    now: SystemTime,
) -> Duration {
    // Cache directives are case-insensitive; the policy parser expects lowercase names.
    for (name, value) in response.headers_mut() {
        if name == CACHE_CONTROL || name == http::header::PRAGMA {
            *value = HeaderValue::from_bytes(&value.as_bytes().to_ascii_lowercase())
                .unwrap_or_else(|_| HeaderValue::from_static("no-store"));
        }
    }
    let policy = CachePolicy::new_options(
        request,
        response,
        now,
        CacheOptions {
            cache_heuristic: 0.0,
            immutable_min_time_to_live: Duration::ZERO,
            ..CacheOptions::default()
        },
    );
    if !policy.is_storable() {
        return Duration::ZERO;
    }
    // Upstream cache age also counts against our one-hour freshness cap.
    let cap = Duration::from_secs(3600).saturating_sub(policy.age(now));
    policy.time_to_live(now).min(cap)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::server_config::GoogleCredentials;
    use axum::{
        body::{to_bytes, Body},
        extract::State,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use http::{Request, StatusCode};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::{json, Value};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    // Public test material only: this key is committed solely to sign isolated fixture tokens.
    pub(crate) fn signing_key() -> EncodingKey {
        EncodingKey::from_rsa_pem(include_bytes!(
            "../../tests/fixtures/oidc/test-only-rsa.pem"
        ))
        .unwrap()
    }

    pub(crate) fn config(root: &std::path::Path) -> ServerConfig {
        let mut config = crate::test_support::config(root);
        config.google = Some(GoogleCredentials {
            client_id: "test-client.apps.googleusercontent.com".into(),
            client_secret: "test-only-secret".into(),
        });
        config
    }

    pub(crate) fn claims(nonce: &str) -> Value {
        let now = jsonwebtoken::get_current_timestamp();
        json!({"iss": ISSUER, "sub": "test-user", "aud": "test-client.apps.googleusercontent.com", "iat": now, "exp": now+300, "nonce": nonce})
    }

    pub(crate) fn token(claims: &Value, kid: &str) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.into());
        encode(&header, claims, &signing_key()).unwrap()
    }

    pub(crate) fn unavailable_provider() -> GoogleProvider {
        let provider = GoogleProvider::new().unwrap();
        provider.cache.try_lock().unwrap().retry_at =
            Some(Instant::now() + Duration::from_secs(30));
        provider
    }

    fn key_set() -> JwkSet {
        let mut key =
            jsonwebtoken::jwk::Jwk::from_encoding_key(&signing_key(), Algorithm::RS256).unwrap();
        key.common.key_id = Some("test-key".into());
        JwkSet { keys: vec![key] }
    }

    pub(crate) struct MockGoogle {
        url: String,
        state: Arc<MockState>,
        task: tokio::task::JoinHandle<()>,
    }

    struct MockState {
        reply: parking_lot::Mutex<(StatusCode, Value)>,
        requests: parking_lot::Mutex<Vec<String>>,
        keys: AtomicUsize,
        discovery: AtomicUsize,
        discovery_reply: parking_lot::Mutex<Value>,
        cache_control: parking_lot::Mutex<String>,
    }

    impl MockGoogle {
        pub(crate) async fn new() -> Self {
            let state = Arc::new(MockState {
                reply: parking_lot::Mutex::new((StatusCode::OK, json!({}))),
                requests: parking_lot::Mutex::new(Vec::new()),
                keys: AtomicUsize::new(0),
                discovery: AtomicUsize::new(0),
                discovery_reply: parking_lot::Mutex::new(
                    json!({"issuer": ISSUER, "authorization_endpoint": AUTHORIZATION, "token_endpoint": TOKEN, "jwks_uri": JWKS}),
                ),
                cache_control: parking_lot::Mutex::new("public, max-age=3600".into()),
            });
            let app = Router::new()
                .route("/token", post(mock_token))
                .route("/keys", get(mock_keys))
                .route("/discovery", get(mock_discovery))
                .route(
                    "/redirect",
                    get(|| async { axum::response::Redirect::temporary("/keys") }),
                )
                .route(
                    "/oversized",
                    get(|| async { "x".repeat(RESPONSE_BYTES + 1) }),
                )
                .route(
                    "/oversized-chunked",
                    get(|| async {
                        // A streaming body makes Hyper choose chunked encoding without a length.
                        let body = Body::from("x".repeat(RESPONSE_BYTES + 1));
                        Body::from_stream(body.into_data_stream())
                    }),
                )
                .with_state(state.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            Self { url, state, task }
        }

        pub(crate) fn reply(&self, status: StatusCode, reply: Value) {
            *self.state.reply.lock() = (status, reply);
        }
        pub(crate) fn requests(&self) -> Vec<String> {
            self.state.requests.lock().clone()
        }

        pub(crate) fn provider(&self) -> GoogleProvider {
            let provider = GoogleProvider::new().unwrap();
            let metadata = Metadata {
                issuer: ISSUER.into(),
                authorization_endpoint: AUTHORIZATION.into(),
                token_endpoint: format!("{}/token", self.url),
                jwks_uri: format!("{}/keys", self.url),
            };
            // Test-only injection: production always validates the fixed discovery/endpoint URLs.
            *provider.cache.try_lock().unwrap() = Cache {
                metadata: Some(Cached {
                    value: metadata,
                    expires_at: Instant::now() + Duration::from_secs(3600),
                }),
                ..Cache::default()
            };
            provider
        }
    }
    impl Drop for MockGoogle {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn mock_token(
        State(state): State<Arc<MockState>>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        let bytes = to_bytes(request.into_body(), RESPONSE_BYTES).await.unwrap();
        state
            .requests
            .lock()
            .push(String::from_utf8(bytes.to_vec()).unwrap());
        let (status, value) = state.reply.lock().clone();
        (status, Json(value))
    }
    async fn mock_discovery(State(state): State<Arc<MockState>>) -> impl IntoResponse {
        state.discovery.fetch_add(1, Ordering::Relaxed);
        (
            [(CACHE_CONTROL, state.cache_control.lock().clone())],
            Json(state.discovery_reply.lock().clone()),
        )
    }

    async fn mock_keys(State(state): State<Arc<MockState>>) -> impl IntoResponse {
        state.keys.fetch_add(1, Ordering::Relaxed);
        (
            [(CACHE_CONTROL, state.cache_control.lock().clone())],
            Json(key_set()),
        )
    }

    #[test]
    fn signed_tokens_require_all_google_identity_checks() {
        let root = crate::test_support::directory();
        let config = config(root.path());
        let keys = key_set();
        let original = claims("expected-nonce");
        assert_eq!(
            verify(
                &token(&original, "test-key"),
                &keys,
                "test-key",
                &config,
                "expected-nonce"
            )
            .unwrap(),
            "test-user"
        );
        // Every mutation is signed correctly: failure must come from claim validation.
        for (field, value) in [
            ("iss", json!("https://attacker.invalid")),
            ("iss", json!("https://accounts.google.com/")),
            ("aud", json!("other-client")),
            ("exp", json!(1)),
            ("exp", original["iat"].clone()),
            ("iat", json!(-1)),
            ("iat", json!(jsonwebtoken::get_current_timestamp() + 120)),
            ("nonce", json!("wrong")),
            ("sub", json!("")),
            ("sub", json!("has spaces")),
            ("sub", json!("x".repeat(256))),
            ("azp", json!("other-client")),
            (
                "aud",
                json!(["test-client.apps.googleusercontent.com", "other-client"]),
            ),
            ("nbf", json!(jsonwebtoken::get_current_timestamp() + 300)),
            ("nbf", json!("invalid")),
            ("nbf", json!(-1)),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            assert!(
                matches!(
                    verify(
                        &token(&changed, "test-key"),
                        &keys,
                        "test-key",
                        &config,
                        "expected-nonce"
                    ),
                    Err(AuthError::Forbidden)
                ),
                "{field}"
            );
        }
        for field in ["iss", "sub", "aud", "exp", "iat", "nonce"] {
            let mut changed = original.clone();
            changed.as_object_mut().unwrap().remove(field);
            assert!(
                verify(
                    &token(&changed, "test-key"),
                    &keys,
                    "test-key",
                    &config,
                    "expected-nonce"
                )
                .is_err(),
                "{field}"
            );
        }
        let mut multiple = original.clone();
        multiple["aud"] = json!(["test-client.apps.googleusercontent.com", "other-client"]);
        multiple["azp"] = json!("test-client.apps.googleusercontent.com");
        assert!(verify(
            &token(&multiple, "test-key"),
            &keys,
            "test-key",
            &config,
            "expected-nonce"
        )
        .is_ok());
        let mut not_before = original.clone();
        not_before["nbf"] = json!(jsonwebtoken::get_current_timestamp() - 1);
        assert!(verify(
            &token(&not_before, "test-key"),
            &keys,
            "test-key",
            &config,
            "expected-nonce"
        )
        .is_ok());
        let mut damaged = token(&original, "test-key").into_bytes();
        let index = damaged.iter().rposition(|b| *b == b'.').unwrap() + 2;
        damaged[index] = if damaged[index] == b'A' { b'B' } else { b'A' };
        assert!(verify(
            std::str::from_utf8(&damaged).unwrap(),
            &keys,
            "test-key",
            &config,
            "expected-nonce"
        )
        .is_err());
        let hs_token = encode(
            &Header::new(Algorithm::HS256),
            &original,
            &EncodingKey::from_secret(b"test-only-hmac").unwrap(),
        )
        .unwrap();
        assert!(verify(&hs_token, &keys, "test-key", &config, "expected-nonce").is_err());
        assert!(verify(
            &token(&original, "unknown"),
            &keys,
            "unknown",
            &config,
            "expected-nonce"
        )
        .is_err());
        for incompatible in ["alg", "use", "key_ops", "n"] {
            let mut keys = serde_json::to_value(key_set()).unwrap();
            keys["keys"][0][incompatible] = match incompatible {
                "alg" => json!("RS512"),
                "use" => json!("enc"),
                "key_ops" => json!(["encrypt"]),
                _ => json!("invalid~base64"),
            };
            assert!(verify(
                &token(&original, "test-key"),
                &serde_json::from_value(keys).unwrap(),
                "test-key",
                &config,
                "expected-nonce"
            )
            .is_err());
        }
    }

    #[test]
    fn cache_headers_bound_freshness_and_respect_age_and_no_store() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        for (header, age, expected) in [
            ("max-age=900", "100", 800),
            ("max-age=999999", "0", 3600),
            ("max-age=999999", "100", 3500),
            ("max-age=999999", "4000", 0),
            ("max-age=10", "20", 0),
            ("max-age=3600, no-store", "0", 0),
            ("Max-Age=3600, No-Store", "0", 0),
            ("max-age=3600, PRIVATE", "0", 0),
            ("no-cache", "0", 0),
            ("max-age=invalid", "0", 0),
            ("", "0", 0),
            ("immutable", "0", 0),
        ] {
            let request = reqwest::Client::new().get(JWKS).build().unwrap();
            let mut response = reqwest::Response::from(
                http::Response::builder()
                    .header(CACHE_CONTROL, header)
                    .header(http::header::AGE, age)
                    .body(Vec::new())
                    .unwrap(),
            );
            assert_eq!(
                cache_lifetime(&request, &mut response, now).as_secs(),
                expected
            );
        }
    }

    #[test]
    fn cache_freshness_uses_dates_without_guessing_a_lifetime() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        for (header, value, expected) in [
            ("expires", "Tue, 14 Nov 2023 22:18:20 GMT", 300),
            ("expires", "Tue, 14 Nov 2023 22:12:20 GMT", 0),
            ("expires", "invalid", 0),
            ("last-modified", "Mon, 13 Nov 2023 22:13:20 GMT", 0),
        ] {
            let request = reqwest::Client::new().get(JWKS).build().unwrap();
            let mut response = reqwest::Response::from(
                http::Response::builder()
                    .header("date", "Tue, 14 Nov 2023 22:13:20 GMT")
                    .header(header, value)
                    .body(Vec::new())
                    .unwrap(),
            );
            assert_eq!(
                cache_lifetime(&request, &mut response, now).as_secs(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn non_cacheable_provider_documents_are_used_once_and_discarded() {
        let mock = MockGoogle::new().await;
        let mut provider = GoogleProvider::new().unwrap();
        provider.discovery_url = format!("{}/discovery", mock.url);
        let mut metadata = provider.metadata().await.unwrap();
        metadata.jwks_uri = format!("{}/keys", mock.url);
        provider.keys(&metadata, "test-key").await.unwrap();

        // Replacing a previously cacheable response must also evict its old cached value.
        for directive in ["no-store", "no-cache", ""] {
            *mock.state.cache_control.lock() = directive.into();
            {
                let mut cache = provider.cache.lock().await;
                if let Some(cached) = &mut cache.metadata {
                    cached.expires_at = Instant::now();
                }
                if let Some(cached) = &mut cache.keys {
                    cached.expires_at = Instant::now();
                }
            }
            for _ in 0..2 {
                assert_eq!(provider.metadata().await.unwrap().issuer, ISSUER);
                assert!(provider.keys(&metadata, "test-key").await.is_ok());
                let cache = provider.cache.lock().await;
                assert!(cache.metadata.is_none());
                assert!(cache.keys.is_none());
            }
        }
        assert_eq!(mock.state.discovery.load(Ordering::Relaxed), 7);
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 7);
    }

    #[tokio::test]
    async fn provider_exchange_uses_pkce_and_caches_keys_and_bounds_rotation() {
        let root = crate::test_support::directory();
        let config = config(root.path());
        let mock = MockGoogle::new().await;
        mock.reply(StatusCode::OK,json!({"access_token":"unused-test-token","token_type":"Bearer","id_token":token(&claims("nonce"),"test-key")}));
        let provider = mock.provider();
        for _ in 0..2 {
            let transaction = LoginTransaction {
                verifier: oauth2::PkceCodeVerifier::new("test-verifier".into()),
                nonce: "nonce".into(),
            };
            assert_eq!(
                provider
                    .exchange(&config, "test-code".into(), transaction)
                    .await
                    .unwrap(),
                "test-user"
            );
        }
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 1);
        let request = mock.requests().pop().unwrap();
        let params: std::collections::HashMap<_, _> =
            url::form_urlencoded::parse(request.as_bytes())
                .into_owned()
                .collect();
        assert_eq!(params["code_verifier"], "test-verifier");
        assert_eq!(params["client_secret"], "test-only-secret");
        assert_eq!(params["redirect_uri"], config.google_callback());
        assert_eq!(params["grant_type"], "authorization_code");
        let metadata = provider.metadata().await.unwrap();
        for _ in 0..3 {
            provider.keys(&metadata, "unknown").await.unwrap();
        }
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 1);
        provider.cache.lock().await.rotation_retry_at = None;
        provider.keys(&metadata, "unknown").await.unwrap();
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 2);
        provider
            .cache
            .lock()
            .await
            .keys
            .as_mut()
            .unwrap()
            .expires_at = Instant::now();
        provider.keys(&metadata, "test-key").await.unwrap();
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn exchange_and_key_failures_never_bypass_validation_or_backoff() {
        let root = crate::test_support::directory();
        let config = config(root.path());
        let mock = MockGoogle::new().await;
        let provider = mock.provider();
        mock.reply(
            StatusCode::BAD_REQUEST,
            json!({"error":"invalid_grant","error_description":"private provider error"}),
        );
        let transaction = || LoginTransaction {
            verifier: oauth2::PkceCodeVerifier::new("test-verifier".into()),
            nonce: "nonce".into(),
        };
        assert!(matches!(
            provider
                .exchange(&config, "test-code".into(), transaction())
                .await,
            Err(AuthError::Forbidden)
        ));
        mock.reply(StatusCode::OK,json!({"access_token":"unused-test-token","token_type":"Bearer","id_token":"x".repeat(16385)}));
        assert!(matches!(
            provider
                .exchange(&config, "test-code".into(), transaction())
                .await,
            Err(AuthError::Forbidden)
        ));
        let bad_algorithm = encode(
            &Header::new(Algorithm::HS256),
            &claims("nonce"),
            &EncodingKey::from_secret(b"test-only-hmac").unwrap(),
        )
        .unwrap();
        mock.reply(StatusCode::OK,json!({"access_token":"unused-test-token","token_type":"Bearer","id_token":bad_algorithm}));
        assert!(matches!(
            provider
                .exchange(&config, "test-code".into(), transaction())
                .await,
            Err(AuthError::Forbidden)
        ));
        let mut metadata = provider.metadata().await.unwrap();
        provider.keys(&metadata, "test-key").await.unwrap();
        provider
            .cache
            .lock()
            .await
            .keys
            .as_mut()
            .unwrap()
            .expires_at = Instant::now();
        metadata.jwks_uri = format!("{}/redirect", mock.url);
        assert!(matches!(
            provider.keys(&metadata, "test-key").await,
            Err(AuthError::Unavailable)
        ));
        let calls = mock.state.keys.load(Ordering::Relaxed);
        assert!(matches!(
            provider.keys(&metadata, "test-key").await,
            Err(AuthError::Unavailable)
        ));
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), calls);
    }

    #[tokio::test]
    async fn discovery_is_lazy_cached_pinned_and_backs_off_on_failure() {
        let mock = MockGoogle::new().await;
        let mut provider = GoogleProvider::new().unwrap();
        provider.discovery_url = format!("{}/discovery", mock.url);
        assert_eq!(mock.state.discovery.load(Ordering::Relaxed), 0);
        assert_eq!(provider.metadata().await.unwrap().issuer, ISSUER);
        provider.metadata().await.unwrap();
        assert_eq!(mock.state.discovery.load(Ordering::Relaxed), 1);
        for field in [
            "issuer",
            "authorization_endpoint",
            "token_endpoint",
            "jwks_uri",
        ] {
            let original = mock.state.discovery_reply.lock().clone();
            mock.state.discovery_reply.lock()[field] = json!("https://untrusted.invalid");
            provider.cache.lock().await.metadata = None;
            assert!(matches!(
                provider.metadata().await,
                Err(AuthError::Unavailable)
            ));
            let calls = mock.state.discovery.load(Ordering::Relaxed);
            assert!(provider.metadata().await.is_err());
            assert_eq!(mock.state.discovery.load(Ordering::Relaxed), calls);
            *mock.state.discovery_reply.lock() = original;
            provider.cache.lock().await.retry_at = None;
        }
    }

    #[tokio::test]
    async fn http_redirects_and_oversized_responses_fail_closed() {
        let mock = MockGoogle::new().await;
        let provider = mock.provider();
        assert!(provider
            .document::<Value>(&format!("{}/redirect", mock.url))
            .await
            .is_err());
        assert_eq!(mock.state.keys.load(Ordering::Relaxed), 0);
        assert!(provider
            .document::<Value>(&format!("{}/oversized", mock.url))
            .await
            .is_err());
        // A missing Content-Length must not bypass the streamed response limit.
        let response = provider
            .http
            .get(format!("{}/oversized-chunked", mock.url))
            .send()
            .await
            .unwrap();
        assert!(response.content_length().is_none());
        assert!(bounded_body(response).await.is_err());
        let mut cache = provider.cache.lock().await;
        cache.retry_at = Some(Instant::now() + Duration::from_secs(30));
        drop(cache);
        assert!(matches!(
            provider.metadata().await,
            Err(AuthError::Unavailable)
        ));
    }
}
