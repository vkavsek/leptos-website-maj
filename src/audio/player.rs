use super::Song;
use crate::app::use_interval;
use leptos::ev::MouseEvent;
use leptos::{html::Audio, prelude::*};

// ------> AUDIO PLAYER
#[component]
pub fn AudioPlayer() -> impl IntoView {
    // Signals
    let (time, set_time) = signal(0u64);
    let (duration, set_duration) = signal(0u64);
    let (volume, set_volume) = signal(0.5f64);
    let (name, set_name) = signal::<Option<String>>(None);
    let (play_btn_class, set_play_btn_class) = signal("pause");
    let (vol_btn_class, set_vol_btn_class) = signal("ico-vol-med");
    // Derived signals
    let _song_title = move || Song::from_filenamename(&name.get().unwrap_or(String::new())).title();
    let _song_artist =
        move || Song::from_filenamename(&name.get().unwrap_or(String::new())).artist();
    let _song_album = move || Song::from_filenamename(&name.get().unwrap_or(String::new())).album();
    let time_fmt = move || fmt_sec_to_mmss(time.get());
    let duration_fmt = move || fmt_sec_to_mmss(duration.get());
    // NodeRefs
    let audio_ref = NodeRef::<Audio>::new();

    // Import the selector and the List of songs
    let _names = use_context::<RwSignal<Vec<String>>>().expect("the names to be provided");
    let selector = use_context::<RwSignal<usize>>().expect("the selector to be provided");

    // Set Song Source
    let song_src = move || {
        // if let Some(name) = names.get().get(selector.get()) {
        //     format!("/music/{}", name)
        // } else {
        // "/music/audio-promo_small.mp3".to_string()
        // }
        "/music/MINORFLAW_albumpromo.mp3"
    };

    // DOM should be built by the time we call the closures so we unwrap them all
    //
    // Check audio percentage and update time accordingly
    let audio_loop = move || {
        if let Some(audio) = audio_ref.get_untracked() {
            set_time.set(audio.current_time() as u64);
            if audio.ended() {
                set_play_btn_class.set("play");
            }
        }
    };
    // NOTE: here the main audio loop gets initialized
    use_interval(10, audio_loop);

    // Reset play button when you change the song
    Effect::new_isomorphic(move |_| {
        // subscribe to selector
        let _ = selector.get();
        set_play_btn_class.set("play");
        if let Some(audio) = audio_ref.get() {
            let _ = audio.pause();
        }
    });

    let sync_metadata = move || {
        let audio = audio_ref.get().unwrap();
        set_duration.set(audio.duration() as u64);
        set_name.set(extract_name(audio.src()));
    };
    // Metadata may already have loaded before hydration attaches event handlers.
    audio_ref.on_load(move |audio| {
        audio.set_volume(0.5);
        sync_metadata();
    });

    // Play
    let play_click = move |_: MouseEvent| {
        let audio = audio_ref.get().unwrap();

        set_duration.set(audio.duration() as u64);
        if audio.paused() {
            set_play_btn_class.set("pause");
            let _ = audio.play();
        } else {
            set_play_btn_class.set("play");
            let _ = audio.pause();
        }
    };

    let seek = move |ev| {
        let audio = audio_ref.get().unwrap();
        let time_to_seek = event_target::<web_sys::HtmlInputElement>(&ev).value_as_number();
        audio.set_current_time(time_to_seek);
        set_time.set(time_to_seek as u64);
    };

    let change_volume = move |ev| {
        let audio = audio_ref.get().unwrap();
        let new_volume = event_target::<web_sys::HtmlInputElement>(&ev).value_as_number();
        if new_volume > 0.0 {
            audio.set_muted(false);
        }
        audio.set_volume(new_volume);
        set_volume.set(new_volume);
    };

    let sync_volume = move |_| {
        let audio = audio_ref.get().unwrap();
        if audio.muted() {
            set_volume.set(0.0);
            set_vol_btn_class.set("ico-vol-mute");
        } else {
            set_volume.set(audio.volume());
            set_vol_btn_class.set("ico-vol-med");
        }
    };

    let vol_button_click = move |_| {
        let audio = audio_ref.get().unwrap();
        audio.set_muted(!audio.muted());
    };

    view! {
        <div class="audio-player">
            <audio node_ref=audio_ref on:loadeddata=move |_| sync_metadata() on:volumechange=sync_volume src=song_src>
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
            <div class="controls">
                <div class="song-info"></div>
                <div class="volume-container">
                    <button
                        class=vol_btn_class
                        on:click=vol_button_click
                        title="Volume / Mute"
                        aria-label=move || if vol_btn_class.get() == "ico-vol-mute" { "Unmute" } else { "Mute" }
                    ></button>
                    <input
                        class="volume-slider"
                        type="range"
                        min="0"
                        max="1"
                        step="0.01"
                        prop:value=move || volume.get()
                        aria-label="Volume"
                        aria-valuetext=move || format!("{}%", (volume.get() * 100.0).round() as u32)
                        on:input=change_volume
                    />
                </div>
            </div>
        </div>
    }
}

// ###################################
// ->   UTILS
// ###################################
fn extract_name(path: String) -> Option<String> {
    let res = path
        .split('/')
        .next_back()
        .map(|name| name.split('.').next());
    match res {
        Some(Some(name)) => Some(name.to_string()),
        _ => None,
    }
}

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
