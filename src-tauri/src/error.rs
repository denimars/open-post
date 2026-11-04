//! Application error type. Serializes to `{ kind, message }` for the frontend.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Could not resolve host: {0}")]
    Dns(String),

    #[error("Connection failed: {0}")]
    Connect(String),

    #[error("The request timed out after {0} ms")]
    Timeout(u64),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Request was cancelled")]
    Cancelled,

    #[error("Request failed: {0}")]
    Request(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Internal(String),
}

impl AppError {
    /// Stable machine-readable discriminant for the UI.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::InvalidUrl(_) => "invalidUrl",
            AppError::InvalidRequest(_) => "invalidRequest",
            AppError::Dns(_) => "dns",
            AppError::Connect(_) => "connect",
            AppError::Timeout(_) => "timeout",
            AppError::Tls(_) => "tls",
            AppError::Cancelled => "cancelled",
            AppError::Request(_) => "request",
            AppError::Storage(_) => "storage",
            AppError::NotFound(_) => "notFound",
            AppError::Internal(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(format!("serialization failed: {e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
