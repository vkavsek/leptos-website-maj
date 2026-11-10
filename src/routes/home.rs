use leptos::{ev::MouseEvent, form::ActionForm, html::Div, prelude::*};
use leptos_meta::{Link, Title};
use leptos_use::use_element_hover;

#[cfg(feature = "ssr")]
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[allow(unused)]
const EMAIL_ADDR: &str = "kavsekmaj@gmail.com";

#[allow(dead_code)]
enum LinkLocation {
    Fb,
    TikTok,
    Ig,
    Yt,
    BandcampGala,
    BandcampMinorFlaw,
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
            LinkLocation::BandcampGala => (
                "https://autrecords.bandcamp.com/album/galaterna",
                "/img/foreign-logos/bandcamp-fin.svg",
                256,
                256,
            ),
            LinkLocation::BandcampMinorFlaw => (
                "https://jazzcerkno.bandcamp.com/album/minor-flaw",
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
        <Link rel="icon" href="/img/trobenta.svg" type_="image/svg" />
        <Title text="Home" />
        <div class="components" id="home-components">
            <div class="title">
                <h1 id="home-title">"Maj Kavšek"</h1>
            </div>
            // <AlbumPromo/>
            <div class="contents" id="home-bottom">
                <div class="all-contacts">
                    <div class="contacts-wrap">
                        <Mail />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=FB if_add_image=true />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=IG if_add_image=true />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=TIKTOK if_add_image=true />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=YT if_add_image=true />
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn AlbumPromo() -> impl IntoView {
    let minorflaw_ref: NodeRef<Div> = NodeRef::new();
    let galaterna_ref: NodeRef<Div> = NodeRef::new();
    let is_hovered_minorf = use_element_hover(minorflaw_ref);
    let is_hovered_gala = use_element_hover(galaterna_ref);
    let (get_id, set_id) = signal("home-animated");

    let anim_id = move || {
        if is_hovered_minorf.get() {
            set_id.set("");
        }
        get_id.get()
    };

    let minorf_display = move || {
        if is_hovered_gala.get() {
            "none"
        } else {
            "grid"
        }
    };
    let gala_display = move || {
        if is_hovered_minorf.get() {
            "none"
        } else {
            "grid"
        }
    };

    view! {
        <div class="home-container">
            <div
                class="home-center minorflaw-promo"
                id=anim_id
                node_ref=minorflaw_ref
                style:display=minorf_display
            >
                <div id="album-img-title-wrap">
                    // TODO:
                    <img
                        src="/img/MINORFLAW-01_600p.webp"
                        width="600"
                        height="600"
                        id="home-album-img"
                        alt="Album Artwork"
                    />
                    <h1 id="album-promo-title">"MINOR FLAW"</h1>
                </div>
                <div id="home-hide">
                    <h3>"New Album Release!"</h3>
                    <p id="home-album-text">
                        "Listen to the album promo here, or find individual songs on YouTube. For more content, visit our "
                        <a href="/media">"media page."</a>
                    </p>
                    <p id="home-album-text">
                        "Buy a digital copy on "
                        <LinkWithModal
                            loc=LinkLocation::BandcampMinorFlaw
                            if_add_image=false
                            opt_text="Bandcamp".to_string()
                        /> "."
                    </p>
                </div>
                <div id="home-album-links">
                    <LinkWithModal loc=LinkLocation::BandcampMinorFlaw if_add_image=true />
                </div>
            </div>
            <div
                class="home-center galaterna-promo"
                node_ref=galaterna_ref
                style:display=gala_display
            >
                <div id="album-img-title-wrap">
                    <img
                        src="/img/album_artwork_360p.webp"
                        width="360"
                        height="360"
                        id="home-album-img"
                        alt="Album Artwork"
                    />
                    <h1 id="album-promo-title">"GALATERNA"</h1>
                </div>
                <div id="home-hide">
                    <h3>"Galaterna (2024)"</h3>
                    <p id="home-album-text">
                        "Buy a digital copy on "
                        <LinkWithModal
                            loc=LinkLocation::BandcampGala
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
                    <LinkWithModal loc=LinkLocation::BandcampGala if_add_image=true />
                    <LinkWithModal loc=LinkLocation::AutRecords if_add_image=true />
                </div>
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
        LinkLocation::BandcampGala => {
            "A link to the digital release of Galaterna, hosted on Bandcamp"
        }
        LinkLocation::BandcampMinorFlaw => {
            "A link to the digital release of Minor Flaw, hosted on Bandcamp"
        }
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
        <a href=href_target title=alt on:click=click_on_link>
            {opt_text}
            // On each dispatch evaluates if it needs to display an icon, doesn't dynamically change!
            {if_add_image
                .then_some(
                    view! {
                        <img
                            class="contacts-img"
                            src=src_target
                            alt=alt
                            width=width
                            height=height
                        />
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

const MAX_NAME_LENGTH: usize = 100;
const MAX_EMAIL_LENGTH: usize = 254;
const MAX_SUBJECT_LENGTH: usize = 200;
const MAX_MESSAGE_LENGTH: usize = 10_000;
const MAIL_RATE_LIMIT_MESSAGE: &str =
    "The contact form has reached its hourly sending limit. Please try again later.";

#[component]
fn Mail() -> impl IntoView {
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
    let mail_error_ref = NodeRef::<leptos::html::P>::new();

    // Keep server errors visible inside the scrollable dialog.
    Effect::new(move |_| {
        if mail_action
            .value()
            .get()
            .is_some_and(|result| result.is_err())
        {
            if let Some(error) = mail_error_ref.get() {
                error.scroll_into_view_with_bool(false);
            }
        }
    });

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
                        role="status"
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
            mail_action.value().get().map(|mail| match mail {
                Ok(_) => view! { <p role="status">"Email sent!"</p> }.into_any(),
                Err(error) => {
                    let message = match error {
                        ServerFnError::Args(message) => message,
                        ServerFnError::ServerError(message) if message == MAIL_RATE_LIMIT_MESSAGE => message,
                        _ => "Internal server error! Couldn't send the email, please try again later!".to_owned(),
                    };
                    view! { <p role="alert" node_ref=mail_error_ref>{message}</p> }.into_any()
                },
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
                <ActionForm action=mail_action attr:class="email-form">
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
                            maxlength=MAX_NAME_LENGTH
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
                            maxlength=MAX_EMAIL_LENGTH
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
                            maxlength=MAX_SUBJECT_LENGTH
                            required
                        />
                    </div>
                    <textarea
                        class="email-content"
                        name="email_content"
                        placeholder="Your message"
                        maxlength=MAX_MESSAGE_LENGTH
                        required
                    ></textarea>
                    <input
                        class="submit-button"
                        type="submit"
                        value="Send"
                        disabled=move || mail_action.pending().get()
                    />
                    {display_mail_status}
                </ActionForm>
            </div>
        </Show>
    }
}

// ###################################
// ->   SERVER
// ###################################
#[cfg(feature = "ssr")]
// shortcut: allowance resets per process; use shared storage for a durable cap across replicas.
static MAIL_ATTEMPTS: Mutex<Vec<Instant>> = Mutex::new(Vec::new());

#[cfg(feature = "ssr")]
fn validate_mail(
    name: &str,
    email: &str,
    subject: &str,
    message: &str,
) -> Result<(), ServerFnError> {
    if name.encode_utf16().count() > MAX_NAME_LENGTH || name.chars().any(char::is_control) {
        return Err(ServerFnError::Args(format!(
            "Name must be a single line of at most {MAX_NAME_LENGTH} characters."
        )));
    }
    if email.is_empty()
        || email.len() > MAX_EMAIL_LENGTH
        || email.chars().any(char::is_control)
        || email.parse::<lettre::Address>().is_err()
    {
        return Err(ServerFnError::Args(
            "Email address is invalid or too long.".to_owned(),
        ));
    }
    if subject.trim().is_empty()
        || subject.encode_utf16().count() > MAX_SUBJECT_LENGTH
        || subject.chars().any(char::is_control)
    {
        return Err(ServerFnError::Args(format!(
            "Enter a single-line subject of 1 to {MAX_SUBJECT_LENGTH} characters."
        )));
    }
    if message.trim().is_empty()
        || message.encode_utf16().count() > MAX_MESSAGE_LENGTH
        || message
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\r' | '\n' | '\t'))
    {
        return Err(ServerFnError::Args(format!(
            "Enter a message of 1 to {MAX_MESSAGE_LENGTH} characters, using normal text, line breaks or tabs."
        )));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
fn reserve_mail_attempt(
    attempts: &Mutex<Vec<Instant>>,
    now: Instant,
) -> Result<bool, ServerFnError> {
    let mut attempts = attempts
        .lock()
        .map_err(|_| ServerFnError::new("The email service is temporarily unavailable."))?;
    attempts.retain(|attempt| now.saturating_duration_since(*attempt) < Duration::from_secs(3600));
    if attempts.len() >= 10 {
        return Ok(false);
    }
    attempts.push(now);
    Ok(true)
}

/// Validates a contact message and reserves one of ten send attempts per rolling hour.
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

    let name = name.trim();
    let email = email.trim();
    let email_subject = email_subject.trim();
    let email_content = email_content.trim();
    let response = use_context::<leptos_axum::ResponseOptions>();

    if let Err(error) = validate_mail(name, email, email_subject, email_content) {
        if let Some(response) = response {
            response.set_status(http::StatusCode::BAD_REQUEST);
        }
        return Err(error);
    }
    if !reserve_mail_attempt(&MAIL_ATTEMPTS, Instant::now())? {
        if let Some(response) = response {
            response.set_status(http::StatusCode::TOO_MANY_REQUESTS);
        }
        return Err(ServerFnError::new(MAIL_RATE_LIMIT_MESSAGE));
    }

    info!(name = %name, address = %email, subject = %email_subject, email_content = %email_content, "SENDING EMAIL");
    let email = Message::builder()
        .from("Info <info@majkavsek.com>".parse().unwrap())
        .to("Maj <kavsekmaj@gmail.com>".parse().unwrap())
        .cc("Vid <vkavsek@gmail.com>".parse().unwrap())
        .subject(email_subject)
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Sender INFO:\n\n{:*>20}{name}\n{:*>20}{email}\n\n\nEmail CONTENT:\n\n{email_content}",
            "Name: ", "Email: "
        ))
        .map_err(<lettre::error::Error as Into<ServerFnError>>::into)?;

    // TODO: add secrecy
    let porkmail_pwd = std::env::var("PORKMAIL_PWD").unwrap_or_default();
    let creds = Credentials::new("info@majkavsek.com".to_owned(), porkmail_pwd.to_string());

    // Open a remote connection to porkbun
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.porkbun.com")?
            .credentials(creds)
            .build();

    // Send the email
    let res = mailer.send(email).await;

    match res {
        Ok(ref response) => info!("Sent email: {:?}", response),
        Err(ref e) => error!("{e}"),
    }
    // Map the `Response` to `()` and convert Error.
    res.map(|_res| {}).map_err(|e| e.into())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn mail_validation_accepts_unicode_and_boundary_lengths() {
        assert!(validate_mail(
            "Žan Kavšek",
            "booking+concert@example.com",
            "Koncert v Ljubljani",
            "Pozdravljeni,\r\n\tSe lahko dogovorimo za koncert?\nHvala!"
        )
        .is_ok());
        assert!(validate_mail("", "booking@example.com", "Booking", "Hello").is_ok());

        let email = format!(
            "{}@{}.{}.{}",
            "a".repeat(64),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(61)
        );
        assert_eq!(email.len(), 254);
        assert!(validate_mail(
            &"🎺".repeat(50),
            &email,
            &"š".repeat(200),
            &"🎺".repeat(5000)
        )
        .is_ok());
    }

    #[test]
    fn mail_validation_rejects_missing_malformed_and_control_character_input() {
        for (name, email, subject, message) in [
            ("", "", "Booking", "Hello"),
            ("", " ", "Booking", "Hello"),
            ("", "not-an-email", "Booking", "Hello"),
            ("", "Name <booking@example.com>", "Booking", "Hello"),
            ("", "booking@example.com\n", "Booking", "Hello"),
            ("", "booking@example.com", "", "Hello"),
            ("", "booking@example.com", " \t ", "Hello"),
            ("", "booking@example.com", "Booking", ""),
            ("", "booking@example.com", "Booking", " \r\n\t "),
            ("A\nB", "booking@example.com", "Booking", "Hello"),
            (
                "",
                "booking@example.com",
                "Booking\r\nBcc: other@example.com",
                "Hello",
            ),
            ("", "booking@example.com", "Booking", "Hello\0"),
            ("", "booking@example.com", "Booking", "Hello\u{7f}"),
        ] {
            assert!(matches!(
                validate_mail(name, email, subject, message),
                Err(ServerFnError::Args(_))
            ));
        }
    }

    #[test]
    fn mail_validation_rejects_overlong_fields() {
        assert!(
            validate_mail(&"n".repeat(101), "booking@example.com", "Booking", "Hello").is_err()
        );
        assert!(
            validate_mail(&"🎺".repeat(51), "booking@example.com", "Booking", "Hello").is_err()
        );
        let email = format!(
            "{}@{}.{}.{}",
            "a".repeat(64),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(62)
        );
        assert_eq!(email.len(), 255);
        assert!(validate_mail("", &email, "Booking", "Hello").is_err());
        assert!(validate_mail("", "booking@example.com", &"s".repeat(201), "Hello").is_err());
        assert!(validate_mail("", "booking@example.com", "Booking", &"m".repeat(10_001)).is_err());
    }

    #[test]
    fn mail_rate_limit_uses_a_rolling_hour_and_bounds_storage() {
        let attempts = Mutex::new(Vec::new());
        let start = Instant::now();
        for _ in 0..9 {
            assert!(reserve_mail_attempt(&attempts, start).unwrap());
        }
        assert!(reserve_mail_attempt(&attempts, start + Duration::from_secs(1)).unwrap());
        assert!(!reserve_mail_attempt(&attempts, start + Duration::from_secs(3599)).unwrap());
        assert_eq!(attempts.lock().unwrap().len(), 10);

        for _ in 0..9 {
            assert!(reserve_mail_attempt(&attempts, start + Duration::from_secs(3600)).unwrap());
        }
        assert!(!reserve_mail_attempt(&attempts, start + Duration::from_secs(3600)).unwrap());
        assert!(reserve_mail_attempt(&attempts, start + Duration::from_secs(3601)).unwrap());
        assert!(!reserve_mail_attempt(&attempts, start + Duration::from_secs(3601)).unwrap());
        assert_eq!(attempts.lock().unwrap().len(), 10);
    }

    #[test]
    fn mail_rate_limit_is_atomic_across_threads() {
        let attempts = Mutex::new(Vec::new());
        let barrier = std::sync::Barrier::new(20);
        let now = Instant::now();
        let admitted: usize = std::thread::scope(|scope| {
            let workers = (0..20)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        reserve_mail_attempt(&attempts, now).unwrap()
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum()
        });
        assert_eq!(admitted, 10);
        assert_eq!(attempts.lock().unwrap().len(), 10);
    }

    #[test]
    fn mail_rate_limit_rejects_sending_if_the_mutex_is_poisoned() {
        let attempts = Mutex::new(Vec::new());
        let _ = std::panic::catch_unwind(|| {
            let _guard = attempts.lock().unwrap();
            panic!("Simulate a failed limiter update");
        });
        assert!(reserve_mail_attempt(&attempts, Instant::now()).is_err());
    }

    #[tokio::test]
    async fn mail_handler_rejects_invalid_and_limited_requests_before_smtp() {
        use axum::{
            body::{to_bytes, Body},
            routing::post,
            Router,
        };
        use http::{
            header::{ACCEPT, CONTENT_TYPE},
            Request, StatusCode,
        };
        use leptos::server_fn::{error::FromServerFnError, ServerFn};
        use tower::ServiceExt;

        let app = Router::new().route(SendMail::PATH, post(leptos_axum::handle_server_fns));
        let request = |body: &str| {
            Request::post(SendMail::PATH)
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(ACCEPT, "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap()
        };
        for body in [
            "name=&email=bad&email_subject=Booking&email_content=Hello",
            "name=&email=booking%40example.com&email_subject=%20%09&email_content=Hello",
            "name=&email=booking%40example.com&email_subject=Booking&email_content=%20%0A%09",
        ] {
            let response = app.clone().oneshot(request(body)).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            let error: ServerFnError = ServerFnError::de(body);
            assert!(matches!(error, ServerFnError::Args(_)));
            assert!(MAIL_ATTEMPTS.lock().unwrap().is_empty());
        }

        *MAIL_ATTEMPTS.lock().unwrap() = vec![Instant::now(); 10];
        let response = app.oneshot(request(
            "name=%20%20&email=%20booking%40example.com%20&email_subject=%20Booking%20&email_content=%20Hello%20"
        )).await.unwrap();
        let attempts = MAIL_ATTEMPTS.lock().unwrap().len();
        MAIL_ATTEMPTS.lock().unwrap().clear();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(attempts, 10);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(
            ServerFnError::de(body),
            ServerFnError::new(MAIL_RATE_LIMIT_MESSAGE)
        );
    }
}
