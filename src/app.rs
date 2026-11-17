use crate::{
    audio::AudioPlayer,
    routes::{
        about::*, discography::*, error::ErrorTemplate, home::*, info::Info, media::*,
        privacy::Privacy, shows::*,
    },
    MajServerError,
};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

/// An HTML outline of our application.
/// Contains the HTML boilerplate, critical styles, etc..
pub fn app_shell(options: LeptosOptions) -> impl IntoView {
    let critical_styles_fonts = r#"
            @font-face {
                font-family: "IBM Plex Mono";
                font-style: normal;
                font-weight: 400;
                src:
                    local("IBM Plex Mono"),
                    local("IBMPlexMono"),
                    url("/fonts/IBM-Plex-Mono/fonts/complete/woff2/IBMPlexMono-Regular.woff2")
                    format("woff2"),
                    url("/fonts/IBM-Plex-Mono/fonts/complete/woff/IBMPlexMono-Regular.woff")
                    format("woff");
                font-display: swap;
            }
            @font-face {
                font-family: "LilitaOne";
                font-style: normal;
                font-weight: 400;
                src: url("/fonts/Lilita_One/LilitaOne-Regular.woff2") format("woff2");
                font-display: swap;
            }"#;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta
                    name="description"
                    content="Maj Kavšek is a Berlin-based trumpeter and composer from Ljubljana, Slovenia. Beginning his musical journey at age 8, he refined his talents at the Conservatorium for Music in Ljubljana and the Jazz Institute Berlin, studying under renowned musicians such as James Robert Rotondi and Ralph Alessi. His versatile experience spans symphonic orchestras, jazz ensembles, and solo performances, highlighted by prestigious accolades like the TEMSIG competition's 1st prize with The Mood Lab Quintet. Maj has graced stages at major festivals and collaborated with esteemed artists, showcasing his profound musicality and professional excellence."
                />
                <style>{critical_styles_fonts}</style>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let formatter = |text| format!("{text} - Maj Kavšek");
    view! {
        <Stylesheet id="leptos" href="/pkg/leptos-website-maj.css" />
        <Link rel="preload" href="/img/bg/bg_smallest.webp" as_="image" />
        <Link rel="shortcut icon" type_="image/svg" href="/img/trobenta.svg" />
        <Title formatter />
        <Router>
            <PageLayout />
        </Router>
    }
}

/// Light-dismiss fallback for browsers without `closedby` support.
pub fn use_dialog_light_dismiss(dialog_ref: NodeRef<leptos::html::Dialog>) {
    use leptos::ev::{pointerdown, pointerup, PointerEvent};
    use leptos_use_event_listener::use_event_listener;

    let (pressed_outside, set_pressed_outside) = signal(false);
    let outside = move |event: &PointerEvent| {
        dialog_ref.get_untracked().is_some_and(|dialog| {
            if web_sys::js_sys::Reflect::has(dialog.as_ref(), &"closedBy".into()).unwrap_or(false)
                || event.target().as_ref() != Some(dialog.as_ref())
            {
                return false;
            }
            let bounds = dialog.get_bounding_client_rect();
            let (x, y) = (event.client_x() as f64, event.client_y() as f64);
            x < bounds.left() || x > bounds.right() || y < bounds.top() || y > bounds.bottom()
        })
    };
    let _ = use_event_listener(dialog_ref, pointerdown, move |event| {
        set_pressed_outside.set(outside(&event));
    });
    let _ = use_event_listener(dialog_ref, pointerup, move |event| {
        // Do not dismiss a text-selection drag that starts inside the dialog.
        if pressed_outside.get_untracked() && outside(&event) {
            if let Some(dialog) = dialog_ref.get_untracked() {
                dialog.close();
            }
        }
        set_pressed_outside.set(false);
    });
}

#[component]
fn PageLayout() -> impl IntoView {
    let location = leptos_router::hooks::use_location();
    let background_position = move || match location.pathname.get().split('/').nth(1) {
        Some("about_me") => "35% 60%",
        Some("shows") => "0 bottom",
        Some("discography") => "0 top",
        Some("media") => "100% 90%",
        _ => "48% 8%",
    };
    view! {
            <div class="bg-wrapper" style:background-position=background_position>
                <nav class="nav-links">
                    <A href="/" attr:title="Home" attr:class="nav-link" attr:id="home-nav-link">
                        <img
                            class="nav-icon nav-icon-1"
                            src="/img/home.svg"
                            alt="Home"
                            width="200"
                            height="200"
                        />
                    </A>
                    <A href="/about_me" attr:title="About" attr:class="nav-link">
                        <img
                            class="nav-icon nav-icon-2"
                            src="/img/izkaznica.svg"
                            alt="About Me"
                            width="378"
                            height="330"
                        />
                    </A>
                    <A href="/shows" attr:title="Shows" attr:class="nav-link">
                        <img
                            class="nav-icon nav-icon-3"
                            src="/img/trobenta.svg"
                            alt="Shows"
                            width="512"
                            height="217"
                        />
                    </A>
                    <A href="/discography" attr:title="Discography" attr:class="nav-link">
                        <img
                            class="nav-icon nav-icon-4"
                            src="/img/vinyl.svg"
                            alt="Discography"
                            width="108"
                            height="119"
                        />
                    </A>
                    <A href="/media" attr:title="Media" attr:class="nav-link">
                        <img
                            class="nav-icon nav-icon-5"
                            src="/img/media.svg"
                            alt="Media"
                            width="395"
                            height="336"
                        />
                    </A>
                </nav>
                <AudioPlayer />
                <main>
                    <Routes fallback=|| {
                        let mut outside_errors = Errors::default();
                        outside_errors.insert_with_default_key(MajServerError::NotFound);
                        view! { <ErrorTemplate outside_errors /> }.into_view()
                    }>
                        <Route path=path!("/") view=Home />
                        <Route path=path!("/about_me") view=About />
                        <ParentRoute path=path!("/shows") view=Shows>
                            <Route path=path!("") view=ShowsFallback />
                            <Route path=path!("/past") view=PastShows />
                            <Route path=path!("/future") view=FutureShows />
                        </ParentRoute>
                        <Route path=path!("/discography") view=Discography />
                        <Route path=path!("/discography/:name") view=DiscographyDisplayAlbum />
                        <Route path=path!("/media") view=Media />
                        <Route path=path!("/privacy") view=Privacy />
                        <Route path=path!("/info") view=Info />
                    </Routes>
                </main>
            </div>
    }
}
