use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Mutex;

use super::Database;
use crate::keys::service::{KeyBundle, Prekey};
use crate::messages::service::Message;

pub struct InMemoryDatabase {
    pub key_bundles: Mutex<HashMap<String, KeyBundle>>,
    pub messages: Mutex<HashMap<String, Vec<Message>>>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        InMemoryDatabase {
            key_bundles: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
        }
    }
}

impl Database for InMemoryDatabase {
    fn insert_keybundle(&self, user: String, key_bundle: KeyBundle) -> Result<bool> {
        let mut kb_lock = self
            .key_bundles
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        kb_lock.insert(user, key_bundle);
        Ok(true)
    }

    fn get_keybundle(&self, user: String) -> Result<Option<KeyBundle>> {
        let kb_lock = self
            .key_bundles
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        // Return a clone of the key bundle if it exists.
        Ok(kb_lock.get(&user).cloned())
    }

    fn add_prekeys(&self, user: String, prekeys: Vec<Prekey>) -> Result<bool> {
        let mut kb_lock = self
            .key_bundles
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;

        if let Some(bundle) = kb_lock.get_mut(&user) {
            // TODO: Ensure no duplicate prekey ids are added.
            bundle.prekeys.extend(prekeys);
            Ok(true)
        } else {
            Err(anyhow!("Key bundle not found for user: {}", user))
        }
    }

    fn insert_message(&self, message: Message) -> Result<bool> {
        let user = message.recipient_id.clone();
        let mut messages_lock = self
            .messages
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        messages_lock
            .entry(user)
            .or_insert_with(Vec::new)
            .push(message);
        Ok(true)
    }

    fn get_messages(&self, user: String) -> Result<Vec<Message>> {
        let messages_lock = self
            .messages
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        // Return a cloned vector of messages (if any) so that users can work on their own copy.
        Ok(messages_lock.get(&user).cloned().unwrap_or_default())
    }

    fn mark_delivered(&self, user: String, ids: Vec<uuid::Uuid>) -> Result<bool> {
        let mut messages_lock = self
            .messages
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        if let Some(messages) = messages_lock.get_mut(&user) {
            let original_len = messages.len();
            // Remove any messages whose message_id is in ids
            messages.retain(|msg| !ids.contains(&msg.message_id));
            Ok(messages.len() != original_len)
        } else {
            Ok(false)
        }
    }
}
