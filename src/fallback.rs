use crate::{app::App, MajServerError};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use leptos::{view, LeptosOptions};
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn static_file_and_err_handler(
    uri: Uri,
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> Response {
    let root = options.site_root.clone();
    let res = match get_static_file(uri.clone(), &root).await {
        Ok(res) => res,
        Err(err) => err.into_response(),
    };

    // Return the file if found or render App to stream,
    // in which case the App renders the 404 page.
    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        let request = format!("{} @ {}", req.method(), req.uri());

        // TODO: This is stupid improve error handling
        tracing::error!("{}", request);
        let handler = leptos_axum::render_app_to_stream(options.to_owned(), move || view! {<App/>});
        handler(req).await.into_response()
    }
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response, MajServerError> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .unwrap();
    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // This path is relative to the cargo root
    match ServeDir::new(root).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(_err) => Err(MajServerError::Internal),
    }
}
