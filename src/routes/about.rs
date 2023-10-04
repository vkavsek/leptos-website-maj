use leptos::{html::Img, *};
use leptos_meta::{Link, Title};

#[component]
pub fn About() -> impl IntoView {
    let cntnt = [
        r"Maj Kavšek is a trumpet player born on April 12, 2000, in Ljubljana, Slovenia. He started his journey with the trumpet at the age of 8 and displayed a passion for music from a young age. His dedication and hard work eventually led him to pursue formal education in music. At the age of 15, his musical pursuits led him to study at the Conservatorium for Music in Ljubljana, Slovenia, where he delved into the world of jazz music. It was during this time he was allowed to refine his trumpet skills and broaden his musical horizons.",
        r"Following the completion of his studies at the Conservatorium in Ljubljana he decided to further his musical education by pursuing studies abroad. He did so by enrolling at Jazz Institut Berlin (UdK Berlin - University of Arts Berlin). This move allowed him to continue his musical growth and exploration. Throughout his academic journey, he has had the privilege of working with esteemed professors and musicians, including notable names like James Robert Rotondi, Nate Wooley, Ralph Alessi, Greg Cohen, Reggie Moore, and others.",
        r"Maj’s musical experiences span a wide range of ensemble settings, showcasing his versatility and adaptability. He has been a part of symphonic and wind orchestras, classical chamber music groups, jazz big bands, a variety of small group settings and even solo performances. This diverse exposure has contributed to his well-rounded musical skills and ability to excel in various musical contexts.",
        r"As a soloist, Maj has had the honour of performing with the 'Big Band Radio & Television Slovenia' on multiple occasions.  In 2019, he won the 1st prize at the TEMSIG musical competition with his quintet. In 2023, he received an invitation to perform with the 'Euroradio Jazz Orchestra' in Lithuania, further solidifying his status as a prominent musician in the international jazz scene.",
        r"Maj's influence in the music world extends beyond the academic realm. He has been an active participant in various jazz festivals, where he has taken on roles as both a band-leader and a side-man, collaborating with esteemed musicians such as Chris Pitsiokos, Philipp Gropper, Dré Hocevar, Bram De Looze, Felix Henkelhausen and others.",
    ];
    let cntnt = cntnt.map(|line| view! {  <p>{line}</p> }).collect_view();

    view! {
        <Link rel="icon" href="/img/izkaznica.svg" type_="image/svg"/>
        <Title text="About Me"/>
        <div class="components">
            <div class="title" id="about-title">
                //<h1>"About Me"</h1>
                <img class="title-img" id="about-img" src="/img/titles/about.svg"/>
            </div>
            <div class="contents" id="about-wrap">
                <div class="about-text">{cntnt}</div>
                <ImagesAbout/>
            </div>
        </div>
    }
}

// TODO: will need to be modified for SSR
#[component]
pub fn ImagesAbout() -> impl IntoView {
    static IMAGE_NAMES: [&str; 15] = [
        "shared8.jpg",
        "shared1.jpg",
        "shared6.jpg",
        "shared11.jpg",
        "shared13.jpg",
        "shared24.jpg",
        "shared28.jpg",
        "trumpet01.jpg",
        "trumpet2.jpg",
        "IMG_0131.jpg",
        "IMG_0793.jpg",
        "IMG_0794.jpg",
        "IMG_0813.jpg",
        "band.jpg",
        "band2.jpg",
    ];

    let img_ref = create_node_ref::<Img>();
    let first_src = format!("/img/about_pics/{}", IMAGE_NAMES[0]);
    let (counter, set_counter) = create_signal(0);

    let click_next = move |_| {
        let img = img_ref().expect("the DOM is built by the time we click the button");
        if let Some(&image_name) = IMAGE_NAMES.get(counter() + 1) {
            set_counter.update(|c| *c += 1);
            let img_src_fmt = format!("/img/about_pics/{}", image_name);
            img.set_src(&img_src_fmt);
        } else {
            set_counter(0);
            let first_src = format!("/img/about_pics/{}", IMAGE_NAMES[0]);
            img.set_src(&first_src);
        }
    };

    let click_back = move |_| {
        let img = img_ref().expect("the DOM is built by the time we click the button");
        if counter() > 0 {
            if let Some(&image_name) = IMAGE_NAMES.get(counter() - 1) {
                set_counter.update(|c| *c -= 1);
                let img_src_fmt = format!("/img/about_pics/{}", image_name);
                img.set_src(&img_src_fmt);
            }
        } else {
            set_counter(IMAGE_NAMES.len() - 1);
            let last_src = format!("/img/about_pics/{}", IMAGE_NAMES.last().unwrap());
            img.set_src(&last_src);
        }
    };

    view! {
        <div class="about-pics">
            <div class="about-img-but-container">
                <img class="about-img" src=first_src node_ref=img_ref/>
                <button class="about-back-img" on:click=click_back>
                    <i class="fa-solid fa-arrow-left"></i>
                </button>
                <button class="about-next-img" on:click=click_next>
                    <i class="fa-solid fa-arrow-right"></i>
                </button>
            </div>
        </div>
    }
}
