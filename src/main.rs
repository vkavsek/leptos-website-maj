#![recursion_limit = "256"]

use clap::Parser;
use leptos::config::get_configuration;
use leptos_axum::generate_route_list;
use leptos_website_maj::app::App;
use leptos_website_maj::cli::{AdminCommand, Cli, Command};
use leptos_website_maj::database::DatabaseManager;
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
    let cli = Cli::parse();
    let command = cli.command.map(|Command::Admin(command)| command);
    if command.is_some() {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .without_time()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    } else {
        #[cfg(not(debug_assertions))]
        {
            leptos_website_maj::init_production_tracing();
        }
        #[cfg(debug_assertions)]
        {
            leptos_website_maj::init_dbg_tracing();
        }
    }

    // Local database administration does not need Google, SMTP, or HTTP configuration.
    let mut server_config = if command.is_some() {
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

    let options = if command.is_none() {
        Some(get_configuration(None)?.leptos_options)
    } else {
        None
    };
    let site_root = options
        .as_ref()
        .map(|options| std::path::PathBuf::from(options.site_root.as_ref()))
        .unwrap_or_else(|| {
            std::env::var_os("LEPTOS_SITE_ROOT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "target/site".into())
        });
    let public_roots = vec![site_root, "public".into(), "site".into()];
    let backup = matches!(command, Some(AdminCommand::Backup { .. }));
    if backup && !data_dir.join("admin.sqlite3").is_file() {
        return Err("Backup requires an existing database".into());
    }

    let data_dir = leptos_website_maj::storage::prepare(
        &data_dir,
        &public_roots
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
    )?;
    let _storage_lock = leptos_website_maj::storage::lock(
        &data_dir,
        command
            .as_ref()
            .is_some_and(AdminCommand::needs_exclusive_storage),
    )?;

    tokio::runtime::Runtime::new()?.block_on(async move {
        let database = if backup {
            DatabaseManager::open(&data_dir)
                .await
                .map_err(|error| error.into())
        } else {
            DatabaseManager::initialize(&data_dir).await
        }
        .map_err(|error| format!("Database initialization failed: {error}"))?;

        if let Some(command) = command {
            let result =
                leptos_website_maj::cli::run(command, &database, &data_dir, &public_roots).await;
            database.close().await;
            return result;
        }

        let mut server_config = server_config
            .take()
            .expect("HTTP configuration was validated");
        server_config.data_dir = data_dir;

        let leptos_options = options.expect("HTTP options were validated");
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
