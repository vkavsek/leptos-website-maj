use http::status::StatusCode;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub enum MajServerError {
    Internal,
    NotFound,
    Io(String),
    SerdeJson(String),
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
