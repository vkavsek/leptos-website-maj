use leptos::{ev::MouseEvent, form::ActionForm, html::Div, prelude::*};
use leptos_meta::{Link, Title};
use leptos_use::use_element_hover;

#[allow(unused)]
const EMAIL_ADDR: &str = "kavsekmaj@gmail.com";

#[allow(dead_code)]
enum LinkLocation {
    Fb,
    TikTok,
    Ig,
    Yt,
    BandcampAut,
    AutRecords,
}

impl LinkLocation {
    /// Returns a tuple in the form of: (HREF_TARGET, IMAGE_LOCATION, size-x, size-y)
    fn process(&self) -> (&'static str, &'static str, i32, i32) {
        match self {
            LinkLocation::Fb => (
                "https://www.facebook.com/majkavsek",
                "/img/contact_icons/FB.svg",
                60,
                60,
            ),
            LinkLocation::TikTok => (
                "https://www.tiktok.com/@majkavsek?lang=en",
                "/img/contact_icons/TIKTOK.svg",
                60,
                61,
            ),
            LinkLocation::Ig => (
                "https://www.instagram.com/maj.kavsek/",
                "/img/contact_icons/insta.svg",
                179,
                174,
            ),
            LinkLocation::Yt => (
                "https://www.youtube.com/@MajKavsek",
                "/img/contact_icons/YT.svg",
                177,
                128,
            ),
            LinkLocation::BandcampAut => (
                "https://autrecords.bandcamp.com/album/galaterna",
                "/img/foreign-logos/bandcamp-fin.svg",
                256,
                256,
            ),
            LinkLocation::AutRecords => (
                "https://www.autrecords.com/",
                "/img/foreign-logos/aut-fin.svg",
                256,
                256,
            ),
        }
    }
}

