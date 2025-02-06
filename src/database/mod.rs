pub mod inmemory;
use anyhow::Result;

use crate::messages::service::Message;

pub trait Database {
    fn insert_message(&self, message: Message) -> Result<bool>;
    fn get_messages(&self, user: String) -> Result<Vec<Message>>;
    fn mark_delivered(&self, user: String, ids: Vec<uuid::Uuid>) -> Result<bool>;
}
