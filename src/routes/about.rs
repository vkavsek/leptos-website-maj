use leptos::{
    html::{Button, Div},
    prelude::*,
    tachys::dom::window,
    wasm_bindgen::JsValue,
};
use leptos_meta::{Link, Title};

#[allow(unused)]
enum Orient {
    Portrait,
    Landscape,
}
#[allow(unused)]
impl Orient {
    pub fn get() -> Result<Self, JsValue> {
        let in_width = window()
            .inner_width()?
            .as_f64()
            .expect("width should be a number");
        let in_height = window()
            .inner_height()?
            .as_f64()
            .expect("height should be a number");
        if in_width > in_height {
            Ok(Orient::Landscape)
        } else {
            Ok(Orient::Portrait)
        }
    }
}
enum Color {
    #[allow(dead_code)]
    Yellow,
    Pink,
}
impl Color {
    fn return_val(&self) -> &'static str {
        match self {
            Color::Yellow => "#fad549",
            Color::Pink => "#ff3988",
        }
    }
    fn switch(&mut self) {
        match self {
            Color::Yellow => {
                *self = Color::Pink;
            }
            Color::Pink => {
                *self = Color::Yellow;
            }
        }
    }
    fn other(&self) -> Self {
        match self {
            Color::Yellow => Color::Pink,
            Color::Pink => Color::Yellow,
        }
    }
}

