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

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TypingGateway: Send + Sync {
    async fn send_typing_update(
        &self,
        typing_status: &TypingStatus,
    ) -> Result<(), TypingGatewayError>;
    async fn register_typing_handler<H>(&self, handler: H)
    where
        H: Fn(TypingStatus) + Send + Sync + 'static;
}
