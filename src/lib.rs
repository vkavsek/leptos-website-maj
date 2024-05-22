// NOTE: 22.05.2024: we were getting warnings for empty docs in `component` macro
#![allow(clippy::empty_docs)]

// ###################################
// ->   MODS
// ###################################
pub mod app;
pub mod audio;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fallback;
pub mod head;
pub mod routes;
#[cfg(feature = "ssr")]
pub mod serve;

// Re-export the serve function
#[cfg(feature = "ssr")]
pub use serve::serve;

// ###################################
// ->   HYDRATE
// ###################################
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}

// ###################################
// ->   TRACING INIT
// ###################################
use tracing::Level;
use tracing_subscriber::EnvFilter;

// Initialize tracing for DEV
pub fn init_dbg_tracing() {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_max_level(Level::DEBUG)
        .compact()
        .init();
    println!("Initialized DEBUG tracing");
}

// Initialize tracing for PRODUCTION
pub fn init_production_tracing() {
    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        // Panic if the env filter RUST_LOG isn't provided!
        .with_env_filter(EnvFilter::try_from_default_env().expect("RUST_LOG is missing!"))
        .init();
    println!("Initialized PRODUCTION tracing");
}
