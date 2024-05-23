use http::status::StatusCode;
use leptos::*;
use leptos_router::A;
use serde::Serialize;
use tracing::error;

#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;

pub type Result<T> = core::result::Result<T, MajServerError>;

#[derive(Serialize, Clone, Debug)]
pub enum MajServerError {
    NotFound,
    Io(String),
    SerdeJson(String),
}

impl MajServerError {
    pub fn status_code(&self) -> StatusCode {
        #[allow(unreachable_patterns)]
        match self {
            MajServerError::NotFound => StatusCode::NOT_FOUND,
            Self::Io(_) => StatusCode::NO_CONTENT,
            Self::SerdeJson(_) => StatusCode::NOT_ACCEPTABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<serde_json::Error> for MajServerError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value.to_string())
    }
}

impl From<std::io::Error> for MajServerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

// Error Boilerplate
impl core::fmt::Display for MajServerError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for MajServerError {}

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
    let errors: Vec<MajServerError> = errors
        .into_iter()
        .filter_map(|(_k, e)| e.downcast_ref::<MajServerError>().cloned())
        .inspect(|e| error!("{:?}", e))
        .collect();

    // Only the response code for the first error is actually sent from the server
    // this may be customized by the specific application
    #[cfg(feature = "ssr")]
    {
        let response = use_context::<ResponseOptions>();
        if let Some(response) = response {
            response.set_status(errors[0].status_code());
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
