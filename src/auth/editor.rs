//! Native content forms. The first multipart field is verified CSRF state before file data.
//! POST/redirect/GET reloads the catalog; mutations never depend on browser JavaScript.

use super::{
    login::{login_failure, page},
    AuthError,
};
use crate::{
    content::{self, gallery, ContentError},
    database::DatabaseManager,
    server_config::ServerConfig,
};
use axum::{
    extract::{multipart::Field, DefaultBodyLimit, FromRequest, Multipart, Request},
    response::{IntoResponse, Redirect, Response},
    routing::post,
    Extension, Router,
};
use http::{HeaderValue, StatusCode};
use leptos::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tower_sessions::Session;

// Bound receiving/decoding memory, including work that continues after a client disconnects.
static PROCESSING: Semaphore = Semaphore::const_new(1);

pub(super) fn routes() -> Router<leptos::config::LeptosOptions> {
    Router::new()
        .route("/admin/content", post(submit))
        .layer(DefaultBodyLimit::max(gallery::UPLOAD_BYTES + 16 * 1024))
}

/// Validated editor selection; normal links work without scripts and with browser history.
#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Section {
    #[default]
    Gallery,
    Shows,
    Videos,
}

/// Render current records and explicit removal confirmation without exposing stored filenames.
pub(super) async fn content(
    database: &DatabaseManager,
    csrf: &str,
    section: Section,
) -> Result<impl IntoView, AuthError> {
    // Render and query only the selected catalog, including imported and older records.
    let photos = if section == Section::Gallery {
        database.gallery().await?
    } else {
        Vec::new()
    };
    let shows = if section == Section::Shows {
        database.shows().await?
    } else {
        Vec::new()
    };
    let videos = if section == Section::Videos {
        database.videos().await?
    } else {
        Vec::new()
    };
    let photo_count = photos.len();
    let show_count = shows.len();
    let video_count = videos.len();

    let gallery_items = photos
        .into_iter()
        .map(|photo| {
            let url = crate::routes::about::gallery_image_url(&photo.public_name);
            let thumbnail = format!("{}.webp", url.replace("/about_pics/", "/about_thumbnails/"));
            let remove = removal("gallery", photo.id, &photo.public_name, csrf);
            view! {
                <li>
                    <a href=url>
                        <img
                            src=thumbnail
                            width="480"
                            height="270"
                            alt=photo.public_name.clone()
                            loading="lazy"
                        />
                    </a>
                    <p>{photo.public_name}</p>
                    {remove}
                </li>
            }
        })
        .collect_view();

    let show_items = shows
        .into_iter()
        .map(|show| {
            let label = format!(
                "{} — {} {} {}",
                show.date,
                show.name.unwrap_or_default(),
                show.club.unwrap_or_default(),
                show.location.unwrap_or_default()
            );
            let remove = removal("shows", show.id, &label, csrf);
            view! {
                <li>
                    <p>{label}</p>
                    {remove}
                </li>
            }
        })
        .collect_view();

    let video_items = videos
        .into_iter()
        .map(|video| {
            let remove = removal("videos", video.id, &video.video_id, csrf);
            let url = format!("https://www.youtube.com/watch?v={}", video.video_id);
            view! {
                <li>
                    <a href=url>{video.video_id}</a>
                    {remove}
                </li>
            }
        })
        .collect_view();

    Ok(view! {
        <nav aria-label="Content editors">
            <a
                href="/admin?section=gallery"
                aria-current=(section == Section::Gallery).then_some("page")
            >
                "Gallery"
            </a>
            " · "
            <a
                href="/admin?section=shows"
                aria-current=(section == Section::Shows).then_some("page")
            >
                "Shows"
            </a>
            " · "
            <a
                href="/admin?section=videos"
                aria-current=(section == Section::Videos).then_some("page")
            >
                "YouTube videos"
            </a>
        </nav>
        <p>
            "Changes appear publicly on the next page load. Removal is permanent; confirm the item before removing it."
        </p>
        {(section == Section::Gallery)
            .then(|| {
                view! {
                    <section id="gallery">
                        <h2 id="gallery-heading">"Gallery"</h2>
                        <form
                            action="/admin/content"
                            method="post"
                            enctype="multipart/form-data"
                            aria-labelledby="gallery-heading"
                        >
                            {fields(csrf, "gallery", "add")}
                            <label for="photo">"Photo (required)"</label>
                            <p id="photo-help">
                                "JPEG, PNG or WebP; up to 10 MiB, 12 megapixels and 6000 pixels per side. Use letters, numbers, spaces, underscores or hyphens in the filename. Spaces become underscores and letters become lowercase. Duplicate names are rejected. JPEGs stay JPEG; other photos become PNG."
                            </p>
                            <input
                                type="file"
                                id="photo"
                                name="photo"
                                accept="image/jpeg,image/png,image/webp"
                                required
                                aria-describedby="photo-help"
                            />
                            <button type="submit">"Add photo"</button>
                        </form>
                        <h3>"Published photos ("{photo_count}")"</h3>
                        {(photo_count == 0).then(|| view! { <p>"No published photos."</p> })}
                        <ul>{gallery_items}</ul>
                    </section>
                }
            })}
        {(section == Section::Shows)
            .then(|| {
                view! {
                    <section id="shows">
                        <h2 id="shows-heading">"Shows"</h2>
                        <form
                            action="/admin/content"
                            method="post"
                            enctype="multipart/form-data"
                            aria-labelledby="shows-heading"
                        >
                            {fields(csrf, "shows", "add")}
                            <label for="date">"Date (required)"</label>
                            <input
                                type="date"
                                id="date"
                                name="date"
                                min="0001-01-01"
                                max="9999-12-31"
                                required
                            />
                            <label for="name">"Event name (optional)"</label>
                            <input id="name" name="name" maxlength="500" />
                            <label for="club">"Venue (optional)"</label>
                            <input id="club" name="club" maxlength="500" />
                            <label for="location">"Location (optional)"</label>
                            <input id="location" name="location" maxlength="500" />
                            <button type="submit">"Add show"</button>
                        </form>
                        <h3>"Published shows ("{show_count}")"</h3>
                        {(show_count == 0).then(|| view! { <p>"No published shows."</p> })}
                        <ul>{show_items}</ul>
                    </section>
                }
            })}
        {(section == Section::Videos)
            .then(|| {
                view! {
                    <section id="videos">
                        <h2 id="videos-heading">"YouTube videos"</h2>
                        <form
                            action="/admin/content"
                            method="post"
                            enctype="multipart/form-data"
                            aria-labelledby="videos-heading"
                        >
                            {fields(csrf, "videos", "add")}
                            <label for="url">"YouTube URL (required)"</label>
                            <p id="url-help">
                                "Paste an HTTPS YouTube watch, short, live, embed or youtu.be link."
                            </p>
                            <input
                                type="url"
                                id="url"
                                name="url"
                                maxlength="2048"
                                required
                                aria-describedby="url-help"
                                autocapitalize="none"
                                spellcheck="false"
                            />
                            <button type="submit">"Add video"</button>
                        </form>
                        <h3>"Published YouTube videos ("{video_count}")"</h3>
                        {(video_count == 0)
                            .then(|| view! { <p>"No published YouTube videos."</p> })}
                        <ul>{video_items}</ul>
                    </section>
                }
            })}
    })
}

