//! Public portfolio description and privacy-policy link for application branding.

use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Info() -> impl IntoView {
    view! {
        <Title text="Site information" />
        <div class="info-page">
            <article class="site-info" aria-labelledby="info-title" tabindex="0">
                <h1 id="info-title">"Site information"</h1>
                <div class="home-intro">
                    <p>"Maj Kavšek’s Portfolio brings together music, photos, live shows and videos from the trumpeter and composer."</p>
                    <a href="/privacy">"Privacy policy"</a>
                </div>
            </article>
        </div>
    }
}
