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
}

// Initialize tracing for PRODUCTION
pub fn init_production_tracing() {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
