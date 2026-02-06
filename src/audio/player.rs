use super::Song;
use crate::app::use_interval;
use leptos::ev::MouseEvent;
use leptos::{
    html::{Audio, Button, Div},
    prelude::*,
};

// ------> AUDIO PLAYER
#[component]
pub fn AudioPlayer() -> impl IntoView {
    // Signals
    let (time, set_time) = signal(0u64);
    let (duration, set_duration) = signal(0u64);
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
    let vol_percent_ref = NodeRef::<Div>::new();
    let play_btn_ref = NodeRef::<Button>::new();
    let timeline_ref = NodeRef::<Div>::new();
    let volume_slider_ref = NodeRef::<Div>::new();
    let progress_ref = NodeRef::<Div>::new();

    // Import the selector and the List of songs
    let _names = use_context::<RwSignal<Vec<String>>>().expect("the names to be provided");
    let selector = use_context::<RwSignal<usize>>().expect("the selector to be provided");

    let f64_to_u64 = |f: f64| (f * 100.0) as u64;

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
            let progress_bar = progress_ref.get_untracked().unwrap();
            let _ = progress_bar.style((
                "width",
                format!("{}%", audio.current_time() / audio.duration() * 100.0),
            ));
            set_time.set(audio.current_time() as u64);
            if audio.ended() {
                set_play_btn_class.set("play");
            }
            if let Some(_slider) = volume_slider_ref.get_untracked() {
                if let Some(vol_percent) = vol_percent_ref.get_untracked() {
                    let new_volume = audio.volume();
                    let _ =
                        vol_percent.style(("width", format!("{}%", (new_volume * 100.0) as u32)));
                }
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

    // When content loads
    let audio_load = move |_| {
        let vol_percent = vol_percent_ref.get().unwrap();
        let audio = audio_ref.get().unwrap();
        let init_volume = 0.50;

        audio.set_volume(init_volume);
        set_duration.set(audio.duration() as u64);
        set_name.set(extract_name(audio.src()));
        let _ = vol_percent.style(("width", format!("{}%", (init_volume * 100.0) as u32)));
    };

    // Play
    let play_click = move |_: MouseEvent| {
        let audio = audio_ref.get().unwrap();

        let play_btn = play_btn_ref.get().unwrap();
        if audio.paused() {
            set_play_btn_class.set("pause");
            let _ = audio.play();
        } else {
            set_play_btn_class.set("play");
            let _ = audio.pause();
        }
    };

    // Click timeline to skip around
    let timeline_click = move |ev: MouseEvent| {
        let timeline = timeline_ref.get().unwrap();
        let audio = audio_ref.get().unwrap();

        let t_width = window()
            .get_computed_style(&timeline)
            .unwrap_or(None)
            .expect("style should be initialized")
            .get_property_value("width")
            .expect("width should be initialized");
        let time_to_seek = ev.offset_x() as f64 / px_to_f64(&t_width) * audio.duration();
        audio.set_current_time(time_to_seek);
    };

    // Volume slider
    let volume_slide_click = move |ev: MouseEvent| {
        let slider = volume_slider_ref.get().unwrap();
        let audio = audio_ref.get().unwrap();
        let vol_percent = vol_percent_ref.get().unwrap();

        if let Ok(Some(slider_width)) = window().get_computed_style(&slider) {
            let slider_width = slider_width
                .get_property_value("width")
                .expect("widht should be initialized");
            let new_volume = ev.offset_x() as f64 / px_to_f64(&slider_width);
            if new_volume > 0.0 {
                audio.set_muted(false)
            }
            audio.set_volume(new_volume);
            let _ = vol_percent.style(("width", format!("{}%", (new_volume * 100.0) as u32)));
        }
    };

    // Volume Button
    let vol_button_click = move |_| {
        let audio = audio_ref.get().unwrap();
        let vol_percent = vol_percent_ref.get().unwrap();

        audio.set_muted(!audio.muted());

        if audio.muted() {
            let _ = vol_percent.style(("width", "0%"));
            set_vol_btn_class.set("ico-vol-mute");
        } else {
            let volume = f64_to_u64(audio.volume());
            let _ = vol_percent.style(("width", format!("{}%", volume)));
            set_vol_btn_class.set("ico-vol-med");
        }
    };

    view! {
        <div class="audio-player">
            <audio node_ref=audio_ref on:loadeddata=audio_load src=song_src>
                "Audio Player"
            </audio>
            <div class="play-container">
                <button
                    class=play_btn_class
                    on:click=play_click
                    node_ref=play_btn_ref
                    title="Play / Pause"
                ></button>
            </div>
            <div class="time-container">
                <div class="timeline" node_ref=timeline_ref on:click=timeline_click>
                    <div class="progress" node_ref=progress_ref></div>
                </div>
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
                    ></button>
                    <div
                        class="volume-slider"
                        on:click=volume_slide_click
                        node_ref=volume_slider_ref
                    >
                        <div class="volume-percentage" node_ref=vol_percent_ref></div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ###################################
// ->   UTILS
// ###################################
fn extract_name(path: String) -> Option<String> {
    let res = path.split('/').last().map(|name| name.split('.').next());
    match res {
        Some(Some(name)) => Some(name.to_string()),
        _ => None,
    }
}

fn px_to_f64(px: &str) -> f64 {
    px.split_at(px.len() - 2)
        .0
        .parse::<f64>()
        .expect("to_be_parsable")
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
