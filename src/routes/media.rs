use leptos::*;
use leptos_meta::{Link, Title};
use std::fmt::Write;

#[component]
pub fn Media() -> impl IntoView {
    let temp_src = [
        "philosopher_stoned.m4v",
        "hexagonal_mirrors.m4v",
        "hand_in_a_hat.m4v",
    ];

    let class = |i: usize| format!("video-js {}", if i % 2 == 1 { "video-move" } else { "" });
    let videos = temp_src
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let id = format!("nat-vid-{}", i);
            view! {
                <div class="video-wrapper">
                    <video class=class(i) id=id.clone() controls preload="auto">
                        <source src=format!("/videos/{}", name) type="video/mp4"/>
                        <p class="vjs-no-js">
                            "To view this video please enable JavaScript, and consider upgrading to a
                            web browser that supports HTML5 video."
                        </p>
                    </video>
                </div>
            }
        })
        .collect_view();

    let video_init_script = temp_src
        .iter()
        .enumerate()
        .fold(String::new(), |mut acc, (i, _)| {
            write!(&mut acc, "videojs('nat-vid-{}', {{}});", i).expect("write failed");
            acc
        });

    view! {
        <Link rel="icon" href="/img/media.svg" type_="image/svg"/>
        <Link href="/video-js/video-js.css" rel="stylesheet"/>
        <Link href="/video-js/videojs-matrix.css" rel="stylesheet"/>
        <Title text="Media"/>

        <div class="components" id="media-components">
            <div class="title" id="media-title">
                // <img class="title-img" id="media-img" src="/img/titles/media.svg"/>
                <h1>"Media"</h1>
            </div>
            <div class="contents" id="media-contents">
                {videos}
            </div>
        </div>
        // Include Video.js library
        <script src="/video-js/video.min.js"></script>
        <script>{video_init_script}</script>
        // <script>"videojs('nat-vid-0', {});"</script>
    }
}

/*
*       NOTE: Youtube videos to use if needed!
*
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
*/
