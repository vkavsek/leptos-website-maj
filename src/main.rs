use leptos::{get_configuration, view};
use leptos_axum::generate_route_list;
use maj_leptos::app::App;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(debug_assertions))]
    {
        init_production_tracing();
    }
    #[cfg(debug_assertions)]
    {
        init_dbg_tracing();
    }

    // `None` imports the default Cargo.toml configuration. ("./Cargo.toml")
    let conf = get_configuration(None).await?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|| view! { <App/> });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on http://{}", listener.local_addr()?);

    serve(listener, routes, leptos_options).await?;

    Ok(())
}

// ###################################
// ->   SERVE
// ###################################
use maj_leptos::fallback::static_file_handler;

use axum::{body::Body, Router};
use http::{HeaderName, Request, Response};
use leptos::LeptosOptions;
use leptos_axum::LeptosRoutes;
use leptos_router::RouteListing;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    classify::ServerErrorsFailureClass,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::Span;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn serve(
    listener: TcpListener,
    routes: Vec<RouteListing>,
    state: LeptosOptions,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let app = Router::new()
        .leptos_routes(&state, routes, App)
        .route("/health-check", axum::routing::get(health))
        .layer(
            ServiceBuilder::new()
                // Set the compression layer
                .layer(tower_http::compression::CompressionLayer::new())
                // Set UUID per request
                .layer(SetRequestIdLayer::new(
                    x_request_id.clone(),
                    MakeRequestUuid,
                ))
                .layer(trace_layer)
                // Propagate UUID to response, keep it last so it processes the response first!
                .layer(PropagateRequestIdLayer::new(x_request_id)),
        )
        .fallback(static_file_handler)
        .with_state(state);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> axum::http::StatusCode {
    http::StatusCode::OK
}

// ###################################
// ->   TRACING INIT
// ###################################
use tracing::Level;
use tracing_subscriber::EnvFilter;

// Initialize tracing for DEV
pub fn init_dbg_tracing() {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_max_level(Level::DEBUG)
        .compact()
        .init();
}

// Initialize tracing for PRODUCTION
pub fn init_production_tracing() {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
