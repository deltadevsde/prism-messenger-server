use super::service::Message;
use anyhow::Result;
use uuid::Uuid;

pub trait MessageDatabase {
    fn insert_message(&self, message: Message) -> Result<bool>;
    fn get_messages(&self, user: String) -> Result<Vec<Message>>;
    fn mark_delivered(&self, user: String, ids: Vec<Uuid>) -> Result<bool>;
}
