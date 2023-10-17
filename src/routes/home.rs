use leptos::{
    ev::MouseEvent,
    html::{Div, Img},
    *,
};
use leptos_meta::{Link, Title};
use leptos_use::{use_interval_fn, utils::Pausable};

#[component]
pub fn Home() -> impl IntoView {
    const FB: LinkLocation = LinkLocation::Fb;
    const TIKTOK: LinkLocation = LinkLocation::TikTok;
    const IG: LinkLocation = LinkLocation::Ig;
    view! {
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg"/>
        <Title text="Home"/>
        <footer class="components" id="home-components">
            <div class="title">
                // <img class="title-img" id="home-img" src="/img/titles/maj_kavsek.svg"/>
                <h1 id="home-title">"Maj Kavšek"</h1>
            </div>
            <div class="contents" id="home-wrap">
                <div class="all-contacts">
                    <div class="contacts-wrap">
                        <Mail/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=FB/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=IG/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=TIKTOK/>
                    </div>
                </div>
            </div>
        </footer>
    }
}

#[component]
fn Mail() -> impl IntoView {
    const EMAIL_ADDR: &str = "maj-kavsek@mail.com";
    let mailto = || format!("mailto:{}", EMAIL_ADDR);
    let copy_mail = format!(
        r#"function copy_mail() {{
               // Copy the text inside the text field
               navigator.clipboard.writeText("{}");
            }}"#,
        EMAIL_ADDR
    );
    let (show_mail, set_show_mail) = create_signal(false);
    let (copied, set_copied) = create_signal(false);

    let mail_ref = create_node_ref::<Div>();
    let copy_ref = create_node_ref::<Img>();

    let counter = create_rw_signal(0);
    let copy_loop = move || {
        if let Some(copy_img) = copy_ref.get() {
            if counter.get() > 5 {
                counter.set(0);
                set_copied.set(false);
                copy_img.set_src("/img/icon/copy.svg");
            }
            counter.update(|count| *count += 1);
        }
    };
    #[allow(unused)]
    let Pausable {
        is_active,
        pause,
        resume,
    } = use_interval_fn(copy_loop, 500);

    // If copied is false stop the loop
    create_effect(move |_| {
        if !copied.get() {
            pause();
        }
    });
    // If copied is true start the loop
    create_effect(move |_| {
        if copied.get() {
            resume();
        }
    });
    let mail_click = move |ev: MouseEvent| {
        ev.prevent_default();
        set_show_mail.set(true);
    };

    let click_copy = move |_| {
        let copy_img = copy_ref.get().expect("the Dom should be built");
        copy_img.set_src("/img/icon/kluk.svg");
        set_copied.set(true);
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
        <script>{copy_mail}</script>
        <a on:click=mail_click>
            <img class="contacts-img" src="/img/contact_icons/MAIL.svg"/>
        </a>
        <Show when=move || show_mail.get() fallback=|| {}>
            <div class="modal-overlay"></div>
            <div class="modal" node_ref=mail_ref>
                <button class="close-button" on:click=close_popup>
                    "close"
                </button>
                <a href=mailto>{EMAIL_ADDR}</a>
                <div class="copy-and-confirm">
                    <button class="copy-button" onclick="copy_mail()" on:click=click_copy>
                        <img src="/img/icon/copy.svg" class="copy-img" node_ref=copy_ref/>
                    </button>
                    <Show when=move || copied.get() fallback=|| {}>
                        <span class="span-copied">"copied"</span>
                    </Show>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn LinkWithModal(loc: LinkLocation) -> impl IntoView {
    let (href_target, src_target) = loc.process();
    let (visible, set_visible) = create_signal(false);
    let modal_ref = create_node_ref();

    let click_on_link = move |ev: MouseEvent| {
        ev.prevent_default();
        set_visible.set(true);
    };

    let take_me_to_link = move |_| {
        let _ = window().open_with_url(href_target);
    };

    let take_me_back = move |_| {
        set_visible.set(false);
    };

    create_effect(move |_| {
        leptos_use::on_click_outside(modal_ref, move |_| {
            set_visible.set(false);
        })
    });

    view! {
        <a href=href_target on:click=click_on_link>
            <img class="contacts-img" src=src_target/>
        </a>
        <Show when=move || visible.get() fallback=|| {}>
            <div class="modal-overlay"></div>
            <div class="link-modal" node_ref=modal_ref>
                <p>"You are now leaving the site!"</p>
                <div class="link-buttons">
                    <button class="yes-link-button" on:click=take_me_to_link>
                        "Ok"
                    </button>
                    <button class="no-link-button" on:click=take_me_back>
                        "I don't want to go yet."
                    </button>
                </div>
            </div>
        </Show>
    }
}

#[allow(dead_code)]
enum LinkLocation {
    Fb,
    TikTok,
    Ig,
}

impl LinkLocation {
    /// Returns a tuple in the form of: (HREF_TARGET, IMAGE_LOCATION)
    fn process(&self) -> (&'static str, &'static str) {
        match self {
            LinkLocation::Fb => ("https://www.facebook.com", "/img/contact_icons/FB.svg"),
            LinkLocation::TikTok => ("https://www.tiktok.com", "/img/contact_icons/TIKTOK.svg"),
            LinkLocation::Ig => ("https://www.instagram.com", "/img/contact_icons/insta.svg"),
        }
    }
}
