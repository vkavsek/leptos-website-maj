use leptos::prelude::*;

use leptos_meta::{Link, Title};
use leptos_router::components::A;
use leptos_router::{hooks::use_params, params::Params};
use strum_macros::EnumIter;

// ###################################
// ->     COMPONENTS
// ###################################

#[component]
pub fn Discography() -> impl IntoView {
    // FIX: REMOVE and replace with proper Enum iteration
    let albums = ALBUMS
        .iter()
        .map(|name| {
            let album = Album::try_init_from_str(*name).expect("valid name in ALBUMS const");
            let ae = Album::get_entry(&album);
            let img = Album::get_image(&album);

            let addr = ae.title.to_lowercase().replace(" ", "");
            view! {
                <A href=addr.clone() attr:class="discography-link">
                    <div id="disco-link-container">
                        <img
                            class=format!("album-artwork-{addr}")
                            src=img.img_path
                            alt=format!("{} Album Artwork", ae.title)
                            width=img.img_size.0
                            height=img.img_size.1
                        />
                        <p>{ae.title}</p>
                        <p>{ae.release_year}</p>
                    </div>
                </A>
            }
        })
        .collect::<Vec<_>>();

    // let guest_appearances_html = GUEST_APPEARANCES
    //     .iter()
    //     .map(|&ap| view! { <p>" - "{ap}</p> })
    //     .collect_view();

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
            // <div id="guest-wrap">
            // <h4 id="disco-heading">"As guest:"</h4>
            // <div id="guest-album-wrap">{guest_appearances_html}</div>
            // </div>
            </div>
        </div>
    }
}

#[component]
pub fn DiscographyDisplayAlbum() -> impl IntoView {
    // extract the name from the query
    let params = use_params::<NameParams>();
    let name = move || {
        params
            .read()
            .as_ref()
            .ok()
            .and_then(|p| p.name.to_owned())
            .unwrap_or_default()
    };
    let album = move || Album::try_init_from_str(name());
    let ae = move || album().map(|album| Album::get_entry(&album));
    let title = move || ae().map(|ae| ae.title).unwrap_or("Missing Album");
    let content = move || {
        if let (Some(album), Some(ae)) = (album(), ae()) {
            let image = Album::get_image_hi_res(&album);
            let links = Album::get_links(&album);
            let notes = ae
                .notes
                .iter()
                .map(|&n| {
                    if n.is_empty() {
                        view! { <br /> }.into_any()
                    } else {
                        view! { <p>{n}</p> }.into_any()
                    }
                })
                .collect_view();

            view! {
                <div class="album-display-wrap">
                    <div class="album-img-wrap">
                        <img
                            class=format!(
                                "album-artwork-{}",
                                ae.title.to_lowercase().replace(" ", ""),
                            )
                            src=image.img_path
                            alt=format!("{} Album Artwork", ae.title)
                            width=image.img_size.0
                            height=image.img_size.1
                        />
                    </div>
                    <div class="album-desc-wrap">
                        {notes} <br /> <p>{ae.label}</p> <p>{ae.release_year}</p>
                        <A href="/discography">"Back"</A>
                    </div>
                </div>
            }
            .into_any()
        } else {
            view! { <p>"Sorry but the album you are looking for doesn't seem to exist."</p> }
                .into_any()
        }
    };
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg" />
        <Title text=title />
        <div class="components" id="disco-display-components">
            <div class="title" id="disco-display-title">
                <h1>{title}</h1>
            </div>
            {content}
        </div>
    }
}
// ###################################
// ->     STATICS
// ###################################

const ALBUMS: [&str; 2] = ["minorflaw", "galaterna"]; //, "galaterna", "minorflaw"];

// const GUEST_APPEARANCES: [&str; 17] = todo!();

// ###################################
// ->     TYPES & IMPLS
// ###################################

#[derive(Debug, Default, EnumIter)]
enum Album {
    #[default]
    MinorFlaw,
    Galaterna,
}
impl Album {
    fn try_init_from_str(src: impl AsRef<str>) -> Option<Self> {
        match src.as_ref().to_lowercase().as_str() {
            "minorflaw" => Some(Self::MinorFlaw),
            "galaterna" => Some(Self::Galaterna),
            _ => None,
        }
    }

    fn get_image<'a>(from: &Self) -> Image<'a> {
        match from {
            Album::MinorFlaw => Image {
                img_path: "/img/MINORFLAW-01_600p.webp",
                img_size: (600, 600),
            },
            Album::Galaterna => Image {
                img_path: "/img/album_artwork_360p.webp",
                img_size: (360, 360),
            },
        }
    }

