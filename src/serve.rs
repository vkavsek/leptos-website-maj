// ###################################
// ->   SERVE
// ###################################
use crate::app::app_shell;
use crate::database::DatabaseManager;
use crate::fallback::static_file_and_err_handler;
use crate::server_config::ServerConfig;

use axum::{body::Body, middleware, routing::get, Extension, Router};
use http::{HeaderName, Request, Response};
use leptos::config::LeptosOptions;
use leptos_axum::{AxumRouteListing, LeptosRoutes};
use std::{future::IntoFuture, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    classify::ServerErrorsFailureClass,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::Span;

const REQUEST_ID_HEADER: &str = "x-request-id";

/// SERVE
/// The core async function returning a future that will serve this application.
///
/// Accepts a listener, generated Leptos routes, rendering options, validated server
/// configuration, and the initialized database manager. Configuration and database
/// are shared through server-only extensions, never hydration data.
/// It sets up a TraceLayer that provides console logging.
/// It returns a `Result` containing a `Serve` future. Needs to be awaited:
/// ```ignore
/// leptos_website_maj::serve(listener, routes, leptos_options, server_config, database).await?;
/// ```
pub async fn serve(
    listener: TcpListener,
    routes: Vec<AxumRouteListing>,
    state: LeptosOptions,
    server_config: ServerConfig,
    database: DatabaseManager,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a cleanup task that removes expired temporary auth data from SQLite.
    // The first tick is immediate; later ticks follow a five-minute schedule.
    let mut cleanup = tokio::task::JoinSet::new();
    let cleanup_database = database.clone();
    cleanup.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            // Purge expired login transactions and browser sessions without blocking HTTP startup.
            if cleanup_database.cleanup_expired_auth().await.is_err() {
                // Keep serving and retry next tick without logging stored authentication details.
                tracing::warn!("Expired authentication record cleanup failed");
            }
        }
    });
    let app = router(routes, state, server_config, database);

    // Notify select! when shutdown starts; normal server uptime must have no timeout.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(());
        })
        .into_future();

    // Reuse the same server future while waiting for a signal and while draining requests.
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result?,
        _ = shutdown_rx => {
            // Cover SMTP's 20-second limit while staying within Fly's 30-second shutdown allowance.
            match tokio::time::timeout(Duration::from_secs(25), server).await {
                Ok(result) => result?,
                // Returning lets main drop the runtime and close unfinished connections.
                Err(_) => tracing::warn!("Shutdown deadline reached; closing remaining connections"),
            }
        }
    }

    Ok(())
}

/// Builds the production route/context/middleware stack without binding a socket.
pub fn router(
    routes: Vec<AxumRouteListing>,
    state: LeptosOptions,
    server_config: ServerConfig,
    database: DatabaseManager,
) -> Router {
    let sessions = crate::auth::session_layer(&database, server_config.development);
    let server_config = Arc::new(server_config);
    let login_routes = crate::auth::login_routes(server_config.clone());
    let context_database = database.clone();
    let context_config = server_config.clone();
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request<Body>| {
            let uuid = req
                .headers()
                .get(REQUEST_ID_HEADER)
                .map(|uuid| uuid.to_str().unwrap_or("").to_string());

            tracing::info_span!("req", id = uuid)
        })
        .on_response(|res: &Response<Body>, latency: Duration, _s: &Span| {
            let st_code = res.status();
            tracing::info!("END in: {:?} STATUS: {st_code}", latency)
        })
        .on_request(|req: &Request<Body>, _s: &Span| {
            tracing::info!("START: {} @ {}", req.method(), req.uri().path(),)
        })
        .on_failure(
            |err: ServerErrorsFailureClass, latency: Duration, _s: &Span| {
                tracing::error!("ERROR: {err:?} — latency: {:?}", latency)
            },
        );

    // build our application with a route
    Router::new()
        .leptos_routes_with_context(
            &state,
            routes,
            move || {
                leptos::prelude::provide_context(context_database.clone());
                leptos::prelude::provide_context(context_config.clone());
            },
            {
                let options = state.clone();
                move || app_shell(options.clone())
            },
        )
        .route("/health-check", get(health))
        .merge(login_routes)
        .layer(
            // Adding tower_http services (except CompressionLayer)
            ServiceBuilder::new()
                // Set UUID per request
                .layer(SetRequestIdLayer::new(
                    x_request_id.clone(),
                    MakeRequestUuid,
                ))
                // Tracing Layer
                .layer(trace_layer)
                // Propagate UUID to response, keep it last so it processes the response first!
                .layer(PropagateRequestIdLayer::new(x_request_id)),
        )
        .fallback(static_file_and_err_handler)
        //  Set the compression layer for all routes.
        .layer(tower_http::compression::CompressionLayer::new())
        // Add Cache-Control without changing file-service responses.
        .layer(middleware::map_response(midware::response_mapper_midware))
        .layer(middleware::from_fn(crate::auth::private_routes))
        .layer(Extension(server_config))
        .layer(Extension(database))
        .layer(sessions)
        .with_state(state)
}

