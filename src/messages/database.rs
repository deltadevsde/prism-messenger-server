use uuid::Uuid;

use super::entities::Message;
use super::error::MessagingError;

#[cfg_attr(test, mockall::automock)]
pub trait MessageDatabase {
    fn insert_message(&self, message: Message) -> Result<bool, MessagingError>;
    fn get_messages(&self, user: &str) -> Result<Vec<Message>, MessagingError>;
    fn mark_delivered(&self, user: &str, ids: Vec<Uuid>) -> Result<bool, MessagingError>;
}
