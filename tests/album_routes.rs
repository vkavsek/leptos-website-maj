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

// Leptos route discovery suppresses resources globally; isolate it from resource-rendering unit tests.
#[tokio::test]
async fn album_routes_return_not_found_only_for_unknown_albums() {
    let options = LeptosOptions::builder()
        .output_name("leptos-website-maj")
        .site_root("tests/fixtures/site")
        .build();
    let app = Router::new()
        .leptos_routes(&options, leptos_axum::generate_route_list(App), {
            let options = options.clone();
            move || app_shell(options.clone())
        })
        .with_state(options);
    for (path, status, marker) in [
        (
            "/discography/missing-album",
            StatusCode::NOT_FOUND,
            "404 Not Found",
        ),
        ("/discography/minorflaw", StatusCode::OK, "Minor Flaw"),
        ("/discography/galaterna", StatusCode::OK, "Galaterna"),
        ("/discography", StatusCode::OK, "Discography"),
        ("/media", StatusCode::OK, "configured-root-test"),
        ("/shows/past", StatusCode::OK, "Configured site root"),
        ("/shows/future", StatusCode::OK, "no events"),
        ("/about_me", StatusCode::OK, "gallery-test.svg.webp"),
        (
            "/discography/another-missing-album",
            StatusCode::NOT_FOUND,
            "404 Not Found",
        ),
    ] {
        // Match Axum's separate request tasks so SSR reactive state cannot leak between requests.
        let response = tokio::spawn(
            app.clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), status, "{path}");
        let body = to_bytes(response.into_body(), 100_000).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains(marker), "{path}");
        if status == StatusCode::NOT_FOUND {
            assert!(html.contains("Go back to the home page."));
            assert!(!html.contains("class=\"album-display-wrap\""));
        }
    }
}
