use leptos::html::Div;
use leptos::*;
use leptos_router::*;

#[component]
/// Navigation + handles dynamic BG style dispatching based on navigation.
pub fn Head(
    /// NodeRef of the div where you'll set the background image.
    bg_div_ref: NodeRef<Div>,
) -> impl IntoView {
    let (home_page, set_home_page) = create_signal(false);
    let (shows_page, set_shows_page) = create_signal(false);
    let set_home = move || if home_page.get() { "page" } else { "" };
    let set_shows = move || if shows_page.get() { "page" } else { "" };

    // DOM should be built by the time we call this, so we can unwrap.
    let do_style = |bg_div_ref: NodeRef<Div>, style: &'static str, value: &'static str| {
        bg_div_ref.get().unwrap().style(style, value);
    };

    let click_home = move |_| {
        do_style(bg_div_ref, "background-position", "46% 18%");
        set_home_page.set(true);
        set_shows_page.set(false);
    };
    let click_about = move |_| {
        do_style(bg_div_ref, "background-position", "35% 60%");
        set_home_page.set(false);
        set_shows_page.set(false);
    };
    let click_concerts = move |_| {
        do_style(bg_div_ref, "background-position", "0 top");
        set_home_page.set(false);
        set_shows_page.set(true);
    };
    let click_media = move |_| {
        do_style(bg_div_ref, "background-position", "50% 90%");
        set_home_page.set(false);
        set_shows_page.set(false);
    };

    view! {
        <nav>
            <a
                class="nav-link"
                id="home-nav-link"
                href="/"
                aria-current=set_home
                on:click=click_home
            >
                <img class="nav-icon nav-icon-1" src="/img/trobenta.svg"/>
            </a>
            <A class="nav-link" href="/about_me" on:click=click_about>
                <img class="nav-icon nav-icon-2" src="/img/izkaznica.svg"/>
            </A>
            <a aria-current=set_shows class="nav-link" href="/shows/past" on:click=click_concerts>
                <img class="nav-icon nav-icon-3" src="/img/nota.svg"/>
            </a>
            <A class="nav-link" href="/media" on:click=click_media>
                <img class="nav-icon nav-icon-4" src="/img/media.svg"/>
            </A>
        </nav>
    }
}
