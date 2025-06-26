use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum PresenceError {
    #[error("Database error: {0}")]
    Database(String),
}

impl IntoResponse for PresenceError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let status = match self {
            PresenceError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
