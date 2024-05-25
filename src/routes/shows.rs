use std::ops::{Deref, DerefMut};

use chrono::prelude::*;
use leptos::*;
use leptos_meta::{Link, Title};
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::error_template::ErrorTemplate;

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
pub struct Events(Vec<Event>);

impl DerefMut for Events {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Deref for Events {
    type Target = Vec<Event>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Default, Serialize, Deserialize)]
pub struct Event {
    date: Option<String>,
    name: Option<String>,
    club: Option<String>,
    location: Option<String>,
}
impl Event {
    pub fn get_date(&self) -> Option<DateTime<Utc>> {
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

// Allow unused otherwise we get warnings because of the macros.
#[allow(unused)]
/// Fetches all events from the server, sorts them, and returns the desired list of events,
/// Past or Future
#[component]
fn RenderShows(selector: EventSelector) -> impl IntoView {
    let shows_resource = create_resource(|| (), |_| async move { get_shows().await });
    let pivot_date = Utc::now();

    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg"/>
        <Suspense fallback=move || {
            view! { <p class="shows-no-shows">"  "</p> }
        }>
            {move || {
                shows_resource
                    .get()
                    .map(|shows| {
                        view! {
                            <ErrorBoundary fallback=move |errors| {
                                view! { <ErrorTemplate errors/> }
                            }>
                                {shows
                                    .map(|shows| {
                                        let shows_col = shows
                                            .iter()
                                            .filter(|show| {
                                                let date = show.get_date().expect("All events need dates!");
                                                match selector {
                                                    EventSelector::Past => date.lt(&pivot_date),
                                                    EventSelector::Future => date.ge(&pivot_date),
                                                }
                                            })
                                            .collect::<Vec<_>>();
                                        if shows_col.is_empty() {
                                            view! {
                                                <p class="shows-no-shows">
                                                    "There are currently no events to display here. Come back later."
                                                </p>
                                            }
                                                .into_view()
                                        } else {
                                            let view = shows_col
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
                                                .collect_view();
                                            view! {
                                                <ul class="shows-list">
                                                    <li class="show-container" id="show-container-id">
                                                        <p>"Date:"</p>
                                                        <p>"Event:"</p>
                                                        <p>"Venue:"</p>
                                                        <p>"Location:"</p>
                                                    </li>
                                                    {view}
                                                </ul>
                                            }
                                                .into_view()
                                        }
                                    })}

                            </ErrorBoundary>
                        }
                    })
            }}

        </Suspense>
    }
}

// ###################################
// ->   SERVER
// ###################################

/// Uses `OnceCell` to deserialize and sort `Shows` from JSON.
/// Can return an Error if there was a problem reading the file or parsing the JSON.
/// Only reads the files, deserializes and sorts them on the initial call.
/// Further calls just return a clone of `Shows`.
#[server(GetShows, "/api", "GetJson", "get_shows")]
async fn get_shows() -> Result<Events, ServerFnError> {
    let shows = get_shows_util()
        .await
        .as_ref()
        .map_err(ServerFnError::WrappedServerError)?;

    Ok(shows.clone())
}

#[cfg(feature = "ssr")]
async fn get_shows_util() -> &'static Result<Events, crate::MajServerError> {
    use std::cmp::Reverse;
    use tokio::sync::OnceCell;
    use tracing::info;

    static SHOWS_INIT: OnceCell<Result<Events, crate::MajServerError>> = OnceCell::const_new();

    tracing::debug!("Retrieving SHOWS");
    SHOWS_INIT
        .get_or_init(|| async {
            let path = if cfg!(not(debug_assertions)) {
                "/app/site/data_json/shows.json"
            } else {
                "./public/data_json/shows.json"
            };

            info!("Initializing SHOWS");
            let show_json = tokio::fs::read_to_string(path).await?;
            let mut shows: Events = serde_json::from_str(&show_json)?;
            shows.sort_by_key(|show_a| Reverse(show_a.get_date()));
            Ok(shows)
        })
        .await
}
