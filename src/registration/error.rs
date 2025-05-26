use axum::{http::StatusCode, response::IntoResponse};
use prism_client::{PrismApiError, TransactionError};

use crate::account::database::AccountDatabaseError;

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
    #[error("Push token is missing")]
    MissingPushToken,
    #[error("Invalid phone number format")]
    InvalidPhoneNumber,
    #[error("OTP verification failed")]
    OtpVerificationFailed,
    #[error("Phone registration session not found or expired")]
    PhoneSessionNotFound,
    #[error("Twilio error: {0}")]
    TwilioError(String),
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
            RegistrationError::InvalidPhoneNumber => StatusCode::BAD_REQUEST,
            RegistrationError::OtpVerificationFailed => StatusCode::BAD_REQUEST,
            RegistrationError::PhoneSessionNotFound => StatusCode::BAD_REQUEST,
            RegistrationError::TwilioError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RegistrationError::ProcessingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
