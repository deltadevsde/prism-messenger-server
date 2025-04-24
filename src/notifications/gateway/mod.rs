pub mod apns;
pub mod dummy;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Failed to send notification: {0}")]
    SendFailure(String),

    #[error("Failed to initialize notification service: {0}")]
    InitializationFailed(String),
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NotificationGateway {
    async fn send_silent_notification(&self, device_token: &[u8]) -> Result<(), NotificationError>;
}
