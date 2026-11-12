use crate::app::use_dialog_light_dismiss;
use leptos::{ev::MouseEvent, form::ActionForm, prelude::*};
use leptos_meta::{Link, Title};

#[cfg(feature = "ssr")]
use parking_lot::Mutex;
#[cfg(feature = "ssr")]
use secrecy::ExposeSecret;
#[cfg(feature = "ssr")]
use std::time::{Duration, Instant};

enum LinkLocation {
    Fb,
    TikTok,
    Ig,
    Yt,
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
            <div class="contents" id="home-bottom">
                <div class="all-contacts">
                    <div class="contacts-wrap">
                        <Mail />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=FB />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=IG />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=TIKTOK />
                    </div>
                    <div class="contacts-wrap">
                        <LinkWithModal loc=YT />
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn LinkWithModal(loc: LinkLocation) -> impl IntoView {
    let (href_target, src_target, width, height) = loc.process();
    let modal_ref = NodeRef::<leptos::html::Dialog>::new();
    use_dialog_light_dismiss(modal_ref);

    let alt = match loc {
        LinkLocation::Fb => "A link to Facebook",
        LinkLocation::Ig => "A link to Instagram",
        LinkLocation::TikTok => "A link to TikTok",
        LinkLocation::Yt => "A link to YouTube",
    };

    let click_on_link = move |ev: MouseEvent| {
        ev.prevent_default();
        if let Some(dialog) = modal_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    let open_link = move |_| {
        // Open in new tab
        let _ = window().open_with_url_and_target_and_features(href_target, "_blank", "noopener");
        if let Some(dialog) = modal_ref.get() {
            dialog.close();
        }
    };

    let take_me_to_link = move |_| {
        // Navigate in the same tab
        let _ = window().location().set_href(href_target);
        if let Some(dialog) = modal_ref.get() {
            dialog.close();
        }
    };

    let take_me_back = move |_| {
        if let Some(dialog) = modal_ref.get() {
            dialog.close();
        }
    };

    view! {
        <a href=href_target title=alt on:click=click_on_link>
            <img
                class="contacts-img"
                src=src_target
                alt=alt
                width=width
                height=height
            />

        </a>
            <dialog class="link-modal" node_ref=modal_ref aria-label="Open external website" closedby="any">
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
                    <button class="no-link-button" on:click=take_me_back autofocus=true>
                        "Back"
                    </button>
                </div>
            </dialog>
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
    let mail_ref = NodeRef::<leptos::html::Dialog>::new();
    let form_ref = NodeRef::<leptos::html::Form>::new();
    use_dialog_light_dismiss(mail_ref);

    let mail_click = move |ev: MouseEvent| {
        ev.prevent_default();
        if let Some(dialog) = mail_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    let close_popup = move |_| {
        if let Some(dialog) = mail_ref.get() {
            dialog.close();
        }
    };

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

    let (control_input, set_control_input) = signal(0);
    Effect::new(move |_| {
        let version = mail_action.version().get();
        let mail_is_some_and_ok = mail_action
            .value()
            .get()
            .is_some_and(|result| result.is_ok());

        if version > control_input.get() && mail_is_some_and_ok {
            if let Some(dialog) = mail_ref.get() {
                dialog.close();
            }
            if let Some(form) = form_ref.get() {
                form.reset();
            }
            set_control_input.set(version);
        }
    });

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
            <dialog class="modal" node_ref=mail_ref aria-label="Contact Maj Kavšek" closedby="any">
                <button class="close-button" on:click=close_popup aria-label="Close contact form">
                    <img
                        src="/img/icon/cross.svg"
                        alt=""
                        style:width="1rem"
                        style:opacity="70%"
                    />
                </button>
                <span style:color="var(--maj-yel)">
                    "Contact for booking, teaching and collaborations."
                </span>
                <ActionForm action=mail_action node_ref=form_ref attr:class="email-form">
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
                            autocomplete="name"
                            autofocus=true
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
                            autocomplete="email"
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
                        aria-label="Your message"
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
            </dialog>
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
fn reserve_mail_attempt(attempts: &Mutex<Vec<Instant>>, now: Instant) -> bool {
    let mut attempts = attempts.lock();
    attempts.retain(|attempt| now.saturating_duration_since(*attempt) < Duration::from_secs(3600));
    if attempts.len() >= 10 {
        return false;
    }
    attempts.push(now);
    true
}

/// Validates a contact message and reserves one of ten send attempts per rolling hour.
#[server(name = SendMail)]
async fn send_mail(
    name: String,
    email: String,
    email_subject: String,
    email_content: String,
) -> Result<(), ServerFnError> {
    use crate::server_config::ServerConfig;
    use axum::Extension;
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };
    use std::sync::Arc;
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
    let config = leptos_axum::extract::<Extension<Arc<ServerConfig>>>()
        .await
        .ok();
    let porkmail_pwd = match config
        .as_ref()
        .and_then(|config| config.porkmail_pwd.as_ref())
    {
        Some(password) => password,
        None => {
            error!("Contact email configuration is missing or empty");
            if let Some(response) = response {
                response.set_status(http::StatusCode::INTERNAL_SERVER_ERROR);
            }
            return Err(ServerFnError::new(
                "The contact form is unavailable. Please try again later.",
            ));
        }
    };
    if !reserve_mail_attempt(&MAIL_ATTEMPTS, Instant::now()) {
        if let Some(response) = response {
            response.set_status(http::StatusCode::TOO_MANY_REQUESTS);
        }
        return Err(ServerFnError::new(MAIL_RATE_LIMIT_MESSAGE));
    }

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

    let creds = Credentials::new(
        "info@majkavsek.com".to_owned(),
        porkmail_pwd.expose_secret().to_owned(),
    );

    // Open a remote connection to porkbun
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.porkbun.com")?
            .credentials(creds)
            .build();

    // Send the email
    let res = match tokio::time::timeout(Duration::from_secs(20), mailer.send(email)).await {
        Ok(result) => result.map(|_| ()).map_err(ServerFnError::from),
        Err(_) => Err(ServerFnError::new(
            "Email delivery timed out. Please try again later.",
        )),
    };

    match res {
        Ok(_) => info!(delivery = "sent", "Contact email delivery"),
        Err(_) => error!(delivery = "failed", "Contact email delivery"),
    }
    res
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
            assert!(reserve_mail_attempt(&attempts, start));
        }
        assert!(reserve_mail_attempt(
            &attempts,
            start + Duration::from_secs(1)
        ));
        assert!(!reserve_mail_attempt(
            &attempts,
            start + Duration::from_secs(3599)
        ));
        assert_eq!(attempts.lock().len(), 10);

        for _ in 0..9 {
            assert!(reserve_mail_attempt(
                &attempts,
                start + Duration::from_secs(3600)
            ));
        }
        assert!(!reserve_mail_attempt(
            &attempts,
            start + Duration::from_secs(3600)
        ));
        assert!(reserve_mail_attempt(
            &attempts,
            start + Duration::from_secs(3601)
        ));
        assert!(!reserve_mail_attempt(
            &attempts,
            start + Duration::from_secs(3601)
        ));
        assert_eq!(attempts.lock().len(), 10);
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
                        reserve_mail_attempt(&attempts, now)
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum()
        });
        assert_eq!(admitted, 10);
        assert_eq!(attempts.lock().len(), 10);
    }

