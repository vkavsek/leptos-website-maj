use leptos::ev::MouseEvent;
use leptos::{html::Audio, prelude::*};

// ------> AUDIO PLAYER
#[component]
pub fn AudioPlayer() -> impl IntoView {
    // Signals
    let (time, set_time) = signal(0u64);
    let (duration, set_duration) = signal(0u64);
    let (play_btn_class, set_play_btn_class) = signal("play");
    // Derived signals
    let time_fmt = move || fmt_sec_to_mmss(time.get());
    let duration_fmt = move || fmt_sec_to_mmss(duration.get());
    // NodeRefs
    let audio_ref = NodeRef::<Audio>::new();

    let sync_metadata = move || {
        let audio = audio_ref.get().unwrap();
        let duration = audio.duration();
        set_duration.set(if duration.is_finite() {
            duration.max(0.0) as u64
        } else {
            0
        });
    };
    // Metadata may already have loaded before hydration attaches event handlers.
    audio_ref.on_load(move |audio| {
        audio.set_volume(0.5);
        sync_metadata();
        set_time.set(audio.current_time() as u64);
        set_play_btn_class.set(if audio.paused() || audio.ended() {
            "play"
        } else {
            "pause"
        });
    });

    // Play
    let play_click = move |_: MouseEvent| {
        let audio = audio_ref.get().unwrap();

        if audio.paused() {
            if let Ok(promise) = audio.play() {
                leptos::task::spawn_local(async move {
                    if web_sys::js_sys::futures::JsFuture::from(promise)
                        .await
                        .is_err()
                    {
                        set_play_btn_class.set("play");
                    }
                });
            }
        } else {
            let _ = audio.pause();
        }
    };

    let seek = move |ev| {
        let audio = audio_ref.get().unwrap();
        let time_to_seek = event_target::<web_sys::HtmlInputElement>(&ev).value_as_number();
        audio.set_current_time(time_to_seek);
        set_time.set(time_to_seek as u64);
    };

    view! {
        <div class="audio-player">
            <audio
                node_ref=audio_ref
                src="/music/MINORFLAW_albumpromo.mp3"
                on:loadedmetadata=move |_| sync_metadata()
                on:durationchange=move |_| sync_metadata()
                on:timeupdate=move |_| {
                    if let Some(audio) = audio_ref.get() {
                        set_time.set(audio.current_time() as u64);
                    }
                }
                on:play=move |_| set_play_btn_class.set("pause")
                on:pause=move |_| set_play_btn_class.set("play")
                on:ended=move |_| set_play_btn_class.set("play")
                on:error=move |_| set_play_btn_class.set("play")
            >
                "Audio Player"
            </audio>
            <div class="play-container">
                <button
                    class=play_btn_class
                    on:click=play_click
                    title="Play / Pause"
                    aria-label=move || if play_btn_class.get() == "play" { "Play" } else { "Pause" }
                ></button>
            </div>
            <div class="time-container">
                <input
                    class="timeline"
                    type="range"
                    min="0"
                    max=move || duration.get()
                    step="1"
                    prop:value=move || time.get()
                    disabled=move || duration.get() == 0
                    aria-label="Playback position"
                    aria-valuetext=move || format!("{} of {}", time_fmt(), duration_fmt())
                    on:input=seek
                />
                <div class="time">
                    <div class="current-time">{time_fmt}</div>
                    <div class="duration-time">{duration_fmt}</div>
                </div>
            </div>
        </div>
    }
}

// ###################################
// ->   UTILS
// ###################################
/// Seconds to MM:SS
/// 62 -> 01:02
fn fmt_sec_to_mmss(time: u64) -> String {
    format!(
        "{}:{}",
        if (time / 60) < 10 {
            format!("0{}", time / 60)
        } else {
            (time / 60).to_string()
        },
        if (time % 60) < 10 {
            format!("0{}", time % 60)
        } else {
            (time % 60).to_string()
        },
    )
}
