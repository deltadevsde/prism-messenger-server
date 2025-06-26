use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    account::{
        database::{AccountDatabase, AccountDatabaseError},
        entities::Account,
    },
    keys::{
        database::KeyDatabase,
        entities::{KeyBundle, Prekey},
        error::KeyError,
    },
    messages::{database::MessageDatabase, entities::Message, error::MessagingError},
    profiles::{database::ProfileDatabase, entities::Profile, error::ProfileError},
};

pub struct InMemoryDatabase {
    pub accounts: Mutex<HashMap<Uuid, Account>>,
    pub key_bundles: Mutex<HashMap<Uuid, KeyBundle>>,
    pub messages: Mutex<HashMap<Uuid, Vec<Message>>>,
    pub profiles: RwLock<HashMap<Uuid, Profile>>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        InMemoryDatabase {
            accounts: Mutex::new(HashMap::new()),
            key_bundles: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl AccountDatabase for InMemoryDatabase {
    async fn upsert_account(&self, account: Account) -> Result<(), AccountDatabaseError> {
        let mut account_lock = self
            .accounts
            .lock()
            .map_err(|_| AccountDatabaseError::OperationFailed)?;

        account_lock.insert(account.id, account);
        Ok(())
    }

    async fn fetch_account(&self, id: Uuid) -> Result<Option<Account>, AccountDatabaseError> {
        let account_lock = self
            .accounts
            .lock()
            .map_err(|_| AccountDatabaseError::OperationFailed)?;

        let account = account_lock.get(&id).cloned();
        Ok(account)
    }

    async fn remove_account(&self, id: Uuid) -> Result<(), AccountDatabaseError> {
        let mut account_lock = self
            .accounts
            .lock()
            .map_err(|_| AccountDatabaseError::OperationFailed)?;

        account_lock.remove(&id);
        Ok(())
    }

    async fn update_apns_token(
        &self,
        id: Uuid,
        token: Vec<u8>,
    ) -> Result<(), AccountDatabaseError> {
        let mut account_lock = self
            .accounts
            .lock()
            .map_err(|_| AccountDatabaseError::OperationFailed)?;

        let Some(account) = account_lock.get_mut(&id) else {
            return Err(AccountDatabaseError::OperationFailed);
        };

        account.apns_token = Some(token);
        Ok(())
    }
}

#[async_trait]
impl KeyDatabase for InMemoryDatabase {
    async fn insert_keybundle(
        &self,
        account_id: Uuid,
        key_bundle: KeyBundle,
    ) -> Result<(), KeyError> {
        let mut kb_lock = self
            .key_bundles
            .lock()
            .map_err(|_| KeyError::DatabaseError("Lock poisoned".to_string()))?;
        kb_lock.insert(account_id, key_bundle);
        Ok(())
    }

    async fn get_keybundle(&self, account_id: Uuid) -> Result<Option<KeyBundle>, KeyError> {
        let kb_lock = self
            .key_bundles
            .lock()
            .map_err(|_| KeyError::DatabaseError("Lock poisoned".to_string()))?;
        // Return a clone of the key bundle if it exists.
        Ok(kb_lock.get(&account_id).cloned())
    }

    async fn add_prekeys(&self, account_id: Uuid, prekeys: Vec<Prekey>) -> Result<(), KeyError> {
        let mut kb_lock = self
            .key_bundles
            .lock()
            .map_err(|_| KeyError::DatabaseError("Lock poisoned".to_string()))?;

        if let Some(bundle) = kb_lock.get_mut(&account_id) {
            // TODO: Ensure no duplicate prekey ids are added.
            bundle.prekeys.extend(prekeys);
            Ok(())
        } else {
            Err(KeyError::NotFound(account_id.to_string()))
        }
    }
}

impl MessageDatabase for InMemoryDatabase {
    fn insert_message(&self, message: Message) -> Result<(), MessagingError> {
        let mut messages_lock = self.messages.lock().map_err(|e| {
            MessagingError::DatabaseError(format!("Lock poisoned during message storage: {}", e))
        })?;
        messages_lock
            .entry(message.recipient_id)
            .or_insert_with(Vec::new)
            .push(message);
        Ok(())
    }

    fn get_all_messages(&self) -> Result<Vec<Message>, MessagingError> {
        let messages_lock = self.messages.lock().map_err(|e| {
            MessagingError::DatabaseError(format!("Lock poisoned during message retrieval: {}", e))
        })?;
        // Return a cloned vector of messages
        Ok(messages_lock.values().flatten().cloned().collect())
    }

    fn get_messages_for_account(&self, account_id: Uuid) -> Result<Vec<Message>, MessagingError> {
        let messages_lock = self.messages.lock().map_err(|e| {
            MessagingError::DatabaseError(format!("Lock poisoned during message retrieval: {}", e))
        })?;
        // Return a cloned vector of messages for a specific account
        Ok(messages_lock.get(&account_id).cloned().unwrap_or_default())
    }

    fn remove_messages(&self, account_id: Uuid, ids: Vec<Uuid>) -> Result<(), MessagingError> {
        let mut messages_lock = self.messages.lock().map_err(|e| {
            MessagingError::DatabaseError(format!(
                "Lock poisoned during message delivery status update: {}",
                e
            ))
        })?;
        let Some(messages) = messages_lock.get_mut(&account_id) else {
            return Ok(());
        };

        // Remove any messages whose message_id is in ids
        messages.retain(|msg| !ids.contains(&msg.message_id));
        Ok(())
    }
}

#[async_trait]
impl ProfileDatabase for InMemoryDatabase {
    async fn get_profile_by_id(&self, id: Uuid) -> Result<Option<Profile>, ProfileError> {
        let profiles = self
            .profiles
            .read()
            .map_err(|e| ProfileError::Database(e.to_string()))?;
        Ok(profiles.get(&id).cloned())
    }

    async fn get_profile_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<Option<Profile>, ProfileError> {
        let profiles = self
            .profiles
            .read()
            .map_err(|e| ProfileError::Database(e.to_string()))?;

        // Find the profile with matching account_id
        let matching_profile = profiles
            .values()
            .find(|profile| profile.account_id == account_id)
            .cloned();
        Ok(matching_profile)
    }

    async fn get_profile_by_username(
        &self,
        username: &str,
    ) -> Result<Option<Profile>, ProfileError> {
        let profiles = self
            .profiles
            .read()
            .map_err(|e| ProfileError::Database(e.to_string()))?;

        // Find the profile with matching username
        let matching_profile = profiles
            .values()
            .find(|profile| profile.username == username)
            .cloned();
        Ok(matching_profile)
    }

    async fn upsert_profile(&self, profile: Profile) -> Result<(), ProfileError> {
        let mut profiles = self
            .profiles
            .write()
            .map_err(|e| ProfileError::Database(e.to_string()))?;

        // Store the profile, overwriting any existing profile with the same ID
        profiles.insert(profile.id, profile.clone());

        Ok(())
    }
}