    #[test]
    fn mail_rate_limit_retains_its_cap_after_a_panicking_lock_holder() {
        let now = Instant::now();
        let attempts = Mutex::new(vec![now; 9]);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = attempts.lock();
            guard.push(now);
            panic!("Simulate a failed limiter update");
        }));
        assert!(!reserve_mail_attempt(&attempts, now));
        assert_eq!(attempts.lock().len(), 10);
        assert!(reserve_mail_attempt(
            &attempts,
            now + Duration::from_secs(3600)
        ));
    }

    #[tokio::test]
    async fn mail_handler_rejects_invalid_unconfigured_and_limited_requests_before_smtp() {
        use crate::server_config::ServerConfig;
        use axum::{
            body::{to_bytes, Body},
            routing::post,
            Extension, Router,
        };
        use http::{
            header::{ACCEPT, CONTENT_TYPE},
            Request, StatusCode,
        };
        use leptos::server_fn::{error::FromServerFnError, ServerFn};
        use std::sync::Arc;
        use tower::ServiceExt;

        let app = Router::new().route(SendMail::PATH, post(leptos_axum::handle_server_fns));
        let app_with_password = |password: Option<&str>| {
            app.clone().layer(Extension(Arc::new(ServerConfig {
                app_origin: "http://localhost:8080".into(),
                data_dir: "/tmp/maj-mail-test".into(),
                google: None,
                porkmail_pwd: password
                    .map(|password| secrecy::SecretString::from(password.to_owned())),
            })))
        };
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
            assert!(MAIL_ATTEMPTS.lock().is_empty());
        }

        let valid_body =
            "name=&email=booking%40example.com&email_subject=Booking&email_content=Hello";
        for app in [app.clone(), app_with_password(None)] {
            let response = app.oneshot(request(valid_body)).await.unwrap();
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(MAIL_ATTEMPTS.lock().is_empty());
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            assert_eq!(
                ServerFnError::de(body),
                ServerFnError::new("The contact form is unavailable. Please try again later.")
            );
        }
        // The only configured request is rate-limited before it can contact SMTP.
        *MAIL_ATTEMPTS.lock() = vec![Instant::now(); 10];
        let response = app_with_password(Some("unused-test-password")).oneshot(request(
            "name=%20%20&email=%20booking%40example.com%20&email_subject=%20Booking%20&email_content=%20Hello%20"
        )).await.unwrap();
        let attempts = MAIL_ATTEMPTS.lock().len();
        MAIL_ATTEMPTS.lock().clear();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(attempts, 10);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(
            ServerFnError::de(body),
            ServerFnError::new(MAIL_RATE_LIMIT_MESSAGE)
        );
    }
}
