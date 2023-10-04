use leptos::{html::Div, *};
use leptos_meta::{Link, Title};

#[component]
pub fn Home() -> impl IntoView {
    let (show_phone, set_show_phone) = create_signal(false);
    let (show_mail, set_show_mail) = create_signal(false);
    let (show_fb, set_show_fb) = create_signal(false);
    let (show_ig, set_show_ig) = create_signal(false);
    let (show_tiktok, set_show_tiktok) = create_signal(false);

    let phone_ref = create_node_ref::<Div>();
    let mail_ref = create_node_ref::<Div>();
    let fb_ref = create_node_ref::<Div>();
    let ig_ref = create_node_ref::<Div>();
    let tiktok_ref = create_node_ref::<Div>();

    let phone_click = move |_| {
        set_show_phone(true);
    };
    let mail_click = move |_| {
        set_show_mail(true);
    };
    let fb_click = move |_| {
        set_show_fb(true);
    };
    let tiktok_click = move |_| {
        set_show_tiktok(true);
    };
    let ig_click = move |_| {
        set_show_ig(true);
    };

    // TODO: This can only run on CSR so it may cause problems with SSR,
    // you can wrap in create_effect() to ensure it only runs on CSR if/when needed.
    create_effect(move |_| {
        leptos_use::on_click_outside(phone_ref, move |_| {
            set_show_phone(false);
        })
    });
    create_effect(move |_| {
        leptos_use::on_click_outside(mail_ref, move |_| {
            set_show_mail(false);
        })
    });
    create_effect(move |_| {
        leptos_use::on_click_outside(fb_ref, move |_| {
            set_show_fb(false);
        })
    });
    create_effect(move |_| {
        leptos_use::on_click_outside(tiktok_ref, move |_| {
            set_show_tiktok(false);
        })
    });
    create_effect(move |_| {
        leptos_use::on_click_outside(ig_ref, move |_| {
            set_show_ig(false);
        })
    });

    let close_popup = move |_| {
        set_show_phone(false);
        set_show_mail(false);
        set_show_fb(false);
        set_show_tiktok(false);
        set_show_ig(false);
    };

    view! {
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg"/>
        <Title text="Home"/>
        <div class="components" id="home-components">
            <div class="title" id="home-title">
                <img class="title-img" id="home-img" src="/img/titles/maj_kavsek.svg"/>
            </div>
            <div class="contents" id="home-wrap">
                <div class="all-contacts">
                    <div class="contacts-wrap" on:click=phone_click>
                        <img class="contacts-img" src="/img/contact_icons/FON.svg"/>
                        <Show when=show_phone fallback=|| {}>
                            <div class="modal" node_ref=phone_ref>
                                <button class="close-button" on:click=close_popup>
                                    <i class="fa-solid fa-xmark fa-xl" style="color: #ff3988;"></i>
                                </button>
                                <p class="phone-num">"00 386 40 396 727"</p>
                            </div>
                        </Show>
                    </div>
                    <div class="contacts-wrap" on:click=fb_click>
                        <img class="contacts-img" src="/img/contact_icons/FB.svg"/>
                        <Show when=show_fb fallback=|| {}>
                            <div class="modal" node_ref=fb_ref>
                                <button class="close-button" on:click=close_popup>
                                    <i class="fa-solid fa-xmark"></i>
                                </button>
                                <a href="https://www.facebook.com" target="_blank">
                                    "https://www.facebook.com"
                                </a>
                            </div>
                        </Show>
                    </div>
                    <div class="contacts-wrap" on:click=mail_click>
                        <img class="contacts-img" src="/img/contact_icons/MAIL.svg"/>
                        <Show when=show_mail fallback=|| {}>
                            <div class="modal" node_ref=mail_ref>
                                <button class="close-button" on:click=close_popup>
                                    <i class="fa-solid fa-xmark"></i>
                                </button>
                                <p>"maj-kavsek@mail.com"</p>
                            </div>
                        </Show>
                    </div>
                    <div class="contacts-wrap" on:click=tiktok_click>
                        <img class="contacts-img" src="/img/contact_icons/TIKTOK.svg"/>
                        <Show when=show_tiktok fallback=|| {}>
                            <div class="modal" node_ref=tiktok_ref>
                                <button class="close-button" on:click=close_popup>
                                    <i class="fa-solid fa-xmark"></i>
                                </button>
                                <a href="https://www.tiktok.com" target="_blank">
                                    "https://www.tiktok.com"
                                </a>
                            </div>
                        </Show>
                    </div>
                    <div class="contacts-wrap" on:click=ig_click>
                        <img class="contacts-img" src="/img/contact_icons/FON.svg"/>
                        <Show when=show_ig fallback=|| {}>
                            <div class="modal" node_ref=ig_ref>
                                <button class="close-button" on:click=close_popup>
                                    <i class="fa-solid fa-xmark"></i>
                                </button>
                                <a href="https://www.instagram.com" target="_blank">
                                    "https://www.instagram.com"
                                </a>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>
        </div>
    }
}
