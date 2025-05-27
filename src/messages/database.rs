use uuid::Uuid;

use super::entities::Message;
use super::error::MessagingError;

#[cfg_attr(test, mockall::automock)]
pub trait MessageDatabase: Send + Sync {
    fn insert_message(&self, message: Message) -> Result<bool, MessagingError>;
    fn get_messages(&self, account_id: Uuid) -> Result<Vec<Message>, MessagingError>;
    fn mark_delivered(&self, account_id: Uuid, ids: Vec<Uuid>) -> Result<bool, MessagingError>;
}