/// Auth middleware verifies approval/Origin first; this handler verifies CSRF before uploads.
async fn submit(
    session: Session,
    Extension(database): Extension<DatabaseManager>,
    Extension(config): Extension<Arc<ServerConfig>>,
    request: Request,
) -> Response {
    let result = async {
        let mut headers = request.headers().clone();

        super::require_origin(&headers, &config.app_origin).map_err(login_failure)?;
        super::require_admin(&session, &database)
            .await
            .map_err(login_failure)?;

        let permit = PROCESSING
            .try_acquire()
            .map_err(|_| content_failure(ContentError::Busy))?;

        let read = async {
            let mut multipart = Multipart::from_request(request, &())
                .await
                .map_err(|_| invalid("Submit the content form as multipart data."))?;
            // Native forms place CSRF first. Reject a file-first body before buffering its bytes.
            let first = multipart
                .next_field()
                .await
                .map_err(|error| content_failure(multipart_error(error)))?
                .ok_or_else(|| login_failure(AuthError::Forbidden))?;

            if first.name() != Some("csrf_token") {
                return Err(login_failure(AuthError::Forbidden));
            }

            let csrf = read_field(first, 256).await.map_err(content_failure)?;
            if !headers.contains_key("x-csrf-token") {
                headers.insert(
                    "x-csrf-token",
                    HeaderValue::from_bytes(&csrf)
                        .map_err(|_| login_failure(AuthError::Forbidden))?,
                );
            }

            super::require_csrf(&session, &headers, &config.app_origin)
                .await
                .map_err(login_failure)?;

            let mut fields = BTreeMap::new();
            let mut photo = None;
            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|error| content_failure(multipart_error(error)))?
            {
                let name = field.name().unwrap_or("").to_string();
                if name == "photo" && photo.is_none() {
                    let filename = field
                        .file_name()
                        .ok_or_else(|| invalid("Choose a photo."))?
                        .to_string();

                    photo = Some((
                        filename,
                        read_field(field, gallery::UPLOAD_BYTES)
                            .await
                            .map_err(content_failure)?,
                    ));
                } else if matches!(
                    name.as_str(),
                    "kind"
                        | "action"
                        | "id"
                        | "confirmed"
                        | "date"
                        | "name"
                        | "club"
                        | "location"
                        | "url"
                ) && !fields.contains_key(&name)
                {
                    let value =
                        String::from_utf8(read_field(field, 2048).await.map_err(content_failure)?)
                            .map_err(|_| invalid("Text fields must be UTF-8."))?;
                    fields.insert(name, value);
                } else {
                    return Err(invalid("The form contains unexpected or duplicate fields."));
                }
            }
            Ok((fields, photo))
        };

        let (fields, photo) = tokio::time::timeout(Duration::from_secs(30), read)
            .await
            .map_err(|_| {
                (StatusCode::REQUEST_TIMEOUT, "Upload timed out. Try again.").into_response()
            })??;

        let value = |key: &str| fields.get(key).map(String::as_str).unwrap_or("");

        let kind = value("kind");
        let action = value("action");
        if photo.is_some() && (kind != "gallery" || action != "add") {
            return Err(invalid("Photos belong only in the gallery upload form."));
        }

        // Recheck approval after body IO so a revoked session cannot publish a delayed request.
        let actor = super::require_mutation(&session, &database, &headers, &config.app_origin)
            .await
            .map_err(login_failure)?;

        let id = match (kind, action) {
            ("shows", "add") => {
                content::validate_show(
                    value("date"),
                    value("name"),
                    value("club"),
                    value("location"),
                )
                .map_err(content_failure)?;

                let optional = |key| (!value(key).trim().is_empty()).then(|| value(key).trim());
                database
                    .add_show(
                        value("date"),
                        optional("name"),
                        optional("club"),
                        optional("location"),
                    )
                    .await
                    .map_err(|e| content_failure(e.into()))?
            }
            ("videos", "add") => {
                let id = content::youtube_id(value("url").trim()).map_err(|_| {
                    invalid("Enter a supported HTTPS YouTube URL with a valid video ID.")
                })?;
                database
                    .add_video(&id)
                    .await
                    .map_err(|e| content_failure(e.into()))?
            }
            ("gallery", "add") => {
                let (filename, bytes) = photo.ok_or_else(|| invalid("Choose a photo."))?;
                let root = config.data_dir.clone();
                let prepared = tokio::task::spawn_blocking(move || {
                    // A cancelled request cannot release the slot while its decoder is still running.
                    let _permit = permit;
                    gallery::prepare(&root, &filename, bytes)
                })
                .await
                .map_err(|_| content_failure(ContentError::Unavailable))?
                .map_err(content_failure)?;

                super::require_mutation(&session, &database, &headers, &config.app_origin)
                    .await
                    .map_err(login_failure)?;

                database
                    .add_gallery(
                        &prepared.name,
                        &prepared.original,
                        &prepared.thumbnail,
                        || prepared.publish(&config.data_dir),
                    )
                    .await
                    .map_err(|e| content_failure(e.into()))?
            }
            ("shows" | "videos" | "gallery", "remove") => {
                if value("confirmed") != "yes" {
                    return Err(invalid("Confirm removal of this item first."));
                }
                let id = value("id")
                    .parse::<i64>()
                    .ok()
                    .filter(|id| *id > 0)
                    .ok_or_else(|| invalid("Choose an existing item."))?;
                let removed = match kind {
                    "shows" => database
                        .remove_show(id)
                        .await
                        .map_err(|e| content_failure(e.into()))?,
                    "videos" => database
                        .remove_video(id)
                        .await
                        .map_err(|e| content_failure(e.into()))?,
                    _ => {
                        let photo = database
                            .remove_gallery(id)
                            .await
                            .map_err(|e| content_failure(e.into()))?;
                        if let Some(photo) = &photo {
                            gallery::cleanup(&config.data_dir, photo).await;
                        }
                        photo.is_some()
                    }
                };
                if !removed {
                    return Err((
                        StatusCode::NOT_FOUND,
                        "This item has already been removed. Reload administration.",
                    )
                        .into_response());
                }
                id
            }
            _ => return Err(invalid("Unknown content action.")),
        };
        tracing::info!(
            actor = actor.id,
            action,
            kind,
            content_id = id,
            "Content changed"
        );
        Ok(Redirect::to(&format!("/admin?section={kind}")).into_response())
    }
    .await;
    result.unwrap_or_else(|response| response)
}

