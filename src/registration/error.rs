use axum::http::StatusCode;
use prism_client::{PrismApiError, TransactionError};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum RegistrationError {
    ProcessingFailed,
}

impl Display for RegistrationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::ProcessingFailed => write!(f, "Processing registration failed"),
        }
    }
}

impl Error for RegistrationError {}

impl From<TransactionError> for RegistrationError {
    fn from(_: TransactionError) -> Self {
        Self::ProcessingFailed
    }
}

impl From<PrismApiError> for RegistrationError {
    fn from(_: PrismApiError) -> Self {
        Self::ProcessingFailed
    }
}

impl From<RegistrationError> for StatusCode {
    fn from(err: RegistrationError) -> Self {
        match err {
            RegistrationError::ProcessingFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
