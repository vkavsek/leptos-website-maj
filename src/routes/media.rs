use leptos::prelude::*;
use leptos_meta::{Link, Title};
use serde::{Deserialize, Serialize};

#[component]
pub fn Media() -> impl IntoView {
    let videos_res = Resource::new(|| (), |_| async move { get_youtube_videos().await });

    view! {
        <Link rel="icon" href="/img/media.svg" type_="image/svg" />
        <Title text="Media" />

        <div class="components" id="media-components">
            <div class="title" id="media-title">
                // <img class="title-img" id="media-img" src="/img/titles/media.svg"/>
                <h1>"Media"</h1>
            </div>
            <div class="contents" id="media-contents">
                <Suspense fallback=|| {
                    view! { <p>"Loading..."</p> }
                }>
                    {move || Suspend::new(async move {
                        videos_res
                            .await
                            .map(|videos| {
                                videos
                                    .into_iter()
                                    .map(|video| view! { <CreateVideo video /> })
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
            <iframe class="yt-video" src=video.url title="YouTube video player"></iframe>
        </div>
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct YoutubeUrl {
    pub url: String,
}

// ###################################
// ->   Server
// ###################################
#[server(GetMediaVideos, "/api", "GetJson", "get_media_videos")]
async fn get_youtube_videos() -> Result<Vec<YoutubeUrl>, crate::MajServerError> {
    use tokio::sync::OnceCell;

    let options =
        use_context::<leptos::config::LeptosOptions>().ok_or(crate::MajServerError::Internal)?;
    static VIDEOS_INIT: OnceCell<Result<Vec<YoutubeUrl>, crate::MajServerError>> =
        OnceCell::const_new();

    tracing::debug!("Retrieving Yt Videos");
    VIDEOS_INIT
        .get_or_init(|| load_youtube_videos(&options.site_root))
        .await
        .clone()
}

#[cfg(feature = "ssr")]
async fn load_youtube_videos(site_root: &str) -> Result<Vec<YoutubeUrl>, crate::MajServerError> {
    let path = std::path::Path::new(site_root).join("data_json/yt_videos.json");
    tracing::info!("Initializing YT videos");
    let videos_json = tokio::fs::read_to_string(path).await?;
    tracing::debug!("videos_json: {videos_json}");
    Ok(serde_json::from_str(&videos_json)?)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn video_loader_uses_configured_roots_and_reports_missing_files() {
        for root in [
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/site"),
            "tests/fixtures/site",
        ] {
            let videos = load_youtube_videos(root).await.unwrap();
            assert_eq!(videos.len(), 1);
            assert_eq!(
                videos[0].url,
                "https://www.youtube.com/embed/configured-root-test"
            );
        }
        assert!(matches!(
            load_youtube_videos("tests/fixtures/missing-site").await,
            Err(crate::MajServerError::Io(_))
        ));
    }
}
