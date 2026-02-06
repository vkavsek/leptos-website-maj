use leptos::html::Div;
use leptos::prelude::*;
use leptos_router::components::A;

/// Navigation + handles dynamic BG style dispatching based on navigation.
#[component]
pub fn NavBar(
    /// NodeRef of the div where you'll set the background image.
    bg_div_ref: NodeRef<Div>,
) -> impl IntoView {
    // DOM should be built by the time we call this, so we can unwrap.
    let do_style = |bg_div_ref: NodeRef<Div>, style: (&'static str, &'static str)| {
        let _ = bg_div_ref.get().unwrap().style(style);
    };

    let click_home = move |_| {
        do_style(bg_div_ref, ("background-position", "48% 8%"));
    };
    let click_about = move |_| {
        do_style(bg_div_ref, ("background-position", "35% 60%"));
    };
    let click_concerts = move |_| {
        do_style(bg_div_ref, ("background-position", "0 bottom"));
    };
    let click_media = move |_| {
        do_style(bg_div_ref, ("background-position", "100% 90%"));
    };

    view! {
        <nav class="nav-links">
            <A href="/" on:click=click_home attr:class="nav-link" attr:id="home-nav-link">
                <img class="nav-icon nav-icon-1" src="/img/home.svg" alt="Home" width="200" height="200"/>
            </A>
            <A href="/about_me" on:click=click_about attr:class="nav-link">
                <img class="nav-icon nav-icon-2" src="/img/izkaznica.svg" alt="About Me" width="378" height="330"/>
            </A>
            <A href="/shows" on:click=click_concerts attr:class="nav-link">
                <img class="nav-icon nav-icon-3" src="/img/trobenta.svg" alt="Shows" width="512" height="217"/>
                // NOTE: old icon kept for width and height reference should they be needed again
                // <img class="nav-icon nav-icon-3" src="/img/nota.svg" alt="Shows" width="144" height="158"/>
            </A>
            <A href="/media" on:click=click_media attr:class="nav-link">
                <img class="nav-icon nav-icon-4" src="/img/media.svg" alt="Media" width="395" height="336"/>
            </A>
        </nav>
    }
}
