use leptos::prelude::*;

use leptos_meta::{Link, Title};
use leptos_router::components::A;
use leptos_router::{hooks::use_params, params::Params};

const ALBUMS: [&str; 6] = [
    "minorflaw",
    "galaterna",
    "galaterna",
    "minorflaw",
    "minorflaw",
    "galaterna",
];
const GUEST_APPEARANCES: [&str; 3] = ["Test Album: 2026 (trumpet)",
    "11111111111111111111111111111111111111111111111111111111111111111111111111111111111 2026 (truempt)",
    "Gost Gost"
];

#[component]
pub fn Discography() -> impl IntoView {
    let albums = ALBUMS
        .iter()
        .map(|name| {
            let ae = AlbumEntry::init_from_str(*name).expect("valid name in ALBUMS const");

            let addr = ae.title.to_lowercase().replace(" ", "");
            view! {
                <A href=addr.clone() attr:class="discography-link">
                    <div id="disco-link-container">
                        <img
                            class=format!("album-artwork-{addr}")
                            src=ae.img_path
                            alt=format!("{} Album Artwork", ae.title)
                            width=ae.img_size.0
                            height=ae.img_size.1
                        />
                        <p>{ae.title}</p>
                        <p>{ae.release_year}</p>
                    </div>
                </A>
            }
        })
        .collect::<Vec<_>>();

    let guest_appearances_html = GUEST_APPEARANCES
        .iter()
        .map(|&ap| view!(<p>{ap}</p>))
        .collect_view();

    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg" />
        <Title text="Discography" />
        <div class="components" id="disco-components">
            <div class="title" id="disco-title">
                <h1>"Discography"</h1>
            </div>
            <div class="contents" id="disco-wrap">
                <div id="leader-wrap">
                    <h4 id="disco-heading">"As leader:"</h4>
                    <div id="album-wrap">{albums}</div>
                </div>
                <div id="guest-wrap">
                    <h4 id="disco-heading">"As guest:"</h4>
                    <div id="guest-album-wrap">{guest_appearances_html}</div>
                </div>
            </div>
        </div>
    }
}

#[derive(Params, PartialEq)]
struct NameParams {
    name: Option<String>,
}

#[component]
pub fn DiscographyDisplayAlbum() -> impl IntoView {
    // extract the name from the query
    let params = use_params::<NameParams>();
    // try to match the extracted name to the album
    let album = move || {
        let name = params
            .read()
            .as_ref()
            .ok()
            .and_then(|p| p.name.to_owned())
            .unwrap_or_default();
        AlbumEntry::init_from_str(name)
    };

    let n = move || {
        album()
            .map(|a| a.title.to_string())
            .unwrap_or("Error".to_string())
    };
    view! {
        <div>"Albums "{n}</div>
        <A href="/discography">"Back"</A>
    }
}

struct AlbumEntry<'a> {
    img_path: &'a str,
    img_size: (u16, u16),
    release_year: u16,
    label: &'a str,
    title: &'a str,
    notes: Vec<&'a str>,
    links: Vec<LinkLogo<'a>>,
}

struct LinkLogo<'b> {
    link: &'b str,
    logo_path: &'b str,
    logo_size_xy: (u16, u16),
}

impl<'a> AlbumEntry<'a> {
    fn new(
        img_path: &'a str,
        img_size: (u16, u16),
        release_year: u16,
        label: &'a str,
        title: &'a str,
        notes: Vec<&'a str>,
        links: Vec<LinkLogo<'a>>,
    ) -> Self {
        Self {
            img_path,
            img_size,
            title,
            links,
            release_year,
            label,
            notes,
        }
    }

    fn init_from_str(src: impl AsRef<str>) -> Option<Self> {
        match src.as_ref().to_lowercase().as_str() {
            "minorflaw" => Some(AlbumEntry::new(
                "/img/MINORFLAW-01_600p.webp",
                (600, 600),
                2026,
                "Jazz Cerkno Records",
                "Minor Flaw",
                vec!["MINOR FLAW is a collection of compositions inspired by the sounds, noises, melodies, and rhythms of everyday environments. The project explores the lower threshold of sonic perception, capturing the subtle deviations and textures that emerge within dense soundscapes and momentarily disrupt their continuity. Rather than treating these moments as flaws, the project embraces them as authentic gestures through which the music breathes and reveals its human dimension.",
                    "Extending beyond sound, the project reflects on perception in an age oversaturated with information, where the quieter details are often overshadowed by dominant narratives. It suggests that attention to the faint, the imperfect, and the easily overlooked can deepen our understanding of both sonic and social environments.",
                    "Through this approach, MINOR FLAW perceives imperfection as a generative force, that shapes collective interaction, guides real-time decision-making, and anchors the music in the immediacy of performance. In doing so, it invites performers and listeners alike to reconsider their relationship to sound, silence, and the unnoticed.", 
                    "Maj Kavšek - trumpet and compositions<br>Julius Gawlik - tenor saxophone<br>Tjan Šoštarič - double bass<br>Lenny Rehm - drums", 
                    "Recorded live at the 30th Jazz Cerkno Festival during the Jazz incubator #3 music residency on May 25 2025.",
                    "Recorded by Iztok Zupan<br>Mixed by Rok Zalokar<br>Mastered by Alastair McNeill<br>Cover art and design by Ana Govc",
                ],
                vec![LinkLogo::new(
                    "https://jazzcerkno.bandcamp.com/album/minor-flaw",
                    "/img/foreign-logos/bandcamp-fin.svg",
                    (256, 256),
                )],
            )),
            "galaterna" => Some(AlbumEntry::new(
                "/img/album_artwork_360p.webp",
                (360, 360),
                2024,
                "Aut Records",
                "Galaterna",
                vec!["Maj Kavšek - trumpet & flugelhorn, compositions<br>Efim Brailovskiy - alto saxophone<br>Samo Hude – piano<br>Gal Golob - double bass<br>Leo Gerstner - drums",
                    "Recorded by Athanasios Karakantas and Laszlo Griese at BabyBazaar Studio in Berlin on the 20th and 21st of Feb 2023. Mixed and mastered by Athanasios Karakantas, Berlin, November 2023.",
                    "Artwork by Gal Tič<br>Editing by Eva Popit<br>Layout by Davide Lorenzon"
                ],
                vec![
                    LinkLogo::new(
                        "https://autrecords.bandcamp.com/album/galaterna",
                        "/img/foreign-logos/bandcamp-fin.svg",
                        (256, 256),
                    ),
                    LinkLogo::new(
                        "https://www.autrecords.com/",
                        "/img/foreign-logos/aut-fin.svg",
                        (256, 256),
                    ),
                ],
            )),
            _ => None
        }
    }
}

impl<'b> LinkLogo<'b> {
    fn new(link: &'b str, logo_path: &'b str, logo_size_xy: (u16, u16)) -> Self {
        LinkLogo {
            link,
            logo_path,
            logo_size_xy,
        }
    }
}