    fn get_image_hi_res<'a>(from: &Self) -> Image<'a> {
        match from {
            Album::MinorFlaw => Image {
                img_path: "/img/MINORFLAW-01.webp",
                img_size: (3000, 3000),
            },
            Album::Galaterna => Image {
                img_path: "/img/album_artwork.webp",
                img_size: (1200, 1200),
            },
        }
    }

    fn get_links<'a>(from: &Self) -> Option<Vec<LinkLogo<'a>>> {
        match from {
            Album::MinorFlaw => Some(vec![LinkLogo::new(
                "https://jazzcerkno.bandcamp.com/album/minor-flaw",
                "/img/foreign-logos/bandcamp-fin.svg",
                (256, 256),
            )]),
            Album::Galaterna => Some(vec![
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
            ]),
        }
    }

    fn get_entry<'a>(from: &Self) -> AlbumEntry<'a> {
        match from {
            Album::MinorFlaw => AlbumEntry::new(
                2026,
                "Jazz Cerkno Records",
                "Minor Flaw",
                vec!["MINOR FLAW is a collection of compositions inspired by the sounds, noises, melodies, and rhythms of everyday environments. The project explores the lower threshold of sonic perception, capturing the subtle deviations and textures that emerge within dense soundscapes and momentarily disrupt their continuity. Rather than treating these moments as flaws, the project embraces them as authentic gestures through which the music breathes and reveals its human dimension.",
                    "Extending beyond sound, the project reflects on perception in an age oversaturated with information, where the quieter details are often overshadowed by dominant narratives. It suggests that attention to the faint, the imperfect, and the easily overlooked can deepen our understanding of both sonic and social environments.",
                    "Through this approach, MINOR FLAW perceives imperfection as a generative force, that shapes collective interaction, guides real-time decision-making, and anchors the music in the immediacy of performance. In doing so, it invites performers and listeners alike to reconsider their relationship to sound, silence, and the unnoticed.", 
                    "",
                    "Maj Kavšek - trumpet and compositions",
                    "Julius Gawlik - tenor saxophone","Tjan Šoštarič - double bass", "Lenny Rehm - drums", 
                    "",
                    "Recorded live at the 30th Jazz Cerkno Festival during the Jazz incubator #3 music residency on May 25 2025.",
                    "Recorded by Iztok Zupan", "Mixed by Rok Zalokar", "Mastered by Alastair McNeill", "Cover art and design by Ana Govc",
                ],
            ),
            Album::Galaterna => AlbumEntry::new(
                2024,
                "Aut Records",
                "Galaterna",
                vec![
                    r#"Maj Kavšek’s inaugural album, “Galaterna" is a testament to his multifaceted talent as a producer, composer, and arranger. The coined title encapsulates the project's ethos—creating something unique while honouring its musical lineage. Within the album, a spectrum of compositions unfolds, drawing inspiration from diverse sources. Traditional jazz motifs pulse with vitality and rhythm, while open harmonies and intricate textures add layers of intrigue. Each piece is a tribute to the artists who have profoundly influenced Kavšek's musical journey. The result is a musical odyssey that seamlessly merges innovation with a profound reverence for the traditions that underpin the quintet's distinctive sound."#,
                    "",
                    "Maj Kavšek - trumpet & flugelhorn, compositions", "Efim Brailovskiy - alto saxophone", "Samo Hude – piano", "Gal Golob - double bass", "Leo Gerstner - drums",
                    "",
                    "Recorded by Athanasios Karakantas and Laszlo Griese at BabyBazaar Studio in Berlin on the 20th and 21st of Feb 2023. Mixed and mastered by Athanasios Karakantas, Berlin, November 2023.",
                    "",
                    "Artwork by Gal Tič", "Editing by Eva Popit", "Layout by Davide Lorenzon"
                ],
            ),
        }
    }
}

struct Image<'a> {
    img_path: &'a str,
    img_size: (u16, u16),
}

struct LinkLogo<'b> {
    link: &'b str,
    logo_path: &'b str,
    logo_size_xy: (u16, u16),
}

struct AlbumEntry<'a> {
    release_year: u16,
    label: &'a str,
    title: &'a str,
    notes: Vec<&'a str>,
}

impl<'a> AlbumEntry<'a> {
    fn new(release_year: u16, label: &'a str, title: &'a str, notes: Vec<&'a str>) -> Self {
        Self {
            title,
            release_year,
            label,
            notes,
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

#[derive(Params, PartialEq)]
struct NameParams {
    name: Option<String>,
}
