use leptos::{html::Div, *};
use leptos_meta::{Link, Title};

#[component]
pub fn Home() -> impl IntoView {
    let (show_mail, set_show_mail) = create_signal(false);

    let mail_ref = create_node_ref::<Div>();

    let mail_click = move |_| {
        set_show_mail.set(true);
    };

    create_effect(move |_| {
        leptos_use::on_click_outside(mail_ref, move |_| {
            set_show_mail.set(false);
        })
    });

    let close_popup = move |_| {
        set_show_mail.set(false);
    };

    view! {
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg"/>
        <Title text="Home"/>
        <footer class="components" id="home-components">
            <div class="title" id="home-title">
                // <img class="title-img" id="home-img" src="/img/titles/maj_kavsek.svg"/>
                <h1>"Maj Kavšek"</h1>
            </div>
            <div class="contents" id="home-wrap">
                <div class="all-contacts">
                    <div class="contacts-wrap" on:click=mail_click>
                        <img class="contacts-img" src="/img/contact_icons/MAIL.svg"/>
                        <Show when=move || show_mail.get() fallback=|| {}>
                            <div class="modal" node_ref=mail_ref>
                                <button class="close-button" on:click=close_popup>
                                    "close"
                                </button>
                                <p>"maj-kavsek@mail.com"</p>
                            </div>
                        </Show>
                    </div>
                    <div class="contacts-wrap">
                        <a href="https://www.facebook.com" target="_blank">
                            <img class="contacts-img" src="/img/contact_icons/FB.svg"/>
                        </a>
                    </div>
                    <div class="contacts-wrap">
                        <a href="https://www.tiktok.com" target="_blank">
                            <img class="contacts-img" src="/img/contact_icons/TIKTOK.svg"/>
                        </a>
                    </div>
                    <div class="contacts-wrap">
                        <a href="https://www.instagram.com" target="_blank">
                            <img class="contacts-img" src="/img/contact_icons/FON.svg"/>
                        </a>
                    </div>
                </div>
            </div>
        </footer>
    }
}
