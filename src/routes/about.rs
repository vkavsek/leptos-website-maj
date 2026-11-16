use crate::app::use_dialog_light_dismiss;
use crate::{routes::error::ErrorTemplate, MajServerError};
use leptos::{
    html::{Button, Div},
    prelude::*,
};
use leptos_meta::{Link, Title};

#[component]
pub fn About() -> impl IntoView {
    let text_content = [
        view! {
            <strong>"Maj Kavšek"</strong>
            " was born on April 12, 2000, in Ljubljana, Slovenia. His musical journey began at 8 when he first picked up the trumpet. After finishing his studies at the Conservatorium for Music in Ljubljana, he decided to further his musical education by enrolling at the prestigious "
            <strong>"Jazz Institut Berlin (UdK Berlin - University of Arts Berlin)"</strong>
            ". This move allowed him to expand his musical knowledge and study under renowned professors, including "
            <strong>"Ralph Alessi"</strong>
            ", "
            <strong>"Greg Cohen"</strong>
            ", "
            <strong>"Reggie Moore"</strong>
            ", "
            <strong>"James Robert Rotondi"</strong>
            ", "
            <strong>"Nate Wooley"</strong>
            "... His studies in Berlin gave him the opportunity to explore a wide range of musical ideas and develop his skills in both performance and composition."
        }.into_any(),
        view! {
            "In 2023, he was invited to perform with the "
            <strong>"Euroradio Jazz Orchestra"</strong>
            " in Lithuania and "
            <strong>"Fame's Institute Orchestra"</strong>
            "in North Macedonia. In 2025 he was awarded the "
            <strong>"Yuhedi Menuhin: LiveMusicNow scholarship"</strong>
            " and a "
            <strong>"third prize at Jazzon: Composition Award"</strong>
            ". The same year he also won the "
            <strong>
                <cite>
                    <b>"Jazz Incubator"</b>
                </cite>
                " Residency at Festival Jazz Cerkno"
            </strong>
            ", where he recorded his second album."
        }.into_any(),
        view! {
            "Maj has been an active participant in various jazz festivals and venues, where he has performed both as a bandleader and a side-man. These opportunities have given him the chance to collaborate with many esteemed musicians, including "
            <strong>
                "Jure Pukl, Marko Churnchetz, Chris Pitsiokos, Philipp Gropper, Bram De Looze, David Gazarov, Felix Henkelhausen, Julius Gawlik, Jan Bang and Arve Henriksen"
            </strong>
            ", among others. Through these collaborations, Maj has demonstrated his adaptability and ability to work in diverse musical environments."
        }.into_any(),
        view! {
            "In 2024, he released his debut album, "
            <strong>
                <cite>
                    <q>"Galaterna"</q>
                </cite>
            </strong>
            ", recorded with his quintet and released under the "
            <strong>"Aut Records"</strong>
            " label. The album has been well received by critics, being praised for its dynamic and expressive qualities."
        }.into_any(),
        view! {
            "As "
            <strong>"Pierre Dulieu"</strong>
            " of JazzMania wrote on September 27, 2024: “This energetic and warm music, tailor-made for club performances, radiates freshness, features generous solos (listening to the trumpeter soar on 'Philosopher’s Stoned' is a treat), and boasts collective playing confidence that makes it exhilarating. Definitely worth a listen!”"
        }.into_any(),
        view! {
            "Additionally, "
            <strong>"Jan Granlie"</strong>
            " of "
            <strong>"Salt Peanuts"</strong>
            " noted on July 5, 2024, that the album "
            <cite>
                <q>
                    "showcases exceptionally well-played music that draws threads back to the American jazz traditions of the 1960s, while maintaining a clear forward-looking perspective."
                </q>
            </cite>
        }.into_any(),
        view! {
            "In 2026, he released his second album, "
            <strong>"MINOR FLAW"</strong>
            ", which features a collection of compositions carefully crafted to represent the creative approach he has developed over the years. Featuring a chordless quartet, the project explores the possibilities of expression and tests the limitations of compositional concepts while maintaining a strong connection to the given structure and form. The music carries the influences that shaped his musical identity, ranging from folk and classical music to traditional as well as contemporary jazz."
        }.into_any(),
        view! {
            "Through his dedication to his craft and his passion for music, Maj Kavšek continues to make a mark on the international music scene. His performances, recordings, and collaborations showcase his versatility and his ability to inspire both audiences and fellow musicians alike."
        }.into_any(),
    ];

    let text_content = text_content
        .map(|line| view! { <p>{line}</p> })
        .collect_view();

    let mut color = Color::Yellow;
    let init_color = format!("color: {}", color.return_val());
    let init_bg_color = format!("background-color: {}", color.return_val());
    let init_next_bg_color = format!("background-color: {}", (color.other()).return_val());

    let text_ref: NodeRef<Div> = NodeRef::new();
    let button_ref: NodeRef<Button> = NodeRef::new();
    let curr_col_ref: NodeRef<Div> = NodeRef::new();
    let next_col_ref: NodeRef<Div> = NodeRef::new();

    let change_color = move |_| {
        let text_div = text_ref.get().expect("the DOM should be built by now");
        let button = button_ref.get().unwrap();
        let current_color = curr_col_ref.get().unwrap();
        let next_color = next_col_ref.get().unwrap();
        color.switch();
        let cd = color.return_val();
        let next_cd = (color.other()).return_val();
        let _ = text_div.style(("color", cd));
        let _ = button.style(("color", cd));
        let _ = current_color.style(("background-color", cd));
        let _ = next_color.style(("background-color", next_cd));
    };

    view! {
        <Link rel="icon" href="/img/izkaznica.svg" type_="image/svg" />
        <Title text="About Me" />
        <div class="components" id="about-components">
            <div class="title" id="about-title">
                <h1>"About Me"</h1>
            </div>
            <div class="contents" id="about-wrap">
                <ImagesAbout />
                <div class="about-text" style=init_color.clone() node_ref=text_ref>
                    {text_content}
                    <button
                        class="change-color-button"
                        aria-label="Change biography text color"
                        on:click=change_color
                        style=init_color.clone()
                        node_ref=button_ref
                    >
                        <div class="color-box" style=init_bg_color node_ref=curr_col_ref></div>
                        <img src="/img/icon/back.svg" class="button-next-arrow" alt="" />
                        <div class="color-box" style=init_next_bg_color node_ref=next_col_ref></div>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn ImagesAbout() -> impl IntoView {
    let files = OnceResource::new_blocking(read_image_files());

    // Click on image to make it fullscreen.
    let (current_img, set_current_img) = signal(None);
    let fullscreen_ref = NodeRef::<leptos::html::Dialog>::new();
    use_dialog_light_dismiss(fullscreen_ref);

    let close_fullscreen = move |_| {
        if let Some(dialog) = fullscreen_ref.get() {
            dialog.close();
        }
    };

    let next = move || {
        if let Some(img) = current_img.get() {
            if let Some(Ok(files)) = files.get() {
                let pos = files
                    .iter()
                    .position(|file_path| file_path == &img)
                    .unwrap_or_default();

                let next_file = if let Some(file) = files.get(pos + 1) {
                    file.clone()
                } else {
                    files[0].clone()
                };
                set_current_img.set(Some(next_file));
            }
        }
    };
    let fullscreen_next = move |_| {
        next();
    };
    let back = move || {
        if let Some(img) = current_img.get() {
            if let Some(Ok(files)) = files.get() {
                let pos = files
                    .iter()
                    .position(|file_path| file_path == &img)
                    .unwrap_or_default();

                let next_file = if pos > 0 && files.get(pos - 1).is_some() {
                    files[pos - 1].clone()
                } else {
                    files[files.len() - 1].clone()
                };
                set_current_img.set(Some(next_file));
            }
        }
    };
    let fullscreen_back = move |_| {
        back();
    };

    view! {
            <dialog
                node_ref=fullscreen_ref
                class="fullscreen-container"
                aria-label="Photo viewer"
                closedby="any"
                on:close=move |_| set_current_img.set(None)
            >
                <div class="fullscreen-img-buttons">
                    <img src=move || current_img.get() class="fullscreen-img" alt="Enlarged gallery photo" />
                    <button on:click=close_fullscreen class="fullscreen-close" aria-label="Close photo" autofocus=true>
                        <img
                            src="/img/icon/cross.svg"
                            alt=""
                            style:width="1rem"
                            style:opacity="70%"
                        />
                    </button>
                    <button on:click=fullscreen_back class="fullscreen-nav-button back-button" aria-label="Previous photo">
                        <img class="nav-button-img back-img" src="/img/icon/back.svg" alt="" />
                    </button>
                    <button on:click=fullscreen_next class="fullscreen-nav-button next-button" aria-label="Next photo">
                        <img class="nav-button-img next-img" src="/img/icon/back.svg" alt="" />
                    </button>
                </div>
            </dialog>
        <div class="about-gallery">
            <div class="about-images-scroller snaps-inline">
                <Suspense fallback=move || {
                    view! { <p>"Loading"</p> }
                }>
                    <ErrorBoundary fallback=move |errors| {
                        view! { <ErrorTemplate errors /> }
                    }>
                        {move || {
                            files.get().map(|result| {
                                result.map(|files| {
                                    files.into_iter()
                                        .enumerate()
                                        .map(|(index, file)| {
                                            let file_c = file.clone();
                                            let click_image = move |ev: leptos::ev::MouseEvent| {
                                                ev.prevent_default();
                                                set_current_img.set(Some(file_c.clone()));
                                                if let Some(dialog) = fullscreen_ref.get() {
                                                    let _ = dialog.show_modal();
                                                }
                                            };
                                            view! {
                                                <button
                                                    type="button"
                                                    class="gallery-thumbnail"
                                                    aria-label=format!("Open gallery photo {}", index + 1)
                                                    on:click=click_image
                                                >
                                                    <img
                                                        src=format!("{}.webp", file.replace("/about_pics/", "/about_thumbnails/"))
                                                        class="image-about"
                                                        alt=""
                                                        width="480"
                                                        height="270"
                                                        loading=if index < 4 { "eager" } else { "lazy" }
                                                        decoding="async"
                                                    />
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                })
                            })
                        }}
                    </ErrorBoundary>
                </Suspense>
            </div>
        </div>
    }
}

enum Color {
    #[allow(dead_code)]
    Yellow,
    Pink,
}
impl Color {
    fn return_val(&self) -> &'static str {
        match self {
            Color::Yellow => "#fad549",
            Color::Pink => "#ff3988",
        }
    }
    fn switch(&mut self) {
        match self {
            Color::Yellow => {
                *self = Color::Pink;
            }
            Color::Pink => {
                *self = Color::Yellow;
            }
        }
    }
    fn other(&self) -> Self {
        match self {
            Color::Yellow => Color::Pink,
            Color::Pink => Color::Yellow,
        }
    }
}

// ###################################
// ->   SERVER
// ###################################

/// Only published catalog entries are visible; an empty catalog is an empty gallery.
#[server(ReadImageFiles, "/api", "GetJson", "get_image_files")]
async fn read_image_files() -> Result<Vec<String>, MajServerError> {
    let database =
        use_context::<crate::database::DatabaseManager>().ok_or(MajServerError::Internal)?;
    load_image_files(&database).await
}

#[cfg(feature = "ssr")]
async fn load_image_files(
    database: &crate::database::DatabaseManager,
) -> Result<Vec<String>, MajServerError> {
    database
        .gallery()
        .await
        .map(|photos| {
            photos
                .iter()
                .map(|photo| gallery_image_url(&photo.public_name))
                .collect()
        })
        .map_err(|_| MajServerError::Internal)
}

#[cfg(feature = "ssr")]
pub(crate) fn gallery_image_url(name: &str) -> String {
    use std::fmt::Write;

    let mut url = String::from("/img/about_pics/");
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            url.push(char::from(byte));
        } else {
            write!(url, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    url
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn every_gallery_original_has_a_smaller_webp_thumbnail() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/public");
        for original in std::fs::read_dir(format!("{root}/img/about_pics")).unwrap() {
            let original = original.unwrap();
            let name = original.file_name();
            let Some(name) = name.to_str() else { continue };
            let thumbnail =
                std::fs::read(format!("{root}/img/about_thumbnails/{name}.webp")).unwrap();
            let original_size = original.metadata().unwrap().len();
            assert!(thumbnail.starts_with(b"RIFF") && thumbnail.get(8..12) == Some(b"WEBP"));
            assert!((thumbnail.len() as u64) < original_size, "{name}");
        }
    }

    #[tokio::test]
    async fn gallery_lists_only_live_records_and_encodes_filename_segments() {
        let (_root, database) = crate::test_support::database().await;
        assert!(load_image_files(&database).await.unwrap().is_empty());
        let id = database
            .add_gallery("photo#1? 100%é.png", "private.png", "private.webp", || {
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            load_image_files(&database).await.unwrap(),
            ["/img/about_pics/photo%231%3F%20100%25%C3%A9.png"]
        );
        database.remove_gallery(id).await.unwrap();
        assert!(load_image_files(&database).await.unwrap().is_empty());
        database.close().await;
        assert!(load_image_files(&database).await.is_err());
    }

    #[tokio::test]
    async fn gallery_loading_failure_renders_an_error_without_panicking() {
        use axum::{
            body::{to_bytes, Body},
            Router,
        };
        use http::Request;
        use leptos_axum::{generate_route_list, LeptosRoutes};
        use tower::ServiceExt;

        let options = leptos::config::LeptosOptions::builder()
            .output_name("leptos-website-maj")
            .site_root(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/missing-site"
            ))
            .build();
        let app = Router::new()
            .leptos_routes(&options, generate_route_list(crate::app::App), {
                let options = options.clone();
                move || crate::app::app_shell(options.clone())
            })
            .with_state(options);
        let response = app
            .oneshot(Request::get("/about_me").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), 100_000).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("error-wrap"));
        assert!(html.contains("Internal"));
    }
}
