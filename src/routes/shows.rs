use chrono::prelude::*;
use leptos::prelude::*;
use leptos_meta::{Link, Title};
use leptos_router::components::{Outlet, A};
use serde::{Deserialize, Serialize};

use crate::{routes::error::ErrorTemplate, MajServerError};

// ###################################
// ->   ROUTES
// ###################################

#[component]
pub fn Shows() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg" />
        <Title text="Shows" />
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
                    <Outlet />
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn ShowsFallback() -> impl IntoView {
    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg" />
        <p class="shows-no-shows">"Select past or future events by clicking on the link above!"</p>
    }
}

#[component]
pub fn PastShows() -> impl IntoView {
    let selector = EventSelector::Past;
    view! { <RenderShows selector /> }
}

#[component]
pub fn FutureShows() -> impl IntoView {
    let selector = EventSelector::Future;
    view! { <RenderShows selector /> }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventsSorted {
    date_desc: Vec<Event>,
    date_asc: Vec<Event>,
}
impl EventsSorted {
    #[inline]
    pub fn get_desc(&self) -> &[Event] {
        &self.date_desc
    }
    #[inline]
    pub fn get_asc(&self) -> &[Event] {
        &self.date_asc
    }
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Deserialize)]
pub struct EventsDeser(Vec<Event>);

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Default, Serialize, Deserialize)]
pub struct Event {
    pub(crate) date: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) club: Option<String>,
    pub(crate) location: Option<String>,
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

#[cfg(feature = "ssr")]
pub(crate) fn parse_shows(json: &str) -> Result<EventsSorted, MajServerError> {
    let shows: EventsDeser = serde_json::from_str(json)?;
    sort_shows(shows)
}

