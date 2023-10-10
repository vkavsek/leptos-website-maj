use chrono::prelude::*;
use leptos::*;
use leptos_meta::{Link, Title};
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct Shows {
    shows: Vec<Show>,
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Show {
    date: Option<String>,
    name: Option<String>,
    club: Option<String>,
    location: Option<String>,
}

#[component]
pub fn Concerts() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg"/>
        <Title text="Shows"/>
        <div class="components">
            <div class="title" id="shows-title">
                <h1>"Shows"</h1>
                // <img class="title-img" id="shows-img" src="/img/titles/shows.svg"/>
            </div>
            <div class="contents" id="shows-contents">
                <nav class="shows-nav">
                    <A href="past">"past"</A>
                    <A href="future">"future"</A>
                </nav>
                <div id="shows-wrap">
                    <Outlet/>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn ConcertsFallback() -> impl IntoView {
    view! {
        <p class="shows-no-shows">"Select past or future events by clicking on the link above!"</p>
    }
}

#[component]
pub fn Past() -> impl IntoView {
    let shows: Shows = serde_json::from_str(SHOWS_JSON).expect("Valid Json 100%");
    let pivot_date = Utc::now();
    let before: Vec<_> = shows
        .shows
        .iter()
        .filter(|show| {
            let date = NaiveDate::parse_from_str(
                show.date.as_ref().expect("every entry has a date"),
                "%d.%m.%Y",
            )
            .expect("All should be valid")
            .and_time(NaiveTime::from_hms_milli_opt(0, 0, 0, 0).unwrap())
            .and_utc();
            date.lt(&pivot_date)
        })
        .map(|show| {
            view! {
                <li class="show-container">
                    <p>{show.date.as_ref().unwrap()}</p>
                    <p>{show.name.as_ref().unwrap()}</p>
                    <p>{show.club.as_ref().unwrap()}</p>
                    <p>{show.location.as_ref().unwrap()}</p>
                </li>
            }
        })
        .collect();

    view! {
        <ul class="shows-list">
            <li class="show-container" id="show-container-id">
                <p>"Date:"</p>
                <p>"Event:"</p>
                <p>"Club:"</p>
                <p>"Location:"</p>
            </li>
            {before}
        </ul>
    }
}

#[component]
pub fn Future() -> impl IntoView {
    let shows: Shows = serde_json::from_str(SHOWS_JSON).expect("Valid Json 100%");
    let pivot_date = Utc::now();
    //NaiveDate::parse_from_str("23.01.2023", "%d.%m.%Y").expect("ok");
    //let t = NaiveTime::from_hms_milli_opt(0, 0, 0, 0).expect("ok");
    //let pivot_date = pivot_date.and_time(t).and_utc();

    let after: Vec<_> = shows
        .shows
        .iter()
        .filter(|show| {
            let date = NaiveDate::parse_from_str(
                show.date.as_ref().expect("every entry has a date"),
                "%d.%m.%Y",
            )
            .expect("All should be valid")
            .and_time(NaiveTime::from_hms_milli_opt(0, 0, 0, 0).unwrap())
            .and_utc();
            date.ge(&pivot_date)
        })
        .collect();

    let after_view = if !after.is_empty() {
        Some(
            after
                .iter()
                .map(|show| {
                    view! {
                        <li class="show-container">
                            <p>{show.date.as_ref().unwrap()}</p>
                            <p>{show.name.as_ref().unwrap()}</p>
                            <p>{show.club.as_ref().unwrap()}</p>
                            <p>{show.location.as_ref().unwrap()}</p>
                        </li>
                    }
                })
                .collect_view(),
        )
    } else {
        None
    };

    let check = after_view.is_some();

    view! {
        <Show
            when=move || check
            fallback=|| {
                view! {  <p class="shows-no-shows">"There are currently no events planned for the future."</p> }
            }
        >
            <ul class="shows-list">{after_view.clone()}</ul>
        </Show>
    }
}

// TODO: Make this dynamic! SQL!
#[allow(unused)]
const SHOWS_JSON: &str = r#"
{"shows": [{
  "date": "23.08.2023",
  "name": "Good old friends",
  "club": "Zorica",
  "location": "Ljubljana, Slovenia"
},{
  "date": "10.08.2023",
  "name": "Local Warming",
  "club": "Boca Wine Bar",
  "location": "Pula, Croatia"
},{
  "date": "08.08.2023",
  "name": "Local Warming",
  "club": "Prulček",
  "location": "Ljubljana, Slovenia"
},{
  "date": "23.06.2023",
  "name": "Collector",
  "club": "Ramiro Zayas - Donau115",
  "location": "Berlin, Germany"
},{
  "date": "17.06.2023",
  "name": "Prismala album release show",
  "club": "Renaissance Theatre Berlin",
  "location": "Berlin, Germany"
},{
  "date": "09.06.2023",
  "name": "Amoeba",
  "club": "Make-up Berlin",
  "location": "Berlin, Germany"
},{
  "date": "27.05.2023",
  "name": "Euroradio Jazz Orchestra",
  "club": "Mama Jazz Festival",
  "location": "Vilnius, Lithuania"
},{
  "date": "26.05.2023",
  "name": "Euroradio Jazz Orchestra",
  "club": "Kaunas Philharmonic",
  "location": "Kaunas, Lithuania"
},{
  "date": "08.05.2023",
  "name": "Oto's Treatment",
  "club": "Jazz Club ZigZag Berlin",
  "location": "Berlin, Germany"
},{
  "date": "28.03.2023",
  "name": "Gal Golob Trio",
  "club": "Zorica",
  "location": "Ljubljana, Slovenia"
},{
  "date": "22.03.2023",
  "name": "Pilgrimage of the lost troubadours",
  "club": "Hupa Brajdič",
  "location": "Ljubljana, Slovenia"
},{
  "date": "17.03.2023",
  "name": "Maj Kavšek Quartet",
  "club": "Jazz Club Ljubljana Castle",
  "location": "Ljubljana, Slovenia"
},{
  "date": "13.02.2023",
  "name": "Luka Kastelic Quintet",
  "club": "ZigZag Jazz Club",
  "location": "Berlin, Germany"
},{
  "date": "01.02.2023",
  "name": "Luka Kastelic Quintet",
  "club": "Orangerie Neukölln",
  "location": "Berlin, Germany"
},{
  "date": "28.12.2022",
  "name": "Christmas and Friends",
  "club": "Prulček Bar",
  "location": "Ljubljana, Slovenia"
},{
  "date": "23.12.2022",
  "name": "Big Band RTV Slovenia",
  "club": "SiTi Teater",
  "location": "Ljubljana, Slovenia"
},{
  "date": "06.12.2022",
  "name": "Jam Opener",
  "club": "Bierhaus Urban",
  "location": "Berlin, Germany"
},{
  "date": "11.11.2022",
  "name": "Chet Baker Tribute",
  "club": "Orville's",
  "location": "Berlin, Germany"
},{
  "date": "13.10.2022",
  "name": "Jam Opener",
  "club": "La Minga",
  "location": "Berlin, Germany"
},{
  "date": "24.09.2022",
  "name": "Ramiro Olaciregui Group",
  "club": "Jazz Club B-flat",
  "location": "Berlin, Germany"
},{
  "date": "22.09.2022",
  "name": "Jam Opener",
  "club": "La Minga",
  "location": "Berlin, Germany"
},{
  "date": "06.09.2022",
  "name": "Local Warming",
  "club": "Prulček Bar",
  "location": "Ljubljana, Slovenia"
},{
  "date": "30.08.2022",
  "name": "Local Warming",
  "club": "Zorica Bar",
  "location": "Ljubljana, Slovenia"
},{
  "date": "28.08.2022",
  "name": "Local Warming",
  "club": "MC Kavarna",
  "location": "Žalec, Slovenia"
},{
  "date": "09.08.2022",
  "name": "Local Warming",
  "club": "Festival “Poletje na Travniku”",
  "location": "Lucija, Slovenia"
},{
  "date": "11.01.2022",
  "name": "Re.Chat Quintet",
  "club": "Jazz Club Schlot",
  "location": "Berlin, Germany"
}]}
"#;
