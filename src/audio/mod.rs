use leptos::prelude::*;

mod list;
mod player;

#[allow(unused)]
use list::AudioList;
use player::AudioPlayer;

#[component]
pub fn AudioWrapper() -> impl IntoView {
    let names = vec![
        "Practical_Anxiety_MASTER.mp3",
        "Aya_Sean_edit-Mark_Babin.mp3",
        "Silicon-Mark.mp3",
    ];
    let names = names
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<String>>();

    // Create signals
    let names = RwSignal::new(names);
    let selector = RwSignal::new(0usize);
    // Provide names and selector to all the children of AudioWrapper
    provide_context(names);
    provide_context(selector);

    view! {
        <AudioPlayer/>
        //<AudioList/>
    }
}

/// A song info struct
pub struct Song {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
}

impl Song {
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }
    pub fn artist(&self) -> Option<String> {
        self.artist.clone()
    }
    pub fn album(&self) -> Option<String> {
        self.album.clone()
    }

    /// Extracts song info from the filename in the form of:
    /// `Song_Title-Song_Artist-Song_Album.filetype`
    /// Fields are separated by '-' and spaces are replaced with '_'.
    pub fn from_filenamename(filename: &str) -> Self {
        let spaces = |filenamename: &str| {
            filenamename
                .split('.')
                .nth(0)
                .unwrap_or("")
                .split('-')
                .map(|cont| cont.replace('_', " "))
                .collect::<Vec<_>>()
        };
        let parts = spaces(filename);

        Self {
            title: parts.first().map(|s| s.to_owned()),
            artist: parts.get(1).map(|s| s.to_owned()),
            album: parts.get(2).map(|s| s.to_owned()),
        }
    }
}
