use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("Key bundle validation failed: {0}")]
    ValidationError(String),

    #[error("Key {0} not found")]
    NotFound(String),

    #[error("Duplicate prekey with index {0}")]
    DuplicatePrekey(u64),

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error("Prism client error: {0}")]
    PrismClientError(String),

    #[error("Unspecified error: {0}")]
    UnspecifiedError(String),
}

impl From<anyhow::Error> for KeyError {
    fn from(err: anyhow::Error) -> Self {
        KeyError::UnspecifiedError(err.to_string())
    }
}

impl IntoResponse for KeyError {
    fn into_response(self) -> Response {
        let status = match self {
            KeyError::ValidationError(_) => StatusCode::BAD_REQUEST,
            KeyError::NotFound(_) => StatusCode::NOT_FOUND,
            KeyError::DuplicatePrekey(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
