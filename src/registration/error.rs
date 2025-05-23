use axum::{http::StatusCode, response::IntoResponse};
use prism_client::{PrismApiError, TransactionError};

use crate::account::database::AccountDatabaseError;

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
    #[error("Push token is missing")]
    MissingPushToken,
}

impl From<TransactionError> for RegistrationError {
    fn from(err: TransactionError) -> Self {
        Self::ProcessingFailed(err.to_string())
    }
}

impl From<PrismApiError> for RegistrationError {
    fn from(err: PrismApiError) -> Self {
        Self::ProcessingFailed(err.to_string())
    }
}

impl From<AccountDatabaseError> for RegistrationError {
    fn from(err: AccountDatabaseError) -> Self {
        Self::ProcessingFailed(err.to_string())
    }
}

impl IntoResponse for RegistrationError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            RegistrationError::MissingPushToken => StatusCode::BAD_REQUEST,
            RegistrationError::ProcessingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
