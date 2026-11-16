use http::status::StatusCode;
use leptos::server_fn::error::FromServerFnError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, thiserror::Error)]
pub enum MajServerError {
    #[error("Internal")]
    Internal,
    #[error("NotFound")]
    NotFound,
    #[error("Io({0:?})")]
    Io(String),
    #[error("SerdeJson({0:?})")]
    SerdeJson(String),
    #[error("LeptosServerFnError({0:?})")]
    LeptosServerFnError(String),
}

#[cfg(feature = "ssr")]
impl axum::response::IntoResponse for MajServerError {
    fn into_response(self) -> axum::response::Response {
        self.status_code().into_response()
    }
}

impl MajServerError {
    pub fn status_code(&self) -> StatusCode {
        #[allow(unreachable_patterns)]
        match self {
            MajServerError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<StatusCode> for MajServerError {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::NOT_FOUND => MajServerError::NotFound,
            _ => MajServerError::Internal,
        }
    }
}

impl FromServerFnError for MajServerError {
    type Encoder = leptos::server_fn::codec::JsonEncoding;

    fn from_server_fn_error(value: leptos::prelude::ServerFnErrorErr) -> Self {
        Self::LeptosServerFnError(value.to_string())
    }
}

impl From<serde_json::Error> for MajServerError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value.to_string())
    }
}

// Keep owned strings at the serialized server-function boundary rather than retaining sources.
impl From<std::io::Error> for MajServerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_errors_preserve_display_serialization_and_http_status() {
        // These errors cross the server-function boundary; keep their existing representation.
        for error in [
            MajServerError::Internal,
            MajServerError::NotFound,
            MajServerError::Io("path with \"quotes\"\nand Unicode é".into()),
            MajServerError::SerdeJson("invalid JSON".into()),
            MajServerError::LeptosServerFnError("server function failed".into()),
        ] {
            assert_eq!(error.to_string(), format!("{error:?}"));
            let encoded = serde_json::to_string(&error).unwrap();
            let decoded: MajServerError = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded.to_string(), error.to_string());
            assert_eq!(decoded.status_code(), error.status_code());
            assert!(std::error::Error::source(&error).is_none());
        }
    }
}