/// Waits for Ctrl+C (SIGINT on Unix) or Unix SIGTERM so Axum can begin graceful shutdown.
///
/// Panics if a signal handler cannot be registered, rather than silently losing shutdown support.
async fn shutdown_signal() {
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("could not register SIGTERM handler")
            .recv()
            .await;
    };
    // Keep the SIGTERM branch inactive on platforms where only Ctrl+C is supported here.
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = tokio::signal::ctrl_c() => result.expect("could not register interrupt handler"),
        _ = terminate => {},
    }
    tracing::info!("Shutdown requested; draining active requests for up to 25 seconds");
}

async fn health() -> axum::http::StatusCode {
    http::StatusCode::OK
}

mod midware {
    use axum::response::Response;
    use http::header::CACHE_CONTROL;
    use tracing::debug;

    /// Adds Cache-Control without replacing the file service's status, headers, or body.
    pub async fn response_mapper_midware(res: Response) -> Response {
        let mut resp = res;

        if resp.headers().contains_key(CACHE_CONTROL) {
            return resp;
        }

        // Add `Cache-control` header
        debug!("Adding cache control header");
        if cfg!(debug_assertions) {
            resp.headers_mut().insert(
                CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, proxy-revalidate"
                    .parse()
                    .unwrap(),
            );
        } else {
            resp.headers_mut().insert(
                CACHE_CONTROL,
                "max-age=3600, must-revalidate".parse().unwrap(),
            );
        }

        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use http::{header::*, StatusCode};
    use std::io::Read;
    use tower::ServiceExt;

    // Test-only endpoints exercise real Leptos registration/context without a production login bypass.
    #[leptos::server(
        Section5ContextProbe,
        "/api/admin",
        "GetJson",
        "section5_context_probe"
    )]
    async fn context_probe() -> Result<i64, leptos::prelude::ServerFnError> {
        let user = crate::auth::require_server_admin()
            .await
            .map_err(crate::auth::AuthError::server_error)?;
        if leptos::prelude::use_context::<Arc<ServerConfig>>().is_none() {
            return Err(leptos::prelude::ServerFnError::new(
                "Server configuration context missing",
            ));
        }
        Ok(user.id)
    }

    #[leptos::server(name = Section5MutationProbe, prefix = "/api/admin", input = leptos::server_fn::codec::Json, output = leptos::server_fn::codec::Json, endpoint = "section5_mutation_probe")]
    async fn mutation_probe() -> Result<i64, leptos::prelude::ServerFnError> {
        Ok(crate::auth::require_server_mutation()
            .await
            .map_err(crate::auth::AuthError::server_error)?
            .id)
    }

    #[tokio::test]
    async fn production_router_protects_private_namespaces_and_shares_leptos_session_context() {
        let (root, database) = crate::test_support::database().await;
        let options = LeptosOptions::builder()
            .output_name("leptos-website-maj")
            .site_root(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/site"))
            .build();
        let app = router(
            Vec::new(),
            options,
            crate::test_support::config(root.path()),
            database.clone(),
        );
        let response = app
            .clone()
            .oneshot(Request::get("/health-check").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(SET_COOKIE));
        for path in [
            "/admin",
            "/api/admin/direct-request",
            "/api/admin/section5_context_probe",
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
            assert_eq!(response.headers()[REFERRER_POLICY], "no-referrer");
        }
        let session = crate::auth::test_session(&database);
        let id = database
            .grant_administrator("https://accounts.google.com", "123", "owner")
            .await
            .unwrap();
        crate::auth::establish(&session, &database, "https://accounts.google.com", "123")
            .await
            .unwrap();
        let cookie = format!("maj_admin_dev={}", session.id().unwrap());
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/admin/section5_context_probe")
                    .header(COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 4096).await.unwrap(),
            id.to_string()
        );
        let token: serde_json::Value = session.get("administrator").await.unwrap().unwrap();
        for valid in [false, true] {
            let request = Request::post("/api/admin/section5_mutation_probe")
                .header(COOKIE, &cookie)
                .header(ORIGIN, "http://localhost:8080")
                .header(
                    "x-csrf-token",
                    if valid {
                        token["csrf"].as_str().unwrap()
                    } else {
                        "invalid"
                    },
                )
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                if valid {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                }
            );
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        }
        database
            .revoke_administrator("https://accounts.google.com", "123")
            .await
            .unwrap();
        let response = app
            .oneshot(
                Request::get("/api/admin/section5_context_probe")
                    .header(COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        database.close().await;
    }

    #[tokio::test]
    async fn json_content_apis_use_the_site_root_from_router_state() {
        let options = LeptosOptions::builder()
            .output_name("leptos-website-maj")
            .site_root(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/site"))
            .build();
        let app = Router::new()
            .leptos_routes(&options, Vec::new(), || ())
            .with_state(options);
        for (path, marker) in [
            ("/api/get_shows", "Configured site root"),
            ("/api/get_media_videos", "configured-root-test"),
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            assert!(
                std::str::from_utf8(&body).unwrap().contains(marker),
                "{path}"
            );
        }
    }

    const AUDIO: &str = "/music/audio-promo_small.mp3";
    const AUDIO_FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/public/music/audio-promo_small.mp3"
    );

    async fn file_response(request: http::request::Builder) -> axum::response::Response {
        let options = LeptosOptions::builder()
            .output_name("leptos-website-maj")
            .site_root(concat!(env!("CARGO_MANIFEST_DIR"), "/public"))
            .build();
        Router::new()
            .leptos_routes(&options, Vec::new(), || ())
            .fallback(static_file_and_err_handler)
            .layer(tower_http::compression::CompressionLayer::new())
            .layer(middleware::map_response(midware::response_mapper_midware))
            .with_state(options)
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn audio_range_returns_only_the_requested_bytes() {
        let mut audio = std::fs::File::open(AUDIO_FILE).unwrap();
        let length = audio.metadata().unwrap().len();
        let mut expected = [0; 2];
        audio.read_exact(&mut expected).unwrap();

        let response = file_response(
            Request::get(AUDIO)
                .header(RANGE, "bytes=0-1")
                .header(ACCEPT_ENCODING, "gzip, br"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            response.headers()[CONTENT_RANGE],
            format!("bytes 0-1/{length}")
        );
        assert_eq!(response.headers()[CONTENT_LENGTH], "2");
        assert_eq!(response.headers()[CONTENT_TYPE], "audio/mpeg");
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(
            to_bytes(response.into_body(), 2).await.unwrap(),
            &expected[..]
        );
    }

    #[tokio::test]
    async fn file_cache_checks_use_the_original_headers() {
        let response = file_response(Request::head(AUDIO)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[CONTENT_LENGTH],
            std::fs::metadata(AUDIO_FILE).unwrap().len().to_string()
        );
        let last_modified = response.headers()[LAST_MODIFIED].clone();
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());

        let response =
            file_response(Request::get(AUDIO).header(IF_MODIFIED_SINCE, last_modified.clone()))
                .await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()[LAST_MODIFIED], last_modified);
        assert!(response.headers().contains_key(CACHE_CONTROL));
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());

        for (date, status) in [
            ("Thu, 01 Jan 2099 00:00:00 GMT", StatusCode::NOT_MODIFIED),
            ("Thu, 01 Jan 1970 00:00:00 GMT", StatusCode::OK),
            ("invalid date", StatusCode::OK),
        ] {
            let response =
                file_response(Request::head(AUDIO).header(IF_MODIFIED_SINCE, date)).await;
            assert_eq!(response.status(), status, "If-Modified-Since: {date}");
            assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn file_errors_are_not_replaced_with_html() {
        let length = std::fs::metadata(AUDIO_FILE).unwrap().len();
        let response =
            file_response(Request::get(AUDIO).header(RANGE, format!("bytes={length}-"))).await;
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(
            response.headers()[CONTENT_RANGE],
            format!("bytes */{length}")
        );
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());

        let response = file_response(Request::post(AUDIO)).await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers()[ALLOW], "GET,HEAD");
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());

        let response = file_response(
            Request::get(AUDIO).header(IF_UNMODIFIED_SINCE, "Thu, 01 Jan 1970 00:00:00 GMT"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn only_missing_files_render_the_app_error_page() {
        let response = file_response(Request::get("/fonts")).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(response.headers()[LOCATION], "/fonts/");

        let response = file_response(Request::get("/missing-audit-test-file.mp3")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers()[CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        let body = to_bytes(response.into_body(), 100_000).await.unwrap();
        assert!(std::str::from_utf8(&body)
            .unwrap()
            .contains("doesn't exist!"));
    }
}
