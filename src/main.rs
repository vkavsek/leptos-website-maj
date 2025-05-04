use leptos::config::get_configuration;
use leptos_axum::generate_route_list;
use leptos_website_maj::app::App;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(debug_assertions))]
    {
        leptos_website_maj::init_production_tracing();
    }
    #[cfg(debug_assertions)]
    {
        leptos_website_maj::init_dbg_tracing();
    }

    // `None` imports the default Cargo.toml configuration. ("./Cargo.toml")
    let conf = get_configuration(None)?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on http://{}", listener.local_addr()?);

    leptos_website_maj::serve(listener, routes, leptos_options).await?;

    Ok(())
}