#[component]
pub fn About() -> impl IntoView {
    let text_content = [
        view! {<strong>"Maj Kavšek"</strong>" was born on April 12, 2000, in Ljubljana, Slovenia. His musical journey began at 8 when he first picked up the trumpet, displaying a deep passion for music early on. This enthusiasm led him to pursue formal music education, and by 15, he enrolled at the Conservatorium for Music in Ljubljana, where he specialized in jazz. His time at the Conservatorium was important in shaping his musical identity and solidifying his commitment to music."}.into_any(),
        view! {"After completing his studies in Ljubljana, Maj decided to further his musical education by enrolling at the prestigious "<mark>"Jazz Institut Berlin (UdK Berlin - University of Arts Berlin)"</mark>". This move allowed him to expand his musical knowledge and study under renowned professors, including James Robert Rotondi, Nate Wooley, Ralph Alessi, Greg Cohen, and Reggie Moore. His studies in Berlin gave him the opportunity to explore a wide range of musical ideas and develop his skills in both performance and composition."}.into_any(),
        view! {"Throughout his career, Maj has gained extensive experience in a variety of musical settings. He has performed with symphonic and wind orchestras, classical chamber music groups, jazz big bands, and small ensembles. His versatility has allowed him to excel in numerous contexts, from large ensemble performances to intimate solo concerts. As a soloist, Maj has had the honor of performing with the "<mark>"Big Band Radio & Television Slovenia"</mark>" on multiple occasions, showcasing his talent."}.into_any(),
        view! {"In 2019, Maj’s quintet won "<strong>"1st prize at the TEMSIG Musical Competition"</strong>", an event that recognized his musicianship and leadership. His success continued to build when, in 2023, he was invited to perform with the "<mark>"Euroradio Jazz Orchestra"</mark>" in Lithuania and "<mark>"Fame's Institute Orchestra"</mark>" in North Macedonia."}.into_any(),
        view! {"Maj has also been an active participant in various jazz festivals, where he has performed both as a bandleader and a side-man. These festivals have given him the opportunity to collaborate with many esteemed musicians, including "<mark>"Chris Pitsiokos"</mark>", "<mark>"Philipp Gropper"</mark>", "<mark>"Dré Hocevar"</mark>", "<mark>"Bram De Looze"</mark>", "<mark>"Felix Henkelhausen"</mark>", and "<mark>"Jan Bang"</mark>", among others. Through these collaborations, Maj has demonstrated his adaptability and ability to work in diverse musical environments."}.into_any(),
        view! {"In 2024, Maj released his debut album, "<mark>"Galaterna"</mark>", recorded with his quintet and released under the "<mark>"Aut Records"</mark>" label. The album blends the energetic, forward-thinking spirit of modern jazz with elements drawn from old-school American jazz traditions. "<q><strong>"Galaterna"</strong></q>" has been well received by critics, with the album praised for its dynamic and expressive qualities."}.into_any(),
        view! {"Pierre Dulieu of JazzMania wrote on September 27, 2024 "<cite><q>"This energetic and warm music, tailor-made for club performances, radiates freshness, features generous solos (listening to the trumpeter soar on 'Philosopher’s Stoned' is a treat), and boasts collective playing confidence that makes it exhilarating. Definitely worth a listen!"</q></cite>}.into_any(),
        view! {"Additionally, Jan Granlie of Salt Peanuts noted on July 5, 2024, that the album "<cite><q>"showcases exceptionally well-played music that draws threads back to the American jazz traditions of the 1960s, while maintaining a clear forward-looking perspective."</q></cite>}.into_any(),
        view! {"Through his dedication to his craft and his passion for music, Maj Kavšek continues to make a mark on the international music scene. His performances, recordings, and collaborations showcase his versatility and his ability to inspire both audiences and fellow musicians alike."}.into_any(),
    ];

    let text_content = text_content
        .map(|line| view! { <p>{line}</p> })
        .collect_view();

    let mut color = Color::Pink;
    let init_color = format!("color: {}", color.return_val());
    let init_bg_color = format!("background-color: {}", color.return_val());
    let init_next_bg_color = format!("background-color: {}", (color.other()).return_val());

    let text_ref: NodeRef<Div> = NodeRef::new();
    let button_ref: NodeRef<Button> = NodeRef::new();
    let curr_col_ref: NodeRef<Div> = NodeRef::new();
    let next_col_ref: NodeRef<Div> = NodeRef::new();

    let change_color = move |_| {
        let text_div = text_ref.get().expect("the DOM should be built by now");
        let button = button_ref.get().unwrap();
        let current_color = curr_col_ref.get().unwrap();
        let next_color = next_col_ref.get().unwrap();
        color.switch();
        let cd = color.return_val();
        let next_cd = (color.other()).return_val();
        let _ = text_div.style(("color", cd));
        let _ = button.style(("color", cd));
        let _ = current_color.style(("background-color", cd));
        let _ = next_color.style(("background-color", next_cd));
    };

    view! {
        <Link rel="icon" href="/img/izkaznica.svg" type_="image/svg" />
        <Title text="About Me" />
        <div class="components" id="about-components">
            <div class="title" id="about-title">
                <h1>"About Me"</h1>
            </div>
            <div class="contents" id="about-wrap">
                <ImagesAbout />
                <div class="about-text" style=init_color.clone() node_ref=text_ref>
                    {text_content}
                    <button
                        class="change-color-button"
                        on:click=change_color
                        style=init_color.clone()
                        node_ref=button_ref
                    >
                        <div class="color-box" style=init_bg_color node_ref=curr_col_ref></div>
                        <img src="/img/icon/back.svg" class="button-next-arrow" />
                        <div class="color-box" style=init_next_bg_color node_ref=next_col_ref></div>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn ImagesAbout() -> impl IntoView {
    let files = Resource::new(
        move || (),
        |_| async move { read_image_files().await.unwrap() },
    );

    // This is named like this because we get warnings because of macros.
    let _gallery_ref = NodeRef::<Div>::new();

    // Click on image to make it fullscreen.
    let (show_img, set_show_img) = signal(false);
    let (current_img, set_current_img) = signal(None);
    let fullscreen_ref = NodeRef::<Div>::new();

    let close_fullscreen = move |_| {
        set_show_img.set(false);
    };

    let next = move || {
        if let Some(img) = current_img.get() {
            if let Some(files) = files.get() {
                let pos = files
                    .iter()
                    .position(|file_path| file_path.contains(&img))
                    .unwrap_or_default();

                let next_file = if let Some(file) = files.get(pos + 1) {
                    file.clone()
                } else {
                    files[0].clone()
                };
                set_current_img.set(Some(next_file));
            }
        }
    };
    let fullscreen_next = move |_| {
        next();
    };
    let back = move || {
        if let Some(img) = current_img.get() {
            if let Some(files) = files.get() {
                let pos = files
                    .iter()
                    .position(|file_path| file_path.contains(&img))
                    .unwrap_or_default();

                let next_file = if pos > 0 && files.get(pos - 1).is_some() {
                    files[pos - 1].clone()
                } else {
                    files[files.len() - 1].clone()
                };
                set_current_img.set(Some(next_file));
            }
        }
    };
    let fullscreen_back = move |_| {
        back();
    };

    let fullscreen_img = move || {
        if let Some(path) = current_img.get() {
            view! {
                <div style:display="flex" node_ref=fullscreen_ref class="fullscreen-img-buttons">
                    <img src=path class="fullscreen-img" />
                    <button on:click=close_fullscreen class="fullscreen-close">
                        <img
                            src="/img/icon/cross.svg"
                            alt="close"
                            style:width="1rem"
                            style:opacity="70%"
                        />
                    </button>
                    <button on:click=fullscreen_back class="fullscreen-nav-button back-button">
                        <img class="nav-button-img back-img" src="/img/icon/back.svg" alt="back" />
                    </button>
                    <button on:click=fullscreen_next class="fullscreen-nav-button next-button">
                        <img class="nav-button-img next-img" src="/img/icon/back.svg" alt="back" />
                    </button>
                </div>
            }
            .into_any()
        } else {
            view! { <p>"Sorry we couldn't load the image!"</p> }.into_any()
        }
    };

    // Close img on click outside
    Effect::new(move |_| {
        leptos_use::on_click_outside(fullscreen_ref, move |_| {
            set_show_img.set(false);
        })
    });

    view! {
        <Show when=move || { show_img.get() }>
            <div class="fullscreen-container">{fullscreen_img}</div>
        </Show>
        <div class="about-gallery">
            <div node_ref=_gallery_ref class="about-images-scroller snaps-inline">
                <Suspense fallback=move || {
                    view! { <p>"Loading"</p> }
                }>
                    {move || {
                        files
                            .get()
                            .map(|files| {
                                files
                                    .into_iter()
                                    .map(|file| {
                                        let file_c = file.clone();
                                        let click_image = move |ev: leptos::ev::MouseEvent| {
                                            ev.prevent_default();
                                            set_current_img.set(Some(file_c.clone()));
                                            set_show_img.set(true);
                                        };
                                        view! {
                                            <a on:click=click_image>
                                                <img src=file class="image-about" />
                                            </a>
                                        }
                                    })
                                    .collect_view()
                            })
                    }}

                </Suspense>
            </div>
        </div>
    }
}

// ###################################
// ->   SERVER
// ###################################

#[server(ReadImageFiles, "/api", "GetJson", "get_image_files")]
async fn read_image_files() -> Result<Vec<String>, ServerFnError> {
    use tokio::sync::OnceCell;
    let path = if cfg!(not(debug_assertions)) {
        "/app/site/img/about_pics"
    } else {
        "./public/img/about_pics"
    };
    static INIT: OnceCell<Result<Vec<String>, ServerFnError>> = OnceCell::const_new();
    INIT.get_or_init(|| async {
        let mut files = tokio::fs::read_dir(path).await?;
        let mut res = Vec::new();
        while let Some(file) = files.next_entry().await? {
            tracing::debug!(
                "path to file: {}",
                file.path()
                    .to_str()
                    .expect("Got an empty Filename in about.rs -> read_files()")
            );
            res.push(extract_name(file.path().to_str()));
        }
        Ok(res)
    })
    .await
    .clone()
}

/// Returns a string in the format: "/img/about_pics/{filename}",
/// if it can't split by '/' or input is `None `it just returns an empty string: ""
#[cfg(feature = "ssr")]
fn extract_name(path: Option<&str>) -> String {
    if let Some(path) = path {
        match path.split('/').last().map(|res| res.to_owned()) {
            Some(path) => format!("/img/about_pics/{}", path),
            None => "".to_owned(),
        }
    } else {
        "".to_owned()
    }
}