#[inline]
fn fields(csrf: &str, kind: &'static str, action: &'static str) -> impl IntoView {
    view! {
        <input type="hidden" name="csrf_token" value=csrf.to_owned() />
        <input type="hidden" name="kind" value=kind />
        <input type="hidden" name="action" value=action />
    }
}

#[inline]
fn removal(kind: &'static str, id: i64, label: &str, csrf: &str) -> impl IntoView {
    let checkbox = format!("remove-{kind}-{id}");
    view! {
        <form
            action="/admin/content"
            method="post"
            enctype="multipart/form-data"
            aria-label=format!("Remove {label}")
        >
            {fields(csrf, kind, "remove")}
            <input type="hidden" name="id" value=id.to_string() />
            <input type="checkbox" id=checkbox.clone() name="confirmed" value="yes" required />
            <label for=checkbox>"Confirm permanent removal"</label>
            " "
            <button type="submit">"Remove"</button>
        </form>
    }
}

#[inline]
async fn read_field(mut field: Field<'_>, limit: usize) -> Result<Vec<u8>, ContentError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > limit {
            return Err(ContentError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[inline]
fn multipart_error(error: axum::extract::multipart::MultipartError) -> ContentError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ContentError::TooLarge
    } else {
        ContentError::Invalid("The upload is malformed. Use the administration form and try again.")
    }
}

