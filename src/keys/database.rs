use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    entities::{KeyBundle, Prekey},
    error::KeyError,
};

#[async_trait]
pub trait KeyDatabase: Send + Sync {
    async fn insert_keybundle(&self, account_id: Uuid, key_bundle: KeyBundle) -> Result<(), KeyError>;

    async fn get_keybundle(&self, account_id: Uuid) -> Result<Option<KeyBundle>, KeyError>;

    async fn add_prekeys(&self, account_id: Uuid, prekeys: Vec<Prekey>) -> Result<(), KeyError>;
}
