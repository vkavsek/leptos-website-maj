use leptos::*;
use leptos_meta::{Link, Title};

#[component]
pub fn Media() -> impl IntoView {
    let videos_res = create_resource(|| (), |_| async move { get_youtube_videos().await });

    view! {
        <Link rel="icon" href="/img/media.svg" type_="image/svg"/>
        <Title text="Media"/>

        <div class="components" id="media-components">
            <div class="title" id="media-title">
                // <img class="title-img" id="media-img" src="/img/titles/media.svg"/>
                <h1>"Media"</h1>
            </div>
            <div class="contents" id="media-contents">
                <Suspense fallback=|| {
                    view! { <p>"Loading..."</p> }
                }>

                    {videos_res
                        .get()
                        .map(|videos| {
                            videos
                                .ok()
                                .map(|videos| {
                                    videos
                                        .into_iter()
                                        .map(|video| view! { <CreateVideo video/> })
                                        .collect_view()
                                })
                        })}

                </Suspense>
            </div>
        </div>
    }
}

#[component]
pub fn CreateVideo(video: YoutubeUrl) -> impl IntoView {
    view! {
        <div class="yt-vid-container">
            <iframe
                class="yt-video"
                src=video.url
                title="YouTube video player"
                frameborder="0"
                loading="lazy"
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                allowfullscreen
            ></iframe>
        </div>
    }
}

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct YoutubeUrl {
    pub url: String,
}

// ###################################
// ->   Server
// ###################################

#[server(GetMediaVideos, "/api", "GetJson", "get_media_videos")]
async fn get_youtube_videos() -> Result<Vec<YoutubeUrl>, ServerFnError> {
    let res = get_shows_util()
        .await
        .as_ref()
        .map_err(ServerFnError::WrappedServerError)?;

    Ok(res.clone())
}

#[cfg(feature = "ssr")]
async fn get_shows_util() -> &'static Result<Vec<YoutubeUrl>, crate::MajServerError> {
    use tokio::sync::OnceCell;
    use tracing::info;

    static SHOWS_INIT: OnceCell<Result<Vec<YoutubeUrl>, crate::MajServerError>> =
        OnceCell::const_new();

    tracing::debug!("Retrieving Yt Videos");
    SHOWS_INIT
        .get_or_init(|| async {
            let path = if cfg!(not(debug_assertions)) {
                "/app/site/data_json/yt_videos.json"
            } else {
                "./public/data_json/yt_videos.json"
            };

            info!("Initializing YT videos");
            let videos_json = tokio::fs::read_to_string(path).await?;
            let mut videos: Vec<YoutubeUrl> = serde_json::from_str(&videos_json)?;

            // Try to convert url to youtube-nocookie
            for video in videos.iter_mut() {
                use lazy_regex::regex_find;
                let regex = regex_find!(r#"(?:v=|\/)([0-9A-Za-z_-]{11}).*"#, &video.url);

                let Some(url) = regex.map(|s| s.strip_prefix("v=").unwrap_or_default()) else {
                    continue;
                };

                video.url = format!("https://www.youtube-nocookie.com/embed/{}", url);
            }

            Ok(videos)
        })
        .await
}
