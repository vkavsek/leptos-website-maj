pub mod app;
pub mod audio;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fallback;
pub mod head;
pub mod routes;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
