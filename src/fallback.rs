use crate::{app::app_shell, MajServerError};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use leptos::config::LeptosOptions;
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn static_file_and_err_handler(
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> Response {
    let root = options.site_root.clone();
    let (parts, body) = req.into_parts();

    let mut static_parts = parts.clone();
    static_parts.headers.clear();
    if let Some(encodings) = parts.headers.get("accept-encoding") {
        static_parts
            .headers
            .insert("accept-encoding", encodings.clone());
    }

    let res = match get_static_file(Request::from_parts(static_parts, Body::empty()), &root).await {
        Ok(res) => res,
        Err(err) => err.into_response(),
    };

    // Return the file if found or render App to stream,
    // in which case the App renders the 404 page.
    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        let request = format!("{} @ {}", parts.method, parts.uri);

        // TODO: This is stupid improve error handling
        tracing::error!("{}", request);
        let handler = leptos_axum::render_app_to_stream(move || app_shell(options.clone()));
        handler(Request::from_parts(parts, body))
            .await
            .into_response()
    }
}

async fn get_static_file(req: Request<Body>, root: &str) -> Result<Response, MajServerError> {
    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // This path is relative to the cargo root
    match ServeDir::new(root)
        .precompressed_gzip()
        .precompressed_br()
        .oneshot(req)
        .await
    {
        Ok(res) => Ok(res.into_response()),
        Err(_err) => Err(MajServerError::Internal),
    }
}