#[inline]
fn invalid(message: &'static str) -> Response {
    content_failure(ContentError::Invalid(message))
}

#[inline]
fn content_failure(error: ContentError) -> Response {
    let (status, message) = match error {
        ContentError::Invalid(message) => (StatusCode::BAD_REQUEST, message),
        ContentError::Conflict => (StatusCode::CONFLICT, "That photo name or video already exists. Reload the catalog or choose a different photo filename."),
        ContentError::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "The submitted field or photo is too large."),
        ContentError::Busy => (StatusCode::TOO_MANY_REQUESTS, "Another content submission is processing. Please try again shortly."),
        ContentError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "Content could not be saved. Reload the catalog before retrying."),
    };
    page(
        status,
        "Content not saved",
        view! {
            <p>{message}</p>
            <a href="/admin">"Return to administration"</a>
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use http::{header::*, Request};
    use tower::ServiceExt;

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn fixture() -> (tempfile::TempDir, DatabaseManager, Router, String, String) {
        let (root, database) = crate::test_support::database().await;
        database
            .grant_administrator("issuer", "owner", "Owner <safe>")
            .await
            .unwrap();
        let session = crate::auth::test_session(&database);
        crate::auth::establish(&session, &database, "issuer", "owner")
            .await
            .unwrap();
        let csrf = session
            .get::<crate::auth::AdminSession>(crate::auth::ADMIN_KEY)
            .await
            .unwrap()
            .unwrap()
            .csrf;
        let cookie = format!("maj_admin_dev={}", session.id().unwrap());
        let options = leptos::config::LeptosOptions::builder()
            .output_name("test")
            .site_root(root.path().join("built").to_str().unwrap())
            .build();
        let app = crate::serve::router(
            Vec::new(),
            options,
            crate::test_support::config(root.path()),
            database.clone(),
        );
        (root, database, app, cookie, csrf)
    }

    fn multipart(fields: &[(&str, &str)], photo: Option<(&str, &[u8])>) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (name, value) in fields {
            bytes.extend_from_slice(format!("--boundary\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        if let Some((name, data)) = photo {
            bytes.extend_from_slice(format!("--boundary\r\nContent-Disposition: form-data; name=\"photo\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
            bytes.extend_from_slice(data);
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"--boundary--\r\n");
        bytes
    }

    async fn post(
        app: &Router,
        cookie: &str,
        origin: &str,
        fields: &[(&str, &str)],
        photo: Option<(&str, &[u8])>,
    ) -> Response {
        let response = app
            .clone()
            .oneshot(
                Request::post("/admin/content")
                    .header(COOKIE, cookie)
                    .header(ORIGIN, origin)
                    .header(CONTENT_TYPE, "multipart/form-data; boundary=boundary")
                    .body(Body::from(multipart(fields, photo)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[REFERRER_POLICY], "no-referrer");
        response
    }

    async fn get(app: &Router, path: &str) -> Response {
        app.clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    /// Starts an isolated authenticated server for Playwright; never built into production.
    #[tokio::test]
    #[ignore = "run through just test-admin-browser after installing Playwright browsers"]
    async fn browser_content_workflows() {
        let _lock = TEST_LOCK.lock().await;
        let (root, database, _app, cookie, _csrf) = fixture().await;
        let source = crate::test_support::content_source();
        crate::content::import(&database, root.path(), source.path())
            .await
            .unwrap();
        let firefox = crate::auth::test_session(&database);
        crate::auth::establish(&firefox, &database, "issuer", "owner")
            .await
            .unwrap();
        let cookies = serde_json::json!({
            "chromium": cookie,
            "firefox": format!("maj_admin_dev={}", firefox.id().unwrap()),
        })
        .to_string();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        // Native forms send this test listener's Origin; production still uses configured Origin.
        let mut config = crate::test_support::config(root.path());
        config.app_origin = format!("http://{address}");
        let options = leptos::config::LeptosOptions::builder()
            .output_name("test")
            .site_root(root.path().join("built").to_str().unwrap())
            .build();
        let app = crate::serve::router(Vec::new(), options, config, database.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let result = tokio::task::spawn_blocking(move || {
            std::process::Command::new("npx")
                .current_dir("end2end")
                .args([
                    "playwright",
                    "test",
                    "admin-content.spec.ts",
                    "--project=chromium",
                    "--project=firefox",
                    "--workers=1",
                    "--reporter=line",
                ])
                .env("PLAYWRIGHT_BASE_URL", format!("http://{address}"))
                .env("MAJ_TEST_ADMIN_COOKIES", cookies)
                .status()
        })
        .await
        .unwrap();
        server.abort();
        let _ = server.await;
        database.close().await;
        assert!(result.unwrap().success(), "Browser workflow tests failed");
    }

    #[tokio::test]
    async fn imported_records_remain_manageable_after_restart_and_each_section_lists_its_catalog() {
        let _lock = TEST_LOCK.lock().await;
        let (root, database, app, cookie, csrf) = fixture().await;
        let source = crate::test_support::content_source();
        crate::content::import(&database, root.path(), source.path())
            .await
            .unwrap();
        // Reopen the same durable data, using the original saved browser session.
        drop(app);
        database.close().await;
        let database = DatabaseManager::initialize(root.path()).await.unwrap();
        let options = leptos::config::LeptosOptions::builder()
            .output_name("test")
            .build();
        let app = crate::serve::router(
            Vec::new(),
            options,
            crate::test_support::config(root.path()),
            database.clone(),
        );
        let show = database.shows().await.unwrap()[0].id;
        let video = database.videos().await.unwrap()[0].id;
        let photo = database.gallery().await.unwrap().remove(0);
        for (kind, id, marker) in [
            ("shows", show, "Published chromium"),
            ("videos", video, "RBwe-c9T5sY"),
            ("gallery", photo.id, photo.public_name.as_str()),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("/admin?section={kind}"))
                        .header(COOKIE, &cookie)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(
                to_bytes(response.into_body(), 100_000)
                    .await
                    .unwrap()
                    .to_vec(),
            )
            .unwrap();
            assert!(html.contains(marker));
            assert_eq!(html.matches("<li>").count(), 2);
            assert_eq!(html.matches("<section ").count(), 1);
            assert!(html.contains(&format!("id=\"{kind}\"")));
            assert!(html.contains(&format!("name=\"id\" value=\"{id}\"")));
            let id = id.to_string();
            let fields = [
                ("csrf_token", csrf.as_str()),
                ("kind", kind),
                ("action", "remove"),
                ("id", id.as_str()),
                ("confirmed", "yes"),
            ];
            let response = post(&app, &cookie, "http://localhost:8080", &fields, None).await;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers()[LOCATION],
                format!("/admin?section={kind}")
            );
        }
        assert_eq!(
            get(
                &app,
                &crate::routes::about::gallery_image_url(&photo.public_name)
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
        for query in ["section=unknown", "section=shows&section=videos"] {
            assert_eq!(
                app.clone()
                    .oneshot(
                        Request::get(format!("/admin?{query}"))
                            .header(COOKIE, &cookie)
                            .body(Body::empty())
                            .unwrap()
                    )
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        database.close().await;
        let reopened = DatabaseManager::initialize(root.path()).await.unwrap();
        assert_eq!(reopened.shows().await.unwrap().len(), 1);
        assert_eq!(reopened.videos().await.unwrap().len(), 1);
        assert_eq!(reopened.gallery().await.unwrap().len(), 1);
        reopened.close().await;
    }

    #[tokio::test]
    async fn approved_forms_add_list_and_remove_live_content_with_confirmation() {
        let _lock = TEST_LOCK.lock().await;
        let (root, database, app, cookie, csrf) = fixture().await;
        let origin = "http://localhost:8080";
        let fields = [
            ("csrf_token", csrf.as_str()),
            ("kind", "shows"),
            ("action", "add"),
            ("date", "2024-02-29"),
            ("name", "<script>unsafe</script>"),
        ];
        assert_eq!(
            post(&app, &cookie, origin, &fields, None).await.status(),
            StatusCode::SEE_OTHER
        );
        let shows = database.shows().await.unwrap();
        assert_eq!(shows.len(), 1);
        assert!(shows[0].club.is_none() && shows[0].location.is_none());
        let fields = [
            ("csrf_token", csrf.as_str()),
            ("kind", "videos"),
            ("action", "add"),
            ("url", "https://youtu.be/3uh_Jc1prRQ"),
        ];
        assert_eq!(
            post(&app, &cookie, origin, &fields, None).await.status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            post(&app, &cookie, origin, &fields, None).await.status(),
            StatusCode::CONFLICT
        );
        let mut photo = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(60, 40))
            .write_to(&mut photo, image::ImageFormat::Png)
            .unwrap();
        let fields = [
            ("csrf_token", csrf.as_str()),
            ("kind", "gallery"),
            ("action", "add"),
        ];
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &fields,
                Some(("My Photo.PNG", photo.get_ref()))
            )
            .await
            .status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &fields,
                Some(("MY PHOTO.PNG", photo.get_ref()))
            )
            .await
            .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(database.gallery().await.unwrap().len(), 1);
        assert_eq!(
            std::fs::read_dir(root.path().join("gallery/originals"))
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("staging"))
                .unwrap()
                .count(),
            0
        );
        let html = app
            .clone()
            .oneshot(
                Request::get("/admin?section=shows")
                    .header(COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(html.status(), StatusCode::OK);
        let html =
            String::from_utf8(to_bytes(html.into_body(), 100_000).await.unwrap().to_vec()).unwrap();
        assert!(html.contains("&lt;script&gt;unsafe&lt;/script&gt;"));
        assert!(html.contains("enctype=\"multipart/form-data\""));
        assert!(html.contains("Confirm permanent removal"));
        let image = get(&app, "/img/about_pics/my_photo.png").await;
        assert_eq!(image.status(), StatusCode::OK);
        assert_eq!(image.headers()[CONTENT_TYPE], "image/png");
        assert_eq!(image.headers()[CACHE_CONTROL], "no-store");
        let thumbnail = get(&app, "/img/about_thumbnails/my_photo.png.webp").await;
        assert_eq!(thumbnail.status(), StatusCode::OK);
        assert_eq!(thumbnail.headers()[CONTENT_TYPE], "image/webp");
        let bytes = to_bytes(thumbnail.into_body(), 100_000).await.unwrap();
        let thumb = image::load_from_memory(&bytes).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (480, 270));
        for (kind, id) in [
            ("shows", shows[0].id),
            ("videos", database.videos().await.unwrap()[0].id),
            ("gallery", database.gallery().await.unwrap()[0].id),
        ] {
            let id = id.to_string();
            let mut fields = vec![
                ("csrf_token", csrf.as_str()),
                ("kind", kind),
                ("action", "remove"),
                ("id", id.as_str()),
            ];
            assert_eq!(
                post(&app, &cookie, origin, &fields, None).await.status(),
                StatusCode::BAD_REQUEST
            );
            fields.push(("confirmed", "yes"));
            assert_eq!(
                post(&app, &cookie, origin, &fields, None).await.status(),
                StatusCode::SEE_OTHER
            );
            assert_eq!(
                post(&app, &cookie, origin, &fields, None).await.status(),
                StatusCode::NOT_FOUND
            );
        }
        assert!(database.shows().await.unwrap().is_empty());
        assert!(database.videos().await.unwrap().is_empty());
        assert!(database.gallery().await.unwrap().is_empty());
        assert_eq!(
            get(&app, "/img/about_pics/my_photo.png").await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&app, "/img/about_thumbnails/my_photo.png.webp")
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("gallery/originals"))
                .unwrap()
                .count(),
            0
        );
        database.close().await;
    }

    #[tokio::test]
    async fn editor_rejects_untrusted_forms_revocation_and_oversized_or_invalid_uploads() {
        let _lock = TEST_LOCK.lock().await;
        let (root, database, app, cookie, csrf) = fixture().await;
        let origin = "http://localhost:8080";
        let fields = [
            ("csrf_token", csrf.as_str()),
            ("kind", "shows"),
            ("action", "add"),
            ("date", "2026-01-01"),
        ];
        assert_eq!(
            post(&app, "", origin, &fields, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            post(&app, &cookie, "https://attacker.invalid", &fields, None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            post(&app, &cookie, origin, &[("csrf_token", "wrong")], None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &[],
                Some(("photo.jpg", b"not read as a photo"))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let permit = PROCESSING.acquire().await.unwrap();
        assert_eq!(
            post(&app, &cookie, origin, &fields, None).await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        drop(permit);
        for date in ["2025-02-29", "2026-02-31", "0000-01-01", "2026-1-1"] {
            let fields = [
                ("csrf_token", csrf.as_str()),
                ("kind", "shows"),
                ("action", "add"),
                ("date", date),
            ];
            assert_eq!(
                post(&app, &cookie, origin, &fields, None).await.status(),
                StatusCode::BAD_REQUEST
            );
        }
        for url in [
            "https://youtube.com.evil.invalid/watch?v=3uh_Jc1prRQ",
            "http://localhost:8080/private",
            "<iframe>",
        ] {
            let fields = [
                ("csrf_token", csrf.as_str()),
                ("kind", "videos"),
                ("action", "add"),
                ("url", url),
            ];
            assert_eq!(
                post(&app, &cookie, origin, &fields, None).await.status(),
                StatusCode::BAD_REQUEST
            );
        }
        let fields = [
            ("csrf_token", csrf.as_str()),
            ("kind", "gallery"),
            ("action", "add"),
        ];
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &fields,
                Some(("photo.jpg", b"<svg/>"))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &fields,
                Some(("photo.jpg", &vec![0; gallery::UPLOAD_BYTES + 1]))
            )
            .await
            .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            post(
                &app,
                &cookie,
                origin,
                &fields,
                Some(("photo.jpg", &vec![0; gallery::UPLOAD_BYTES + 20_000]))
            )
            .await
            .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("staging"))
                .unwrap()
                .count(),
            0
        );
        assert!(database.shows().await.unwrap().is_empty());
        assert!(database.videos().await.unwrap().is_empty());
        assert!(database.gallery().await.unwrap().is_empty());
        database
            .revoke_administrator("issuer", "owner")
            .await
            .unwrap();
        assert_eq!(
            post(&app, &cookie, origin, &fields, None).await.status(),
            StatusCode::FORBIDDEN
        );
        database.close().await;
    }

    #[tokio::test]
    async fn gallery_serves_imported_encoded_names_but_never_orphans_or_bundled_fallbacks() {
        let _lock = TEST_LOCK.lock().await;
        let (root, database, app, _cookie, _csrf) = fixture().await;
        let built = root.path().join("built/img/about_pics");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::write(built.join("removed.png"), b"must not serve").unwrap();
        std::fs::write(
            root.path().join("gallery/originals/orphan.png"),
            b"private orphan",
        )
        .unwrap();
        for path in [
            "/img/about_pics/removed.png",
            "/img/about_pics/orphan.png",
            "/%69mg/about_pics/removed.png",
            "/img/%61bout_pics/removed.png",
            "/img/x/../about_pics/removed.png",
            "/img/x/%2e%2e/about_pics/removed.png",
        ] {
            let response = get(&app, path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            let bytes = to_bytes(response.into_body(), 100_000).await.unwrap();
            assert!(!String::from_utf8_lossy(&bytes).contains("must not serve"));
        }
        std::fs::write(
            root.path().join("gallery/originals/imported.png"),
            b"imported image",
        )
        .unwrap();
        let id = database
            .add_gallery("Photo é #1.png", "imported.png", "imported.webp", || Ok(()))
            .await
            .unwrap();
        let path = crate::routes::about::gallery_image_url("Photo é #1.png");
        assert_eq!(get(&app, &path).await.status(), StatusCode::OK);
        database.remove_gallery(id).await.unwrap();
        assert_eq!(get(&app, &path).await.status(), StatusCode::NOT_FOUND);
        database.close().await;
    }
}
