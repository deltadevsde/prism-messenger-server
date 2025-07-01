use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum TypingGatewayError {
    #[error("Failed to send message: {0}")]
    SendingFailed(String),
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    #[error("Recipient not connected: {0}")]
    RecipientNotConnected(String),
}

#[derive(Clone, Debug)]
pub struct TypingStatus {
    pub recipient_id: Uuid,
    pub sender_id: Uuid,
    pub is_typing: bool,
}

/// Provides methods for sending typing updates and registering handlers
/// for incoming typing status notifications.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TypingGateway: Send + Sync {
    /// Sends a typing status update to the specified recipient.
    ///
    /// # Arguments
    ///
    /// * `typing_status` - A reference to the `TypingStatus` to be sent.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or a `TypingGatewayError` if the update fails.
    async fn send_typing_update(
        &self,
        typing_status: &TypingStatus,
    ) -> Result<(), TypingGatewayError>;

    /// Registers a handler for incoming typing status updates.
    ///
    /// The provided handler function will be called whenever a new typing status
    /// update is received.
    ///
    /// # Arguments
    ///
    /// * `handler` - A closure that takes a `TypingStatus` as input.
    async fn register_typing_handler<H>(&self, handler: H)
    where
        H: Fn(TypingStatus) + Send + Sync + 'static;
}
