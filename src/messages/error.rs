use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;
use uuid::Error as UuidError;

use crate::{account::database::AccountDatabaseError, notifications::gateway::NotificationError};

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error("Notification delivery failed: {0}")]
    NotificationError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl From<AccountDatabaseError> for MessagingError {
    fn from(err: AccountDatabaseError) -> Self {
        match err {
            AccountDatabaseError::NotFound(username) => MessagingError::UserNotFound(username),
            AccountDatabaseError::OperationFailed => {
                MessagingError::DatabaseError("Account database operation failed".to_string())
            }
        }
    }
}

impl From<NotificationError> for MessagingError {
    fn from(err: NotificationError) -> Self {
        MessagingError::NotificationError(err.to_string())
    }
}

impl From<UuidError> for MessagingError {
    fn from(err: UuidError) -> Self {
        MessagingError::ParseError(err.to_string())
    }
}

impl IntoResponse for MessagingError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let status = match self {
            MessagingError::UserNotFound(_) => StatusCode::BAD_REQUEST,
            MessagingError::ParseError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
