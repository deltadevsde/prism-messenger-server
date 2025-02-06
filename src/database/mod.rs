mod inmemory;
use anyhow::Result;

use crate::{
    keys::service::{KeyBundle, Prekey},
    messages::service::Message,
};

pub trait Database {
    fn insert_keybundle(&self, user: String, key_bundle: KeyBundle) -> Result<bool>;
    fn get_keybundle(&self, user: String) -> Result<Option<KeyBundle>>;
    fn add_prekeys(&self, user: String, prekeys: Vec<Prekey>) -> Result<bool>;

    fn insert_message(&self, message: Message) -> Result<bool>;
    fn get_messages(&self, user: String) -> Result<Vec<Message>>;
    fn mark_delivered(&self, user: String, ids: Vec<uuid::Uuid>) -> Result<bool>;
}
