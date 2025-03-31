use axum::http::StatusCode;
use prism_client::{PrismApiError, TransactionError};

use crate::account::database::AccountDatabaseError;

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
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

impl From<RegistrationError> for StatusCode {
    fn from(err: RegistrationError) -> Self {
        match err {
            RegistrationError::ProcessingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
