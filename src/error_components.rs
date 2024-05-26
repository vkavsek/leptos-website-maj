use http::status::StatusCode;
use leptos::*;
use leptos_router::A;

#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;

use crate::MajServerError;

// A basic function to display errors served by the error boundaries.
// Feel free to do more complicated things here than just displaying the error.
#[component]
pub fn ErrorTemplate(
    #[prop(optional)] outside_errors: Option<Errors>,
    #[prop(optional)] errors: Option<RwSignal<Errors>>,
) -> impl IntoView {
    let errors = match outside_errors {
        Some(e) => create_rw_signal(e),
        None => match errors {
            Some(e) => e,
            None => panic!("No Errors found and we expected errors!"),
        },
    };
    // Get Errors from Signal
    let errors = errors.get_untracked();

    // Downcast lets us take a type that implements `std::error::Error`
    let errors: Vec<_> = errors
        .into_iter()
        .filter_map(|(_k, e)| e.downcast_ref::<MajServerError>().cloned())
        .collect();

    // Only the response code for the first error is actually sent from the server
    // this may be customized by the specific application
    #[cfg(feature = "ssr")]
    {
        let response = use_context::<ResponseOptions>();
        if let Some(response) = response {
            response.set_status(
                errors
                    .first()
                    .map(|err| err.status_code())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            );
        }
    }

    view! {
        <div class="error-wrap">
            <div>
                <h1 class="error-title">{if errors.len() > 1 { "Errors" } else { "Error" }}</h1>
                <For
                    // a function that returns the items we're iterating over; a signal is fine
                    each=move || { errors.clone().into_iter().enumerate() }
                    // a unique key for each item as a reference
                    key=|(index, _error)| *index
                    // renders each item to a view
                    children=move |error| {
                        let error_string = error.1.to_string();
                        let error_code = error.1.status_code();
                        if error_code == StatusCode::NOT_FOUND {
                            view! {
                                <h2 class="error-subtitle">{error_code.to_string()}</h2>
                                <p>"The site you are looking for doesn't exist!"</p>
                                <A href="/">"Go back to the home page."</A>
                            }
                        } else {
                            view! {
                                <h2 class="error-subtitle">{error_code.to_string()}</h2>
                                <p>"Error: " {error_string}</p>
                            }
                        }
                    }
                />

            </div>
        </div>
    }
}
