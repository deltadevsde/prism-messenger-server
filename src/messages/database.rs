use anyhow::Result;
use uuid::Uuid;

use super::entities::Message;

pub trait MessageDatabase {
    fn insert_message(&self, message: Message) -> Result<bool>;
    fn get_messages(&self, user: &str) -> Result<Vec<Message>>;
    fn mark_delivered(&self, user: &str, ids: Vec<Uuid>) -> Result<bool>;
}
