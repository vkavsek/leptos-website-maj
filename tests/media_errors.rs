#![cfg(feature = "ssr")]

use axum::{
    body::{to_bytes, Body},
    Router,
};
use http::Request;
use leptos::config::LeptosOptions;
use leptos_axum::LeptosRoutes;
use leptos_website_maj::app::{app_shell, App};
use tower::ServiceExt;

#[tokio::test]
async fn media_content_failure_renders_an_error() {
    let options = LeptosOptions::builder()
        .output_name("leptos-website-maj")
        .site_root("tests/fixtures/missing-site")
        .build();
    let app = Router::new()
        .leptos_routes(&options, leptos_axum::generate_route_list(App), {
            let options = options.clone();
            move || app_shell(options.clone())
        })
        .with_state(options);
    let response = app
        .oneshot(Request::get("/media").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), 100_000).await.unwrap();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(html.contains("error-wrap"));
    assert!(html.contains("500 Internal Server Error"));
    assert!(html.contains("Io("));
    assert!(!html.contains("class=\"yt-video\""));
}
