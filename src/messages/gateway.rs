use async_trait::async_trait;

use super::{entities::Message, error::MessagingError};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MessageGateway: Send + Sync {
    async fn send_message(&self, message: Message) -> Result<(), MessagingError>;
}
