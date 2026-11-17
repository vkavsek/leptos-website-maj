//! Public privacy information; reading it never requires an administrator session.

use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Privacy() -> impl IntoView {
    view! {
        <Title text="Privacy policy" />
        <div class="privacy-page">
            // The fixed application shell needs a keyboard-scrollable reading area.
            <article class="privacy-policy" aria-labelledby="privacy-title" tabindex="0">
                <header>
                    <h1 id="privacy-title">"Privacy policy"</h1>
                    <p>"Maj Kavšek’s Portfolio — majkavsek.com"</p>
                    <p>"Last updated: "<time datetime="2026-11-17">"17 November 2026"</time></p>
                </header>

                <h2>"About this website"</h2>
                <p>
                    "This is Maj Kavšek’s portfolio for music, photographs, live shows and videos. "
                    "You can browse the public pages without signing in. Google sign-in is used "
                    "only for the private editor, where explicitly approved administrators can "
                    "add or remove gallery photos, shows and YouTube videos."
                </p>

                <h2>"Google sign-in and administrator information"</h2>
                <p>
                    "When you choose Google sign-in, Google authenticates your account and "
                    "supplies an identity token. We use its verified issuer and account identifier "
                    "to check whether you are approved for administration. The sign-in request "
                    "uses only the openid permission: it does not request your Google email "
                    "address, profile information, Gmail, Drive, Photos or YouTube account data."
                </p>
                <p>
                    "For approved administrators, the website stores the Google issuer and "
                    "account identifier, an operator-assigned label, approval status and session "
                    "revocation information. Local browser sessions store the administrator’s "
                    "local ID, security tokens and activity timestamps. Approval is an explicit "
                    "decision by the site operator; signing in with Google does not grant access."
                </p>
                <p>
                    "Temporary sign-in records are used to protect the login flow. An unapproved "
                    "account’s verified identifier is temporarily available to that browser for "
                    "requesting approval. Google access and refresh tokens are not retained. "
                    "Google identity information is used for sign-in and access control, not "
                    "advertising or public display."
                </p>

                <h2>"Cookies"</h2>
                <p>
                    "The administrator sign-in flow uses an essential cookie containing an "
                    "opaque local session ID. Authentication details stay on the server. "
                    "Administrator access expires after 30 minutes without activity or eight "
                    "hours after sign-in, whichever comes first. Blocking this cookie prevents "
                    "administrator sign-in but does not require you to sign in to browse."
                </p>

                <h2>"Contact messages"</h2>
                <p>
                    "If you use the contact form, you provide your email address, subject and "
                    "message, and optionally your name. This is information you enter yourself, "
                    "not information obtained through Google sign-in. The message is sent by "
                    "email to Maj and the site administrator so they can respond. Contact "
                    "messages are not stored in the website’s application database."
                </p>

                <h2>"Hosting, email and external services"</h2>
                <p>
                    "Fly.io hosts the website and its private database and gallery files. "
                    "Requests necessarily send connection information, including an IP address, "
                    "to the hosting service. Application logs record request information such "
                    "as requested URLs, response status, timing and operational events for "
                    "reliability and security. Avoid putting private information in URLs."
                </p>
                <p>
                    "Contact messages are delivered through Porkbun’s email service and the "
                    "recipients’ email providers. Google processes sign-in requests under its "
                    "own privacy policy. The Media page embeds YouTube videos using "
                    "youtube-nocookie.com; loading or interacting with an embedded player "
                    "connects your browser to YouTube. Social links connect you to their "
                    "respective services when followed."
                </p>
                <p>
                    "Read "<a href="https://policies.google.com/privacy">"Google’s privacy policy"</a>
                    ", "<a href="https://fly.io/legal/privacy-policy/">"Fly.io’s privacy policy"</a>
                    " and "<a href="https://porkbun.com/legal/agreement/privacy_policy">"Porkbun’s privacy policy"</a>
                    " for information about their processing."
                </p>

                <h2>"Retention and deletion"</h2>
                <p>
                    "Temporary login attempts and the identity display for unapproved accounts "
                    "expire after five minutes. Expired login records and browser sessions are "
                    "normally cleaned every five minutes while the server is running; expiry "
                    "and physical deletion are separate, and cleanup can be delayed by outages."
                </p>
                <p>
                    "Administrator approval records have no automatic deletion timer. "
                    "Disabling an administrator invalidates access but keeps the identity "
                    "record; removal of that record is a separate operator action. Published "
                    "photos, shows and videos remain until removed by an administrator. "
                    "Failed file cleanup can leave private, unlisted copies for later removal."
                </p>
                <p>
                    "The site operator has not configured fixed deletion periods for delivered "
                    "contact emails or backup copies. They remain until manually deleted or "
                    "removed by the relevant provider. Hosting log retention is controlled by "
                    "Fly.io; no separate log retention schedule has been configured by the site "
                    "operator. Removing live data does not immediately remove historical "
                    "backup copies, which remain until the backup is expired or deleted."
                </p>

                <h2>"Privacy questions and deletion requests"</h2>
                <p>
                    "Email Maj Kavšek at "<span>"info{at}majkavsek.com"</span>
                    " (replace {at} with @), or use the "<a href="/">"homepage contact form"</a>
                    ", to ask about this policy or request access, correction or deletion of "
                    "information associated with you. Describe the information concerned; "
                    "do not send passwords, session cookies or Google authentication tokens. "
                    "We may need to confirm ownership before acting on a request."
                </p>
                <p>
                    "You can also remove this website’s Google connection in your "
                    <a href="https://myaccount.google.com/connections">"Google Account settings"</a>
                    ". That does not by itself erase the website’s local approval records or "
                    "backups; contact us to request their removal."
                </p>

                <footer><a href="/">"Back to homepage"</a></footer>
            </article>
        </div>
    }
}
