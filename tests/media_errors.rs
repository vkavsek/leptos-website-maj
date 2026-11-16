#![cfg(feature = "ssr")]

use axum::{
    body::{to_bytes, Body},
    Router,
};
use http::{Request, StatusCode};
use leptos::config::LeptosOptions;
use leptos_axum::LeptosRoutes;
use leptos_website_maj::app::{app_shell, App};
use tower::ServiceExt;

#[tokio::test]
async fn content_failures_return_internal_server_error_before_streaming() {
    let options = LeptosOptions::builder()
        .output_name("leptos-website-maj")
        .site_root("tests/fixtures/invalid-site")
        .build();
    let app = Router::new()
        .leptos_routes(&options, leptos_axum::generate_route_list(App), {
            let options = options.clone();
            move || app_shell(options.clone())
        })
        .with_state(options);
    for (path, error) in [
        ("/media", "Internal"),
        ("/shows/past", "Internal"),
        ("/shows/future", "Internal"),
        ("/about_me", "Internal"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{path}"
        );
        let body = to_bytes(response.into_body(), 100_000).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("error-wrap"), "{path}");
        assert!(html.contains("500 Internal Server Error"), "{path}");
        assert!(html.contains(error), "{path}");
    }
}
