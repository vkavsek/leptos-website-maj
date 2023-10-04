use crate::{
    audio::AudioWrapper,
    error_template::{AppError, ErrorTemplate},
    head::*,
    routes::{about::*, home::*, media::*, shows::*},
};
use leptos::html::Div;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let bg_div_ref = create_node_ref::<Div>();

    let formatter = |text| format!("{text} - Maj Kavšek");
    view! {
        <Stylesheet id="leptos" href="/styles/index.css"/>
        <Stylesheet id="font-1" href="/styles/fonts/ibm-plex.css"/>
        <Stylesheet id="font-2" href="/styles/fonts/brianjames.css"/>
        <Title formatter/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <div class="bg-wrapper" node_ref=bg_div_ref>
                <Head bg_div_ref/>
                <AudioWrapper/>
                <main>
                    <Routes>
                        <Route path="/" view=Home/>
                        <Route path="/about_me" view=About/>
                        <Route path="/shows" view=Concerts>
                            <Route path="" view=ConcertsFallback/>
                            <Route path="/past" view=Past/>
                            <Route path="/future" view=Future/>
                        </Route>
                        <Route path="/media" view=Media/>
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
