use uuid::Uuid;

use super::entities::Message;
use super::error::MessagingError;

#[cfg_attr(test, mockall::automock)]
pub trait MessageDatabase: Send + Sync {
    fn insert_message(&self, message: Message) -> Result<(), MessagingError>;
    fn get_all_messages(&self) -> Result<Vec<Message>, MessagingError>;
    fn get_messages_for_account(&self, account_id: Uuid) -> Result<Vec<Message>, MessagingError>;
    fn remove_messages(&self, account_id: Uuid, ids: Vec<Uuid>) -> Result<(), MessagingError>;
}