// ###################################
// ->   COMPONENTS
// ###################################
#[component]
pub fn Home() -> impl IntoView {
    const FB: LinkLocation = LinkLocation::Fb;
    const TIKTOK: LinkLocation = LinkLocation::TikTok;
    const IG: LinkLocation = LinkLocation::Ig;
    const YT: LinkLocation = LinkLocation::Yt;
    view! {
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg"/>
        <Title text="Home"/>
        <div class="components" id="home-components">
            <div>
                <div class="title">
                    <h1 id="home-title">"Maj Kavšek"</h1>
                </div>
            </div>
            <AlbumPromo/>
            <div class="contents" id="home-bottom">
                <div class="all-contacts">
                    <div class="contacts-wrap">
                        <Mail/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=FB if_add_image=true/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=IG if_add_image=true/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=TIKTOK if_add_image=true/>
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=YT if_add_image=true/>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn AlbumPromo() -> impl IntoView {
    let center_ref: NodeRef<Div> = NodeRef::new();
    let is_hovered = use_element_hover(center_ref);
    let (get_id, set_id) = signal("home-animated");

    let dyn_id = move || {
        if is_hovered.get() {
            set_id.set("");
        }
        get_id.get()
    };

    view! {
        <div class="home-center" id=dyn_id node_ref=center_ref>
            <div id="album-img-title-wrap">
                <img src="/img/album_artwork_360p.webp" width="360" height="360" id="home-album-img" alt="Album Artwork"/>
                <h1 id="album-promo-title">"GALATERNA"</h1>
            </div>
            <div id="home-hide">
                <h3>"New Album Release!"</h3>
                <p id="home-album-text">
                    "A number of songs are available on YouTube, but you can check them out on the "
                    <a href="/media">"media page of this site."</a>
                </p>
                <p id="home-album-text">
                    "Buy a digital copy on "
                    <LinkWithModal
                        loc=LinkLocation::BandcampAut
                        if_add_image=false
                        opt_text="Bandcamp".to_string()
                    /> " or a hard copy on the "
                    <LinkWithModal
                        loc=LinkLocation::AutRecords
                        if_add_image=false
                        opt_text="label's website!".to_string()
                    />
                </p>
            </div>
            <div id="home-album-links">
                <LinkWithModal loc=LinkLocation::BandcampAut if_add_image=true/>
                <LinkWithModal loc=LinkLocation::AutRecords if_add_image=true/>
            </div>
        </div>
    }
}

#[component]
fn LinkWithModal(
    loc: LinkLocation,
    if_add_image: bool,
    #[prop(optional)] opt_text: Option<String>,
) -> impl IntoView {
    let (href_target, src_target, width, height) = loc.process();
    let (visible, set_visible) = signal(false);
    let modal_ref = NodeRef::new();

    let alt = match loc {
        LinkLocation::Fb => "A link to Facebook",
        LinkLocation::Ig => "A link to Instagram",
        LinkLocation::TikTok => "A link to TikTok",
        LinkLocation::Yt => "A link to YouTube",
        LinkLocation::AutRecords => "A link to the label company",
        LinkLocation::BandcampAut => "A link to label company's Bandcamp page",
    };

    let click_on_link = move |ev: MouseEvent| {
        ev.prevent_default();
        set_visible.set(true);
    };

    let open_link = move |_| {
        // Open in new tab
        let _ = window().open_with_url(href_target);
        set_visible.set(false);
    };

    let take_me_to_link = move |_| {
        // Navigate in the same tab
        let _ = window().location().set_href(href_target);
        set_visible.set(false);
    };

    let take_me_back = move |_| {
        set_visible.set(false);
    };

    Effect::new(move |_| {
        leptos_use::on_click_outside(modal_ref, move |_| {
            set_visible.set(false);
        })
    });

    view! {
        <a href=href_target on:click=click_on_link>
            {opt_text}
            // On each dispatch evaluates if it needs to display an icon, doesn't dynamically change!
            {if_add_image
                .then_some(
                    view! {
                        <img class="contacts-img" src=src_target alt=alt width=width height=height/>
                    },
                )}

        </a>
        <Show when=move || visible.get() fallback=|| {}>
            <div class="modal-overlay"></div>
            <div class="link-modal" node_ref=modal_ref>
                <p>"You are now leaving the site!"</p>
                <div>
                    <p style:font-size="0.7rem">" Going to: "</p>
                    <p style:font-size="0.7rem">{href_target}</p>
                </div>
                <div class="link-buttons">
                    <button class="yes-link-button" on:click=take_me_to_link>
                        "Ok"
                    </button>
                    <button class="yes-link-button" on:click=open_link>
                        "Open in new tab"
                    </button>
                    <button class="no-link-button" on:click=take_me_back>
                        "Back"
                    </button>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn Mail() -> impl IntoView {
    const EMAIL_PATTERN: &str = r".+@.+\..+";

    let (show_mail, set_show_mail) = signal(false);
    let mail_ref = NodeRef::new();

    let mail_click = move |ev: MouseEvent| {
        ev.prevent_default();
        set_show_mail.set(true);
    };

    let close_popup = move |_| {
        set_show_mail.set(false);
    };

    Effect::new(move |_| {
        leptos_use::on_click_outside(mail_ref, move |_| {
            set_show_mail.set(false);
        })
    });

    let mail_action = ServerAction::<SendMail>::new();

    // TODO: Improve this, does it have to be this complicated?
    let (control_input, set_control_input) = signal(0);
    let mail_visibility = move || {
        let version = mail_action.version().get();
        let mail_is_some_and_ok = mail_action
            .value()
            .get()
            .unwrap_or(Err(ServerFnError::ServerError(
                "No previous return Values".to_string(),
            )))
            .map(|_| true)
            .unwrap_or_else(|_| false);

        if version > control_input.get() && mail_is_some_and_ok {
            set_show_mail.set(false);
            set_control_input.set(version);
        }
        show_mail.get()
    };

    let display_mail_status = move || {
        if mail_action.pending().get() {
            Some(
                view! {
                    <div
                        style:display="flex"
                        style:width="100%"
                        style="justify-content: space-evenly"
                    >
                        <p>"Sending..."</p>
                        <span class="email-sending"></span>
                    </div>
                }
                .into_any(),
            )
        } else {
            let err = view! { <p>"Internal server error! Couldn't send the email, please try again later!"</p> }.into_any();
            let v = mail_action.value().get().map(|mail| match mail {
                Ok(_) => view! { <p>"Email sent!"</p> }.into_any(),
                Err(_) => err,
            });
            v
        }
    };

    view! {
        <button class="mail-container-button" on:click=mail_click title="Mail Button">
            <img
                class="contacts-img"
                alt="Send me an email"
                src="/img/contact_icons/MAIL.svg"
                width="62"
                height="38"
            />
        </button>
        <Show when=mail_visibility fallback=|| {}>
            <div class="modal-overlay"></div>
            <div class="modal" node_ref=mail_ref>
                <button class="close-button" on:click=close_popup title="Close Button">
                    <img
                        src="/img/icon/cross.svg"
                        alt="close"
                        style:width="1rem"
                        style:opacity="70%"
                    />
                </button>
                <span style:color="var(--maj-yel)">
                    "Contact for booking, teaching and collaborations."
                </span>
                // FIXME: whats up with ActionForm class do i have to wrap in a div?
                // class="email-form"
                <ActionForm action=mail_action>
                    <div class="form-div">
                        <label for="name" class="name-label">
                            "Enter your name: "
                        </label>
                        <input
                            name="name"
                            id="name"
                            type="text"
                            class="name-input"
                            placeholder="Your Name"
                        />
                    </div>
                    <div class="form-div">
                        <label for="email" class="email-label">
                            "Enter your email address: "
                        </label>
                        <input
                            name="email"
                            id="email"
                            class="email-input"
                            type="email"
                            pattern=EMAIL_PATTERN
                            placeholder="email@example.com"
                            required
                        />
                    </div>
                    <div class="form-div">
                        <label class="email-sub-label" for="email-subject">
                            "Subject: "
                        </label>
                        <input
                            class="email-sub-input"
                            name="email_subject"
                            id="email-subject"
                            type="text"
                            placeholder="Message subject"
                            required
                        />
                    </div>
                    <textarea
                        class="email-content"
                        name="email_content"
                        placeholder="Your message"
                        required
                    ></textarea>
                    <input class="submit-button" type="submit" value="Send"/>
                    {display_mail_status}
                </ActionForm>
            </div>
        </Show>
    }
}

// ###################################
// ->   SERVER
// ###################################
/// Takes (Name, Email, Email Subject, Email Content) as input and sends an email, returning errors
/// if any.
#[server(name = SendMail)]
async fn send_mail(
    name: String,
    email: String,
    email_subject: String,
    email_content: String,
) -> Result<(), ServerFnError> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };
    use tracing::{error, info};

    info!(name = %name, address = %email, subject = %email_subject, email_content = %email_content, "SENDING EMAIL");
    let email_a = Message::builder()
        .from("Info <info@majkavsek.com>".parse().unwrap())
        .to("Me <kavsekmaj@gmail.com>".parse().unwrap())
        .subject(email_subject.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Sender INFO:\n\n{:*>20}{name}\n{:*>20}{email}\n\n\nEmail CONTENT:\n\n{email_content}",
            "Name: ", "Email: "
        ))
        .map_err(<lettre::error::Error as Into<ServerFnError>>::into)?;

    let email_b = Message::builder()
        .from("Info <info@majkavsek.com>".parse().unwrap())
        .to("Me <vkavsek@gmail.com>".parse().unwrap())
        .subject(email_subject)
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Sender INFO:\n\n{:*>20}{name}\n{:*>20}{email}\n\n\nEmail CONTENT:\n\n{email_content}",
            "Name: ", "Email: "
        ))
        .map_err(<lettre::error::Error as Into<ServerFnError>>::into)?;

    let porkmail_pwd = std::env::var("PORKMAIL_PWD").unwrap_or_default();
    let creds = Credentials::new("info@majkavsek.com".to_owned(), porkmail_pwd.to_string());

    // Open a remote connection to porkbun
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.porkbun.com")?
            .credentials(creds)
            .build();

    // Send the email
    let res = mailer.send(email_a).await;
    let res_2 = mailer.send(email_b).await;

    if let Err(res_2) = res_2 {
        error!("{res_2}");
    }
    match res {
        Ok(ref response) => info!("Sent email: {:?}", response),
        Err(ref e) => error!("{e}"),
    }
    // Map the `Response` to `()` and convert Error.
    res.map(|_res| {}).map_err(|e| e.into())
}
