use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PresenceError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("No presence status for {0}")]
    AccountNotFound(Uuid),
    #[error("Sending presence status failed: {0}")]
    SendingFailed(String),
}

impl IntoResponse for PresenceError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let status = match self {
            PresenceError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PresenceError::AccountNotFound(_) => StatusCode::NOT_FOUND,
            PresenceError::SendingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
