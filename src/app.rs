use crate::{
    audio::AudioWrapper,
    head::*,
    routes::{about::*, error::ErrorTemplate, home::*, media::*, shows::*},
    MajServerError,
};
use core::time::Duration;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

#[component]
pub fn App() -> impl IntoView {
    // FIXME: figure out leptos_meta
    provide_meta_context();

    let bg_div_ref = NodeRef::new();

    let formatter = |text| format!("{text} - Maj Kavšek");
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
        <Style>{critical_styles_fonts}</Style>
        <Stylesheet id="leptos" href="/pkg/leptos-website-maj.css" />
        <Link rel="shortcut icon" type_="image/svg" href="/img/trobenta.svg" />
        <Link rel="preload" href="/img/bg/bg_smallest.webp" as_="image" />
        <Html {..} lang="en" />
        <Meta
            name="description"
            content="Maj Kavšek is a Berlin-based trumpeter and composer from Ljubljana, Slovenia. Beginning his musical journey at age 8, he refined his talents at the Conservatorium for Music in Ljubljana and the Jazz Institute Berlin, studying under renowned musicians such as James Robert Rotondi and Ralph Alessi. His versatile experience spans symphonic orchestras, jazz ensembles, and solo performances, highlighted by prestigious accolades like the TEMSIG competition's 1st prize with The Mood Lab Quintet. Maj has graced stages at major festivals and collaborated with esteemed artists, showcasing his profound musicality and professional excellence."
        />
        <Meta name="viewport" content="width=device-width, initial-scale=1" />
        <Title formatter />
        <Router>
            <div class="bg-wrapper" node_ref=bg_div_ref>
                <NavBar bg_div_ref />
                <AudioWrapper />
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
                        <Route path=path!("/media") view=Media />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}

pub fn use_interval<T, F>(interval_millis: T, f: F)
where
    F: Fn() + Clone + 'static,
    T: Into<Signal<u64>> + 'static,
{
    let interval_millis = interval_millis.into();
    Effect::new(move |prev_handle: Option<IntervalHandle>| {
        // effects get their previous return value as an argument
        // each time the effect runs, it will return the interval handle
        // so if we have a previous one, we cancel it
        if let Some(prev_handle) = prev_handle {
            prev_handle.clear();
        };

        // here, we return the handle
        set_interval_with_handle(
            f.clone(),
            // this is the only reactive access, so this effect will only
            // re-run when the interval changes
            Duration::from_millis(interval_millis.get()),
        )
        .expect("could not create interval")
    });
}
