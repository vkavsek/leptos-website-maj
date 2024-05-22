use leptos::{
    html::{Button, Div},
    *,
};
use leptos_meta::{Link, Title};
#[allow(unused)]
use std::path::{Path, PathBuf};
use wasm_bindgen::JsValue;
// use web_sys::{ScrollBehavior, ScrollToOptions};

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
        r#"Maj Kavšek (2000) is a Berlin-based trumpeter and composer born in Ljubljana, Slovenia. He started playing trumpet at the age of 8 and displayed a passion for music from a young age. His dedication and hard work eventually led him to pursue a formal education in music. At the age of 15, he started his studies at the Conservatorium for Music in Ljubljana, where he discovered the world of jazz music. It was during this time he was able to refine his trumpet skills and broaden his musical horizons."#,
        r#"After completing his studies at the Conservatorium in Ljubljana, Maj continued his musical education by pursuing studies abroad. He did so by enrolling at the Jazz Institute Berlin (UdK Berlin - Berlin University of the Arts). This move allowed him to continue his musical growth and exploration. Throughout his academic journey, he has had the privilege of working with esteemed professors and musicians, including notable names such as James Robert Rotondi, Nate Wooley, Ralph Alessi, Greg Cohen, Reggie Moore, and others."#,
        r#"Maj’s musical experience spans a wide range of ensemble settings. He has been a part of symphonic and wind orchestras, classical chamber music groups, jazz big bands, a variety of smaller jazz group settings as well as performing as a soloist. These diverse opportunities have contributed to his well-rounded musical skills and ability to excel in various musical contexts."#,
        r#"Maj has had the honor of performing as a featured soloist and orchestra musician with the "Big Band - Radio & Television Slovenia" on multiple occasions. In 2019, he won the 1st prize at the TEMSIG competition with his band - “The Mood Lab Quintet” in Slovenia. In 2023, he received an invitation to perform with the "Euroradio Jazz Orchestra" in Lithuania and “Fame’s Institute Orchestra” in North Macedonia. He has actively participated at various music festivals (Jazz Festival Ljubljana, Zvončki in Trobentice, Fête de la Musique Berlin, Skopje Jazz Festival, Vilnius Mama Jazz Festival and others) where he has taken on both the roles of a band-leader and a side-man, collaborating with esteemed musicians such as Chris Pitsiokos, Philipp Gropper, Bram De Looze, Felix Henkelhausen, Jan Bang and others."#,
    ];
    let text_content = text_content
        .map(|line| view! { <p>{line}</p> })
        .collect_view();

    let mut color = Color::Pink;
    let init_color = format!("color: {}", color.return_val());
    let init_bg_color = format!("background-color: {}", color.return_val());
    let init_next_bg_color = format!("background-color: {}", (color.other()).return_val());

    let text_ref = create_node_ref::<Div>();
    let button_ref = create_node_ref::<Button>();
    let curr_col_ref = create_node_ref::<Div>();
    let next_col_ref = create_node_ref::<Div>();

    let change_color = move |_| {
        let text_div = text_ref.get().expect("the DOM should be built by now");
        let button = button_ref.get().expect("the DOM should be built by now");
        let current_color = curr_col_ref.get().expect("the DOM should be built by now");
        let next_color = next_col_ref.get().expect("the DOM should be built by now");
        color.switch();
        let cd = color.return_val();
        let next_cd = (color.other()).return_val();
        let _ = text_div.style("color", cd);
        let _ = button.style("color", cd);
        let _ = current_color.style("background-color", cd);
        let _ = next_color.style("background-color", next_cd);
    };

    view! {
        <Link rel="icon" href="/img/izkaznica.svg" type_="image/svg"/>
        <Title text="About Me"/>
        <div class="components" id="about-components">
            <div class="title" id="about-title">
                <h1>"About Me"</h1>
            // <img class="title-img" id="about-img" src="/img/titles/about.svg"/>
            </div>
            <div class="contents" id="about-wrap">
                <ImagesAbout/>
                <div class="about-text" style=init_color.clone() node_ref=text_ref>
                    {text_content}
                    <button
                        class="change-color-button"
                        on:click=change_color
                        style=init_color
                        node_ref=button_ref
                    >
                        <div class="color-box" style=init_bg_color node_ref=curr_col_ref></div>
                        <img src="/img/icon/back.svg" class="button-next-arrow"/>
                        <div class="color-box" style=init_next_bg_color node_ref=next_col_ref></div>
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn ImagesAbout() -> impl IntoView {
    let files = create_resource(
        move || (),
        |_| async move {
            // TODO: Handle errors
            // NOTE: this only works in Docker
            read_files("/app/site/img/about_pics".to_string())
                // read_files("./public/img/about_pics".to_string())
                .await
                .unwrap()
        },
    );

    let _gallery_ref = create_node_ref::<Div>();

    // Click on image to make it fullscreen.
    let (show_img, set_show_img) = create_signal(false);
    let (current_img, set_current_img) = create_signal::<Option<String>>(None);
    let fullscreen_ref = create_node_ref::<Div>();

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
                    <img src=path class="fullscreen-img"/>
                    <button on:click=close_fullscreen class="fullscreen-close">
                        <img
                            src="/img/icon/cross.svg"
                            alt="close"
                            style:width="1rem"
                            style:opacity="70%"
                        />
                    </button>
                    <button on:click=fullscreen_back class="fullscreen-nav-button back-button">
                        <img class="nav-button-img back-img" src="/img/icon/back.svg" alt="back"/>
                    </button>
                    <button on:click=fullscreen_next class="fullscreen-nav-button next-button">
                        <img class="nav-button-img next-img" src="/img/icon/back.svg" alt="back"/>
                    </button>
                </div>
            }
            .into_view()
        } else {
            view! { <p>"Sorry we couldn't load the image!"</p> }.into_view()
        }
    };

    // Close img on click outside
    create_effect(move |_| {
        leptos_use::on_click_outside(fullscreen_ref, move |_| {
            set_show_img.set(false);
        })
    });

    view! {
        <Show when=move || { show_img.get() }>
            <div class="fullscreen-container">{fullscreen_img}</div>
        </Show>
        <div class="about-gallery">
            // <button on:click=move_back class="about-nav-button">
            // <img class="nav-button-img" src="/img/icon/back.svg" alt="back"/>
            // </button>
            <div node_ref=_gallery_ref class="about-images-scroller snaps-inline">
                // TODO: Add proper loading screen with Transition/Suspense
                <Suspense fallback=move || {
                    view! { <p>"Loading"</p> }
                }>
                    {move || {
                        files
                            .get()
                            .map(|files| {
                                files
                                    .iter()
                                    .map(|file| {
                                        let file_c = file.clone();
                                        let click_image = move |ev: leptos::ev::MouseEvent| {
                                            ev.prevent_default();
                                            set_current_img.set(Some(file_c.clone()));
                                            set_show_img.set(true);
                                        };
                                        view! {
                                            <a on:click=click_image>
                                                <img src=file class="image-about"/>
                                            </a>
                                        }
                                    })
                                    .collect_view()
                            })
                    }}

                </Suspense>
            </div>
        // <button on:click=move_next class="about-nav-button">
        // <img
        // class="nav-button-img"
        // src="/img/icon/back.svg"
        // style:transform="rotate(180deg)"
        // alt="back"
        // />
        // </button>
        </div>
    }
}

#[server(ReadFiles)]
async fn read_files(path: String) -> Result<Vec<String>, ServerFnError> {
    let mut files = tokio::fs::read_dir(&path).await?;
    let mut res = Vec::new();
    while let Some(file) = files.next_entry().await? {
        tracing::info!(
            "path to file: {}",
            file.path().to_str().expect("filename empty")
        );
        res.push(extract_name(file.path().to_str()));
    }
    Ok(res)
}
/// Always returns a string, if it can't split by '/' or input is None it just returns an empty
/// string: ""
#[allow(dead_code)]
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
