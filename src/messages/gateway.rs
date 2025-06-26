use async_trait::async_trait;

use crate::websocket::center::WebSocketError;

use super::entities::Message;

#[derive(Debug, thiserror::Error)]
pub enum MessageGatewayError {
    #[error("Failed to send message: {0}")]
    SendingFailed(String),
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    #[error("Recipient not connected: {0}")]
    RecipientNotConnected(String),
}

impl From<WebSocketError> for MessageGatewayError {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::SerializationFailed(msg) => {
                MessageGatewayError::InvalidMessageFormat(msg)
            }
            WebSocketError::SendingFailed(msg) => MessageGatewayError::SendingFailed(msg),
            WebSocketError::ConnectionNotFound(account_id) => {
                MessageGatewayError::RecipientNotConnected(account_id)
            }
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MessageGateway: Send + Sync {
    async fn send_message(&self, message: Message) -> Result<(), MessageGatewayError>;
}
