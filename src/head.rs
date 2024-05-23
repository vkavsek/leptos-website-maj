use leptos::html::Div;
use leptos::*;
use leptos_router::*;

/// Navigation + handles dynamic BG style dispatching based on navigation.
#[component]
pub fn Head(
    /// NodeRef of the div where you'll set the background image.
    bg_div_ref: NodeRef<Div>,
) -> impl IntoView {
    // DOM should be built by the time we call this, so we can unwrap.
    let do_style = |bg_div_ref: NodeRef<Div>, style: &'static str, value: &'static str| {
        let _ = bg_div_ref.get().unwrap().style(style, value);
    };

    let click_home = move |_| {
        do_style(bg_div_ref, "background-position", "48% 8%");
    };
    let click_about = move |_| {
        do_style(bg_div_ref, "background-position", "35% 60%");
    };
    let click_concerts = move |_| {
        do_style(bg_div_ref, "background-position", "0 bottom");
    };
    let click_media = move |_| {
        do_style(bg_div_ref, "background-position", "100% 90%");
    };

    view! {
        <nav>
            <A
                class="nav-link"
                id="home-nav-link"
                href="/"
                on:click=click_home
            >
                <img class="nav-icon nav-icon-1" src="/img/trobenta.svg" alt="Home" width="512" height="217"/>
            </A>
            <A class="nav-link" href="/about_me" on:click=click_about>
                <img class="nav-icon nav-icon-2" src="/img/izkaznica.svg" alt="About Me" width="378" height="330"/>
            </A>
            <A class="nav-link" href="/shows" on:click=click_concerts>
                <img class="nav-icon nav-icon-3" src="/img/nota.svg" alt="Shows" width="144" height="158"/>
            </A>
            <A class="nav-link" href="/media" on:click=click_media>
                <img class="nav-icon nav-icon-4" src="/img/media.svg" alt="Media" width="395" height="336"/>
            </A>
        </nav>
    }
}