#[cfg(feature = "ssr")]
fn sort_shows(mut shows: EventsDeser) -> Result<EventsSorted, MajServerError> {
    use std::cmp::Reverse;
    for (index, show) in shows.0.iter().enumerate() {
        if show.get_date().is_none() {
            return Err(MajServerError::SerdeJson(format!(
                "Show {} must have a valid date in day.month.year format.",
                index + 1
            )));
        }
    }
    shows.0.sort_by_key(|show| Reverse(show.get_date()));

    let date_desc = shows.0.clone();
    let mut date_asc = shows.0;
    date_asc.reverse();

    Ok(EventsSorted {
        date_desc,
        date_asc,
    })
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
    let shows_resource = Resource::new_blocking(|| (), |_| async move { get_shows().await });
    let pivot_date = Utc::now()
        .with_time(
            NaiveTime::from_hms_milli_opt(0, 0, 0, 0)
                .expect("Unexpected error creating 0000 NaiveTime"),
        )
        .single()
        .unwrap_or(Utc::now());

    view! {
        <Link rel="icon" href="/img/nota.svg" type_="image/svg" />
        <Suspense fallback=move || {
            view! { <p class="shows-no-shows">"  "</p> }
        }>
            <ErrorBoundary fallback=move |errors| {
                view! { <ErrorTemplate errors /> }
            }>
                {move || {
                    Suspend::new(async move {
                        shows_resource
                            .await
                            .map(|shows| {
                                {
                                    let shows_col = match selector {
                                        EventSelector::Past => {
                                            shows
                                                .get_desc()
                                                .iter()
                                                .filter(|show| {
                                                    let date = show.get_date().expect("All events need dates!");
                                                    date.lt(&pivot_date)
                                                })
                                                .collect::<Vec<_>>()
                                        }
                                        EventSelector::Future => {
                                            shows
                                                .get_asc()
                                                .iter()
                                                .filter(|show| {
                                                    let date = show.get_date().expect("All events need dates!");
                                                    date.ge(&pivot_date)
                                                })
                                                .collect::<Vec<_>>()
                                        }
                                    };
                                    if shows_col.is_empty() {

                                        view! {
                                            <p class="shows-no-shows">
                                                "There are currently no events to display here. Come back later."
                                            </p>
                                        }
                                            .into_any()
                                    } else {
                                        let view = shows_col
                                            .iter()
                                            .map(|show| {
                                                view! {
                                                    <li class="show-container">
                                                        <p>{show.date.clone()}</p>
                                                        <p>{show.name.clone()}</p>
                                                        <p>{show.club.clone()}</p>
                                                        <p>{show.location.clone()}</p>
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
                                            .into_any()
                                    }
                                }
                            })
                    })
                }}

            </ErrorBoundary>
        </Suspense>
    }
}

/// Read live records, preserving the existing public date format and chronological ordering.
#[server(GetShows, "/api", "GetJson", "get_shows")]
async fn get_shows() -> Result<EventsSorted, MajServerError> {
    let database =
        use_context::<crate::database::DatabaseManager>().ok_or(MajServerError::Internal)?;
    load_shows(&database).await
}

#[cfg(feature = "ssr")]
async fn load_shows(
    database: &crate::database::DatabaseManager,
) -> Result<EventsSorted, MajServerError> {
    let rows = database
        .shows()
        .await
        .map_err(|_| MajServerError::Internal)?;
    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let date = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
            .map_err(|_| MajServerError::Internal)?;
        events.push(Event {
            date: Some(date.format("%d.%m.%Y").to_string()),
            name: row.name,
            club: row.club,
            location: row.location,
        });
    }
    sort_shows(EventsDeser(events))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn show_loader_reads_live_data_and_sorts_real_dates() {
        let (_root, database) = crate::test_support::database().await;
        let id = database
            .add_show("2027-01-01", None, Some("Venue"), None)
            .await
            .unwrap();
        database
            .add_show("2024-02-29", Some("Leap day"), None, None)
            .await
            .unwrap();
        let shows = load_shows(&database).await.unwrap();
        assert_eq!(shows.get_asc()[0].date.as_deref(), Some("29.02.2024"));
        assert_eq!(shows.get_desc()[0].club.as_deref(), Some("Venue"));
        database.remove_show(id).await.unwrap();
        assert_eq!(load_shows(&database).await.unwrap().get_asc().len(), 1);
        database.close().await;
        assert!(load_shows(&database).await.is_err());
    }

    #[test]
    fn invalid_dates_reject_the_entire_show_file() {
        for invalid in [
            r#"{}"#,
            r#"{"date":null}"#,
            r#"{"date":""}"#,
            r#"{"date":"31.02.2026"}"#,
            r#"{"date":"29.02.2025"}"#,
            r#"{"date":"00.12.2026"}"#,
            r#"{"date":"01.13.2026"}"#,
            r#"{"date":"2026-11-10"}"#,
        ] {
            let json = format!(r#"[{{"date":"10.11.2026"}},{invalid}]"#);
            let error = parse_shows(&json).unwrap_err();
            assert!(
                matches!(error, MajServerError::SerdeJson(ref message) if message.contains("Show 2")),
                "{invalid}: {error}"
            );
        }
    }

    #[test]
    fn valid_dates_include_leap_days_and_sort_chronologically() {
        let shows =
            parse_shows(r#"[{"date":"01.01.2027"},{"date":"29.02.2024"},{"date":"31.12.2025"}]"#)
                .unwrap();
        let ascending: Vec<_> = shows
            .get_asc()
            .iter()
            .map(|show| show.date.as_deref())
            .collect();
        assert_eq!(
            ascending,
            [Some("29.02.2024"), Some("31.12.2025"), Some("01.01.2027")]
        );
        assert!(shows.get_asc().iter().all(|show| show.get_date().is_some()));
        assert!(shows.get_desc().iter().eq(shows.get_asc().iter().rev()));
    }

    #[test]
    fn current_show_file_and_empty_list_are_valid() {
        let json = include_str!("../../public/data_json/shows.json");
        let expected: EventsDeser = serde_json::from_str(json).unwrap();
        let shows = parse_shows(json).unwrap();
        assert_eq!(shows.get_asc().len(), expected.0.len());
        assert!(shows.get_asc().iter().all(|show| show.get_date().is_some()));
        assert!(shows.get_desc().iter().eq(shows.get_asc().iter().rev()));

        let empty = parse_shows("[]").unwrap();
        assert!(empty.get_asc().is_empty());
        assert!(empty.get_desc().is_empty());
    }
}
