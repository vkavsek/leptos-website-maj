#![allow(unused)]
use super::Song;
use crate::app::use_interval;
use leptos::ev::MouseEvent;
use leptos::{
    html::{Audio, Button, Div},
    *,
};
use leptos_use::UseRafFnOptions;
use leptos_use::{use_raf_fn, use_raf_fn_with_options, utils::Pausable};
use std::rc::Rc;

// ------> AUDIO PLAYER
// TODO: If end of song next song or make button PAUSE
#[component]
pub fn AudioPlayer() -> impl IntoView {
    // Signals
    let (time, set_time) = create_signal(0u64);
    let (duration, set_duration) = create_signal(0u64);
    let (name, set_name) = create_signal::<Option<String>>(None);
    // Derived signals
    let song_title = move || Song::from_filenamename(&name.get().unwrap_or(String::new())).title();
    let song_artist =
        move || Song::from_filenamename(&name.get().unwrap_or(String::new())).artist();
    let song_album = move || Song::from_filenamename(&name.get().unwrap_or(String::new())).album();
    let time_fmt = move || fmt_sec_to_mmss(time.get());
    let duration_fmt = move || fmt_sec_to_mmss(duration.get());
    // NodeRefs
    let audio_ref = create_node_ref::<Audio>();
    let vol_percent_ref = create_node_ref::<Div>();
    let play_btn_ref = create_node_ref::<Button>();
    let timeline_ref = create_node_ref::<Div>();
    let volume_slider_ref = create_node_ref::<Div>();
    let vol_el_ref = create_node_ref::<Button>();
    let progress_ref = create_node_ref::<Div>();

    // Import the selector and the List of songs
    let names = use_context::<RwSignal<Vec<String>>>().expect("the names to be provided");
    let selector = use_context::<RwSignal<usize>>().expect("the selector to be provided");

    let f64_to_u64 = |f: f64| (f * 100.0) as u64;

    // Set Song Source
    let song_src = move || {
        // if let Some(name) = names.get().get(selector.get()) {
        //     format!("/music/{}", name)
        // } else {
        // "/music/audio-promo_small.mp3".to_string()
        // }
        "/music/audio-promo_small.mp3"
    };

    // DOM should be built by the time we call the closures so we unwrap them all
    //
    // Check audio percentage and update time accordingly
    let audio_loop = move || {
        if let Some(audio) = audio_ref.get_untracked() {
            let progress_bar = progress_ref.get_untracked().unwrap();
            progress_bar.style(
                "width",
                format!("{}%", audio.current_time() / audio.duration() * 100.0),
            );
            set_time.set(audio.current_time() as u64);
            if audio.ended() {
                let play_btn = play_btn_ref.get_untracked().unwrap();
                play_btn.clone().class("pause", false);
                play_btn.class("play", true);
            }
            if let Some(slider) = volume_slider_ref.get_untracked() {
                if let Some(vol_percent) = vol_percent_ref.get_untracked() {
                    let new_volume = audio.volume();
                    vol_percent.style("width", format!("{}%", (new_volume * 100.0) as u32));
                }
            }
        }
    };
    // NOTE: here the main audio loop gets initialized
    use_interval(10, audio_loop);

    // TODO: this only runs on CSR what if SSR ?
    // Reset play button when you change the song
    create_effect(move |_| {
        // subscribe to selector
        let _ = selector.get();
        if let Some(play_btn) = play_btn_ref.get() {
            play_btn.clone().class("pause", false);
            play_btn.class("play", true);
        }
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
        vol_percent.style("width", format!("{}%", (init_volume * 100.0) as u32));
    };

    // Play
    let play_click = move |_: MouseEvent| {
        let audio = audio_ref.get().unwrap();

        let play_btn = play_btn_ref.get().unwrap();
        if audio.paused() {
            play_btn.clone().class("play", false);
            play_btn.clone().class("pause", true);
            let _ = audio.play();
        } else {
            play_btn.clone().class("pause", false);
            play_btn.class("play", true);
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
            .expect("the style to be calculated")
            .get_property_value("width")
            .expect("the width to be calculated");
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
                .expect("the width to be calculated");
            let new_volume = ev.offset_x() as f64 / px_to_f64(&slider_width);
            if new_volume > 0.0 {
                audio.set_muted(false)
            }
            audio.set_volume(new_volume);
            vol_percent.style("width", format!("{}%", (new_volume * 100.0) as u32));
        }
    };

    // Volume Button
    let vol_button_click = move |_| {
        let audio = audio_ref.get().unwrap();
        let vol_el = vol_el_ref.get().unwrap();
        let vol_percent = vol_percent_ref.get().unwrap();

        audio.set_muted(!audio.muted());

        if audio.muted() {
            vol_percent.style("width", "0%");
            vol_el.clone().class("ico-vol-med", false);
            vol_el.clone().class("ico-vol-mute", true);
        } else {
            let volume = f64_to_u64(audio.volume());
            vol_percent.style("width", format!("{}%", volume));
            vol_el.clone().class("ico-vol-mute", false);
            vol_el.class("ico-vol-med", true);
        }
    };

    // TODO: add NextSong, PrevSong buttons?
    view! {
        <div class="audio-player">
            <audio node_ref=audio_ref on:loadeddata=audio_load src=song_src>
                "Audio Player"
            </audio>
            <div class="play-container">
                <button
                    class="play"
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
                        class="ico-vol-med"
                        on:click=vol_button_click
                        node_ref=vol_el_ref
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
