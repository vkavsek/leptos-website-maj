use leptos::*;
use leptos_meta::{Link, Title};

#[component]
pub fn Media() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/media.svg" type_="image/svg"/>
        <Title text="Media"/>
        <div class="components" id="media-components">
            <div class="title" id="media-title">
                <img class="title-img" id="media-img" src="/img/titles/media.svg"/>
            </div>
            <div class="contents" class="media-contents">
                <div class="youtube-video video-1">
                    <iframe
                        src="https://www.youtube.com/embed/kR12N0Yfs-4?si=6TIcz6c7mYv43lbU"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-2">
                    <iframe
                        src="https://www.youtube.com/embed/yXpqygsFzU8?si=uz8IgwKn8ZFZzCCQ"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-3">
                    <iframe
                        src="https://www.youtube.com/embed/iQrQ0kNrlMU?si=L2HxKVoPUf_3HgjG"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-4">
                    <iframe
                        src="https://www.youtube.com/embed/HL4MXsVs-ZA?si=VonHLddhsJfNKvK8"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-5">
                    <iframe
                        src="https://www.youtube.com/embed/buCZtlFn5Fg?si=moIpj0Dg40mvlmUe"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-6">
                    <iframe
                        src="https://www.youtube.com/embed/XBjTWamI4JI?si=-lxsM_OdgvDYqDxw"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-7">
                    <iframe
                        src="https://www.youtube.com/embed/TfxIp_AfoTw?si=QHcderJfKot6DIQE"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-8">
                    <iframe
                        src="https://www.youtube.com/embed/d1eFDhfsSg4?si=jJ4XdEFpiXjL3d3C"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-9">
                    <iframe
                        src="https://www.youtube.com/embed/GXXcPsvp1C8?si=t2wRgCfl3Ib_a93v"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
                <div class="youtube-video video-10">
                    <iframe
                        src="https://www.youtube.com/embed/EhEZyFIRvqI?si=soB6emQlo1a8XDCd"
                        title="YouTube video player"
                        frameborder="0"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowfullscreen
                    ></iframe>
                </div>
            </div>
        </div>
    }
}
