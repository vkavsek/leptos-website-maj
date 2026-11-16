use leptos::prelude::*;
use leptos_meta::{Link, Title};
use serde::{Deserialize, Serialize};

use crate::routes::error::ErrorTemplate;

#[component]
pub fn Media() -> impl IntoView {
    let videos_res = Resource::new_blocking(|| (), |_| async move { get_youtube_videos().await });

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
                    <ErrorBoundary fallback=move |errors| view! { <ErrorTemplate errors /> }>
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
                    </ErrorBoundary>
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
                allow="encrypted-media; picture-in-picture; fullscreen"
                allowfullscreen
            ></iframe>
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
/// Read the current catalog on each resource load; never retain editable data process-wide.
#[server(GetMediaVideos, "/api", "GetJson", "get_media_videos")]
async fn get_youtube_videos() -> Result<Vec<YoutubeUrl>, crate::MajServerError> {
    let database =
        use_context::<crate::database::DatabaseManager>().ok_or(crate::MajServerError::Internal)?;
    load_youtube_videos(&database).await
}

#[cfg(feature = "ssr")]
async fn load_youtube_videos(
    database: &crate::database::DatabaseManager,
) -> Result<Vec<YoutubeUrl>, crate::MajServerError> {
    database
        .videos()
        .await
        .map(|videos| {
            videos
                .into_iter()
                .map(|video| YoutubeUrl {
                    url: format!("https://www.youtube-nocookie.com/embed/{}", video.video_id),
                })
                .collect()
        })
        .map_err(|_| crate::MajServerError::Internal)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn video_loader_reflects_additions_and_removals() {
        let (_root, database) = crate::test_support::database().await;
        assert!(load_youtube_videos(&database).await.unwrap().is_empty());
        let id = database.add_video("3uh_Jc1prRQ").await.unwrap();
        assert_eq!(
            load_youtube_videos(&database).await.unwrap()[0].url,
            "https://www.youtube-nocookie.com/embed/3uh_Jc1prRQ"
        );
        database.remove_video(id).await.unwrap();
        assert!(load_youtube_videos(&database).await.unwrap().is_empty());
        database.close().await;
        assert!(load_youtube_videos(&database).await.is_err());
    }
}
