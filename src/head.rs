use leptos::html::Div;
use leptos::*;
use leptos_router::*;

#[component]
/// Navigation + handles dynamic BG style dispatching based on navigation.
pub fn Head(
    /// NodeRef of the div where you'll set the background image.
    bg_div_ref: NodeRef<Div>,
) -> impl IntoView {
    // DOM should be built by the time we call this, so we can unwrap.
    let do_style = |bg_div_ref: NodeRef<Div>, style: &'static str, value: &'static str| {
        bg_div_ref.get().unwrap().style(style, value);
    };

    let click_home = move |_| {
        do_style(bg_div_ref, "background-position", "46% 18%");
    };
    let click_about = move |_| {
        do_style(bg_div_ref, "background-position", "35% 60%");
    };
    let click_concerts = move |_| {
        do_style(bg_div_ref, "background-position", "0 top");
    };
    let click_media = move |_| {
        do_style(bg_div_ref, "background-position", "50% 90%");
    };

    view! {
        <nav>
            <A
                class="nav-link"
                id="home-nav-link"
                href="/"
                // aria-current=set_home
                on:click=click_home
            >
                <img class="nav-icon nav-icon-1" src="/img/trobenta.svg"/>
            </A>
            <A class="nav-link" href="/about_me" on:click=click_about>
                <img class="nav-icon nav-icon-2" src="/img/izkaznica.svg"/>
            </A>
            <A class="nav-link" href="/shows" on:click=click_concerts>
                <img class="nav-icon nav-icon-3" src="/img/nota.svg"/>
            </A>
            <A class="nav-link" href="/media" on:click=click_media>
                <img class="nav-icon nav-icon-4" src="/img/media.svg"/>
            </A>
        </nav>
    }
}
