#![recursion_limit = "256"]

use leptos::config::get_configuration;
use leptos_axum::generate_route_list;
use leptos_website_maj::app::App;
use leptos_website_maj::server_config::ServerConfig;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the process-wide creation mask before any runtime threads can create files.
    // Gives the server private defaults for newly created files and directories.
    // 0: remove no owner permissions,
    // 7: remove all group permissions,
    // 7: remove all permissions for everyone else
    #[cfg(unix)]
    // SAFETY: umask has no preconditions; this runs before threads start, avoiding creation races.
    unsafe {
        libc::umask(0o077);
    }
    #[cfg(not(debug_assertions))]
    {
        leptos_website_maj::init_production_tracing();
    }
    #[cfg(debug_assertions)]
    {
        leptos_website_maj::init_dbg_tracing();
    }

    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let migrate_only = match arguments.as_slice() {
        [] => false,
        [admin, command] if admin == "admin" && command == "migrate" => true,
        _ => return Err("Usage: leptos-website-maj [admin migrate]".into()),
    };
    // Local database administration does not need Google, SMTP, or HTTP configuration.
    let mut server_config = if migrate_only {
        None
    } else {
        Some(ServerConfig::from_env()?)
    };
    let data_dir = std::path::PathBuf::from(
        std::env::var_os("MAJ_DATA_DIR").ok_or("MAJ_DATA_DIR is required")?,
    );
    if std::env::var_os("FLY_APP_NAME").is_some() {
        leptos_website_maj::storage::require_fly_volume(&data_dir)?;
    }

    // `None` imports the default Cargo.toml configuration. ("./Cargo.toml")
    let conf = get_configuration(None)?;
    let leptos_options = conf.leptos_options;
    let data_dir = leptos_website_maj::storage::prepare(
        &data_dir,
        &[
            std::path::Path::new(leptos_options.site_root.as_ref()),
            std::path::Path::new("public"),
        ],
    )?;

    tokio::runtime::Runtime::new()?.block_on(async move {
        let database = leptos_website_maj::database::initialize(&data_dir)
            .await
            .map_err(|error| format!("Database initialization failed: {error}"))?;
        if migrate_only {
            database.close().await;
            info!("Database migrations and session schema are ready");
            return Ok(());
        }

        let mut server_config = server_config
            .take()
            .expect("HTTP configuration was validated");
        server_config.data_dir = data_dir;

        let routes = generate_route_list(App);
        let listener = tokio::net::TcpListener::bind(&leptos_options.site_addr).await?;
        info!("Listening on http://{}", listener.local_addr()?);

        let result = leptos_website_maj::serve(
            listener,
            routes,
            leptos_options,
            server_config,
            database.clone(),
        )
        .await;
        database.close().await;
        result
    })
}
