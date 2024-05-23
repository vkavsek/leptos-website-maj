use chrono::prelude::*;
use leptos::*;
use leptos_meta::{Link, Title};
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::error_template::MajServerError;

// ###################################
// ->   ROUTES
// ###################################

#[component]
pub fn Shows() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg"/>
        <Title text="Shows"/>
        <div class="components" id="shows-components">
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
pub fn ShowsFallback() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg"/>
        <p class="shows-no-shows">"Select past or future events by clicking on the link above!"</p>
    }
}

#[component]
pub fn PastShows() -> impl IntoView {
    let selector = EventSelector::Past;
    view! { <RenderShows selector/> }
}

#[component]
pub fn FutureShows() -> impl IntoView {
    let selector = EventSelector::Future;
    view! { <RenderShows selector/> }
}

// ###################################
// ->   UTILS
// ###################################

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Events {
    shows: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Default, Serialize, Deserialize)]
pub struct Event {
    date: Option<String>,
    name: Option<String>,
    club: Option<String>,
    location: Option<String>,
}
impl Event {
    fn get_date(&self) -> Option<DateTime<Utc>> {
        // TODO: Do I like this implementation ?
        self.date.as_ref().map(|date| {
            NaiveDate::parse_from_str(date, "%d.%m.%Y")
                .map(|date| {
                    date.and_time(
                        NaiveTime::from_hms_milli_opt(0, 0, 0, 0)
                            .expect("Unexpected error creating 0000 NaiveTime"),
                    )
                    .and_utc()
                })
                .ok()
        })?
    }
}

#[derive(Clone, Copy, Debug)]
enum EventSelector {
    Past,
    Future,
}

/// Fetches all events from the server, sorts them, and returns the desired list of events,
/// Past or Future
#[component]
fn RenderShows(selector: EventSelector) -> impl IntoView {
    let shows = create_resource(|| (), |_| async move { get_shows().await });
    let pivot_date = Utc::now();

    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg"/>
        <Transition fallback=move || {
            view! { <p class="shows-no-shows">"Loading future shows!"</p> }
        }>
            <ul class="shows-list">
                <li class="show-container" id="show-container-id">
                    <p>"Date:"</p>
                    <p>"Event:"</p>
                    <p>"Venue:"</p>
                    <p>"Location:"</p>
                </li>
                {move || {
                    shows
                        .get()
                        .map(|shows| {
                            let shows_col = shows
                                .expect("GetShows should be infallible")
                                .shows
                                .into_iter()
                                .filter(|show| {
                                    let date = show.get_date().expect("All events need dates!");
                                    match selector {
                                        EventSelector::Past => date.lt(&pivot_date),
                                        EventSelector::Future => date.ge(&pivot_date),
                                    }
                                })
                                .collect::<Vec<_>>();
                            shows_col
                                .iter()
                                .map(|show| {
                                    view! {
                                        <li class="show-container">
                                            <p>{show.date.as_ref()}</p>
                                            <p>{show.name.as_ref()}</p>
                                            <p>{show.club.as_ref()}</p>
                                            <p>{show.location.as_ref()}</p>
                                        </li>
                                    }
                                })
                                .collect_view()
                        })
                }}

            </ul>

        </Transition>
    }
}

// ###################################
// ->   SERVER
// ###################################

/// Uses `OnceCell` to deserialize and sort `Shows` from JSON.
/// Can return an Error if there was a problem reading the file or parsing the JSON.
/// Only reads the files, deserializes and sorts them on the initial call.
/// Further calls just return a clone of `Shows`.
#[server(GetShows)]
async fn get_shows() -> Result<Events, ServerFnError> {
    let shows = get_shows_util()
        .await
        .as_ref()
        .map_err(ServerFnError::WrappedServerError)?;

    Ok(shows.clone())
}

#[cfg(feature = "ssr")]
async fn get_shows_util() -> &'static Result<Events, MajServerError> {
    use std::cmp::Reverse;
    use tokio::sync::OnceCell;

    let path = if cfg!(not(debug_assertions)) {
        "/app/shows/shows.json"
    } else {
        "./public/shows/shows.json"
    };

    static SHOWS_INIT: OnceCell<Result<Events, MajServerError>> = OnceCell::const_new();

    SHOWS_INIT
        .get_or_init(|| async {
            let show_json = tokio::fs::read_to_string(path).await?;
            let mut shows: Events = serde_json::from_str(&show_json)?;
            shows.shows.sort_by_key(|show_a| Reverse(show_a.get_date()));
            Ok(shows)
        })
        .await
}
