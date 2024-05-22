use leptos::{get_configuration, view};
use leptos_axum::generate_route_list;
use maj_leptos::app::App;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(debug_assertions))]
    {
        maj_leptos::init_production_tracing();
    }
    #[cfg(debug_assertions)]
    {
        maj_leptos::init_dbg_tracing();
    }

    // `None` imports the default Cargo.toml configuration. ("./Cargo.toml")
    let conf = get_configuration(None).await?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|| view! { <App/> });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on http://{}", listener.local_addr()?);

    maj_leptos::serve(listener, routes, leptos_options).await?;

    Ok(())
}
