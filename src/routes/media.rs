use leptos::*;
use leptos_meta::{Link, Title};

#[component]
pub fn Media() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/media.svg" type_="image/svg"/>
        <Title text="Media"/>

        <div class="components" id="media-components">
            <div class="title" id="media-title">
                // <img class="title-img" id="media-img" src="/img/titles/media.svg"/>
                <h1>"Media"</h1>
            </div>
            <div class="contents" id="media-contents">
                    <iframe
                        class="yt-video hero-vid"
                        width="960"
                        height="615"
                        src="https://www.youtube-nocookie.com/embed/73KJzjvW_1Q?si=MQ4I2VTipAaEUayG"
                        title="YouTube video player"
                        frameborder="0"
                        loading="lazy"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                    <iframe
                        class="yt-video vid"
                        width="560"
                        height="315"
                        src="https://www.youtube-nocookie.com/embed/aaRGcLUQ4ss?si=5BEyXSQYtl6ogSCL"
                        title="YouTube video player"
                        frameborder="0"
                        loading="lazy"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                    <iframe
                        class="yt-video vid"
                        width="560"
                        height="315"
                        src="https://www.youtube-nocookie.com/embed/RBwe-c9T5sY?si=0m4cEVb4BktHR9Pr"
                        title="YouTube video player"
                        frameborder="0"
                        loading="lazy"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
            </div>
        </div>
    }
}
