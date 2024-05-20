use axum::Router;
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use maj_fullstack::app::App;
use maj_fullstack::fallback::static_file_handler;

#[tokio::main]
async fn main() {
    // FIXME:
    simple_logger::init_with_level(log::Level::Error).expect("couldn't initialize logging");
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|| view! { <App/> });

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .layer(tower_http::compression::CompressionLayer::new())
        .fallback(static_file_handler)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // FIXME:
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
