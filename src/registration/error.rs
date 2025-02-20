use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use prism_client::TransactionError;

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
