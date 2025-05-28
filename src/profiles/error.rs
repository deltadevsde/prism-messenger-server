use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

use crate::account::database::AccountDatabaseError;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum ProfileError {
    #[error("Profile not found")]
    NotFound,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AccountDatabaseError> for ProfileError {
    fn from(err: AccountDatabaseError) -> Self {
        match err {
            AccountDatabaseError::NotFound(_) => Self::NotFound,
            AccountDatabaseError::OperationFailed => {
                Self::Database("Account database operation failed".to_string())
            }
        }
    }
}

impl From<sqlx::Error> for ProfileError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl IntoResponse for ProfileError {
    fn into_response(self) -> Response {
        error!("{}", self);
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
