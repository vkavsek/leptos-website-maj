use leptos::{
    ev::{MouseEvent, SubmitEvent},
    html::{Div, Input, Textarea},
    *,
};
use leptos_meta::{Link, Title};

#[allow(unused)]
const EMAIL_ADDR: &str = "kavsekmaj@gmail.com";

#[allow(dead_code)]
enum LinkLocation {
    Fb,
    TikTok,
    Ig,
    Yt,
}

impl LinkLocation {
    /// Returns a tuple in the form of: (HREF_TARGET, IMAGE_LOCATION)
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
        }
    }
}

#[component]
pub fn Home() -> impl IntoView {
    const FB: LinkLocation = LinkLocation::Fb;
    const TIKTOK: LinkLocation = LinkLocation::TikTok;
    const IG: LinkLocation = LinkLocation::Ig;
    const YT: LinkLocation = LinkLocation::Yt;
    view! {
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg"/>
        <Title text="Home"/>
        <footer class="components" id="home-components">
            <div>
                <div class="title">
                    <h1 id="home-title">"Maj Kavšek"</h1>
                </div>
                // <p  style="font-size: 1.25rem; font-weight: 700">"Trumpeter & composer"</p>
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
                    <div class="contacts-wrap">
                        <LinkWithModal loc=YT/>
                    </div>
                </div>
            </div>
        </footer>
    }
}
#[component]
fn LinkWithModal(loc: LinkLocation) -> impl IntoView {
    let (href_target, src_target, width, height) = loc.process();
    let (visible, set_visible) = create_signal(false);
    let modal_ref = create_node_ref();

    let alt = match loc {
        LinkLocation::Fb => "A link to Facebook",
        LinkLocation::Ig => "A link to Instagram",
        LinkLocation::TikTok => "A link to TikTok",
        LinkLocation::Yt => "A link to YouTube",
    };

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
            <img class="contacts-img" src=src_target alt=alt width=width height=height/>
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

#[component]
fn Mail() -> impl IntoView {
    let (show_mail, set_show_mail) = create_signal(false);

    let mail_ref = create_node_ref::<Div>();
    let name_ref = create_node_ref::<Input>();
    let email_ref = create_node_ref::<Input>();
    let email_sub_ref = create_node_ref::<Input>();
    let email_cont_ref = create_node_ref::<Textarea>();

    const EMAIL_PATTERN: &str = r".+@.+\..+";

    let mail_click = move |ev: MouseEvent| {
        ev.prevent_default();
        set_show_mail.set(true);
    };

    let close_popup = move |_| {
        set_show_mail.set(false);
    };

    create_effect(move |_| {
        leptos_use::on_click_outside(mail_ref, move |_| {
            set_show_mail.set(false);
        })
    });

    let mail_action =
        create_action(|input: &(String, String, String, String)| send_mail(input.clone()));

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let name = name_ref
            .get()
            .expect("the DOM should be built by now!")
            .value();
        let email = email_ref.get().unwrap().value();
        let email_sub = email_sub_ref.get().unwrap().value();
        let email_cont = email_cont_ref.get().unwrap().value();

        // TODO: content validation?
        // No need to check if the Strings are empty, <form> does that for us.

        mail_action.dispatch((name, email, email_sub, email_cont));
    };

    // TODO: Improve this, does it have to be this complicated?
    let (control_input, set_control_input) = create_signal(0);
    let mail_visibility = move || {
        let version = mail_action.version().get();
        let mail_is_some_and_ok = mail_action
            .value()
            .get()
            .unwrap_or(Err(ServerFnError::ServerError(String::new())))
            .map(|_| true)
            .unwrap_or_else(|_| false);

        if version > control_input.get() && mail_is_some_and_ok {
            log::info!("Closing email popup, email sent sucessfully!");
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
                .into_view(),
            )
        } else {
            let err = view! { <p>"Internal server error! Couldn't send the email, please try again later!"</p> }.into_view();
            mail_action.value().get().map(|mail| match mail {
                Ok(_) => view! { <p>"Email sent!"</p> }.into_view(),
                Err(_) => err,
            })
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
                <form class="email-form" on:submit=on_submit>
                    <div class="form-div">
                        <label for="name" class="name-label">
                            "Enter your name: "
                        </label>
                        <input
                            name="name"
                            id="name"
                            type="text"
                            class="name-input"
                            node_ref=name_ref
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
                            node_ref=email_ref
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
                            name="email-subject"
                            id="email-subject"
                            type="text"
                            node_ref=email_sub_ref
                            placeholder="Message subject"
                            required
                        />
                    </div>
                    <textarea
                        class="email-content"
                        name="email-content"
                        node_ref=email_cont_ref
                        placeholder="Your message"
                        required
                    ></textarea>
                    <input class="submit-button" type="submit" value="Send"/>
                    {display_mail_status}
                </form>
            </div>
        </Show>
    }
}

/// Takes (Name, Email, Email Subject, Email Content) as input and sends an email, returning errors
/// if any.
#[server(SendMail)]
async fn send_mail(input: (String, String, String, String)) -> Result<(), ServerFnError> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    let (name, email, email_sub, email_cont) = input;

    let email_a = Message::builder()
        .from("Info <info@majkavsek.com>".parse().unwrap())
        .to("Me <kavsekmaj@gmail.com>".parse().unwrap())
        .subject(email_sub.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Sender INFO:\n\n{:*>20}{name}\n{:*>20}{email}\n\n\nEmail CONTENT:\n\n{email_cont}",
            "Name: ", "Email: "
        ))
        .map_err(<lettre::error::Error as Into<ServerFnError>>::into)?;

    let email_b = Message::builder()
        .from("Info <info@majkavsek.com>".parse().unwrap())
        .to("Me <vkavsek@gmail.com>".parse().unwrap())
        .subject(email_sub)
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Sender INFO:\n\n{:*>20}{name}\n{:*>20}{email}\n\n\nEmail CONTENT:\n\n{email_cont}",
            "Name: ", "Email: "
        ))
        .map_err(<lettre::error::Error as Into<ServerFnError>>::into)?;

    let porkmail_pwd = std::env::var("PORKMAIL_PWD").unwrap_or_default();
    let creds = Credentials::new("info@majkavsek.com".to_owned(), porkmail_pwd);

    // Open a remote connection to porkbun
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.porkbun.com")?
            .credentials(creds)
            .build();

    // Send the email
    let res = mailer.send(email_a).await;
    // Ignore errors.
    let _ = mailer.send(email_b).await;

    match res {
        Ok(ref response) => log::info!("Sent email: {:?}", response),
        Err(ref e) => log::error!("Error: {}", e),
    }
    res.map(|_res| {}).map_err(|e| e.into())
}
