#[allow(unused)]
use anyhow::Result;
use leptos::{
    html::{Button, Div, Img},
    *,
};
use leptos_meta::{Link, Title};
#[allow(unused)]
use std::path::{Path, PathBuf};

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
        text_div.style("color", cd);
        button.style("color", cd);
        current_color.style("background-color", cd);
        next_color.style("background-color", next_cd);
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
                <div class="about-images-scroller snaps-inline">
                    <ImagesAbout/>
                </div>
                <div class="about-button-text">
                    <button on:click=change_color style=init_color.clone() node_ref=button_ref>
                        <div class="color-box" style=init_bg_color node_ref=curr_col_ref></div>
                        <img src="/img/icon/back.svg" class="button-next-arrow"/>
                        <div class="color-box" style=init_next_bg_color node_ref=next_col_ref></div>
                    </button>
                    <div class="about-text" style=init_color node_ref=text_ref>
                        {text_content}
                    </div>
                </div>
            </div>
        </div>
    }
}

// TODO: implement leptos_image and change the architecture
#[component]
pub fn ImagesAbout() -> impl IntoView {
    // Set FIRST IMAGE TO SHOW
    // let img_ref = create_node_ref::<Img>();
    // let (counter, set_counter) = create_signal(0);

    let files = create_resource(
        move || (),
        |_| async move {
            // TODO: Handle errors
            // NOTE: this only works in Docker
            // read_files("/app/site/img/about_pics".to_string())
            read_files("./public/img/about_pics".to_string())
                .await
                .unwrap()
        },
    );

    // let first_src = files.get().map_or_else(
    //     || "/img/about_pics/shared28.jpg".to_string(),
    //     |files| {
    //         format!(
    //             "/img/about_pics/{}",
    //             files.get(0).expect("no pictures in ABOUTME dir!")
    //         )
    //     },
    // );

    // let click_next = move |_| {
    //     let img = img_ref
    //         .get()
    //         .expect("the DOM is built by the time we click the button");
    //     if let Some(files) = files.get() {
    //         if let Some(ref image_name) = files.get(counter.get() + 1) {
    //             set_counter.update(|c| *c += 1);
    //             let img_src_fmt = format!("/img/about_pics/{}", image_name);
    //             img.set_src(&img_src_fmt);
    //         } else {
    //             let first_src = format!(
    //                 "/img/about_pics/{}",
    //                 files.get(0).cloned().unwrap_or_default()
    //             );
    //             set_counter.set(0);
    //             img.set_src(&first_src);
    //         }
    //     }
    // };
    //
    // let click_back = move |_| {
    //     let img = img_ref
    //         .get()
    //         .expect("the DOM is built by the time we click the button");
    //     if counter.get() > 0 {
    //         if let Some(files) = files.get() {
    //             if let Some(ref image_name) = files.get(counter.get() - 1) {
    //                 set_counter.update(|c| *c -= 1);
    //                 let img_src_fmt = format!("/img/about_pics/{}", image_name);
    //                 img.set_src(&img_src_fmt);
    //             }
    //         }
    //     } else {
    //         set_counter.set(files.get().map(|f| f.len()).unwrap_or(1) - 1);
    //         let last_src = format!(
    //             "/img/about_pics/{}",
    //             files
    //                 .get()
    //                 .and_then(|files| files.last().cloned())
    //                 .unwrap_or_default()
    //         );
    //         img.set_src(&last_src);
    //     }
    // };
    //
    // use leptos_image::*;
    view! {
        <Suspense fallback=move || {}>
            {move || {
                files
                    .get()
                    .map(|files| {
                        files
                            .iter()
                            .map(|file| {
                                let source = format!("/img/about_pics/{}", file);
                                view! { <img src=source class="image-about"/> }
                            })
                            .collect_view()
                    })
            }}

        </Suspense>
    }
    // <div class="about-img-but-container">
    //     <img class="about-img" src=first_src node_ref=img_ref/>
    //     <button class="about-back-img" on:click=click_back>
    //         <img src="/img/icon/back.svg" class="about-navigation-img about-back"/>
    //     </button>
    //     <button class="about-next-img" on:click=click_next>
    //         <img src="/img/icon/back.svg" class="about-navigation-img about-next"/>
    //     </button>
    // </div>
}

#[server(ReadFiles)]
async fn read_files(path: String) -> Result<Vec<String>, ServerFnError> {
    let mut files = tokio::fs::read_dir(&path).await?;
    let mut res = Vec::new();
    while let Some(file) = files.next_entry().await? {
        log::info!(
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
            Some(path) => path,
            None => "".to_owned(),
        }
    } else {
        "".to_owned()
    }
}
