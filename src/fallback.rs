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

    let res = match get_static_file(Request::from_parts(parts.clone(), Body::empty()), &root).await
    {
        Ok(res) => res,
        Err(err) => err.into_response(),
    };

    // Only missing files should render the app's 404 page.
    if res.status() != StatusCode::NOT_FOUND {
        res
    } else {
        // Unmatched URLs can carry private OAuth codes or other sensitive query values.
        tracing::error!(method = %parts.method, path = parts.uri.path(), "Static file not found");
        let handler = leptos_axum::render_app_to_stream(move || app_shell(options.clone()));
        handler(Request::from_parts(parts, body))
            .await
            .into_response()
    }
}

async fn get_static_file(req: Request<Body>, root: &str) -> Result<Response, MajServerError> {
    // ServeDir decodes and normalizes paths. Block aliases as well as direct paths
    // so stale seed files cannot bypass the live catalog.
    let decoded = percent_encoding::percent_decode_str(req.uri().path())
        .decode_utf8()
        .map_err(|_| MajServerError::NotFound)?;
    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            part => segments.push(part),
        }
    }
    if segments.first() == Some(&"data_json")
        || matches!(
            segments.as_slice(),
            ["img", "about_pics" | "about_thumbnails", ..]
        )
    {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use leptos_axum::LeptosRoutes;
    use tracing::instrument::WithSubscriber;

    #[tokio::test]
    async fn seed_json_is_never_served_even_from_stale_builds_or_aliases() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("data_json")).unwrap();
        std::fs::write(
            root.path().join("data_json/shows.json"),
            b"private-seed-canary",
        )
        .unwrap();
        for path in [
            "/data_json/shows.json",
            "/%64ata_json/shows.json",
            "/other/../data_json/shows.json",
        ] {
            let response = get_static_file(
                Request::get(path).body(Body::empty()).unwrap(),
                root.path().to_str().unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn missing_page_logs_the_path_without_private_query_values() {
        let root = tempfile::tempdir().unwrap();
        let log = tempfile::NamedTempFile::new().unwrap();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_max_level(tracing::Level::ERROR)
            .with_writer(log.reopen().unwrap())
            .finish();
        let options = LeptosOptions::builder()
            .output_name("test")
            .site_root(root.path().to_str().unwrap())
            .build();
        // Initialize the renderer through the same router integration as the server.
        let router = axum::Router::new()
            .leptos_routes(&options, Vec::new(), || ())
            .fallback(static_file_and_err_handler)
            .with_state(options);
        let response = router
            .oneshot(
                Request::get("/missing?private=query_canary&code=oauth_canary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .with_subscriber(subscriber)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let output = std::fs::read_to_string(log.path()).unwrap();
        assert!(output.contains("/missing"));
        assert!(output.contains("GET"));
        assert!(!output.contains("query_canary"));
        assert!(!output.contains("oauth_canary"));
        assert!(!output.contains("?private="));
    }
}
