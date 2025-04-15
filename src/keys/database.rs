use anyhow::Result;
use async_trait::async_trait;

use super::{
    entities::{KeyBundle, Prekey},
    error::KeyError,
};

#[async_trait]
pub trait KeyDatabase {
    async fn insert_keybundle(&self, user_id: &str, key_bundle: KeyBundle) -> Result<(), KeyError>;

    async fn get_keybundle(&self, user_id: &str) -> Result<Option<KeyBundle>, KeyError>;

    async fn add_prekeys(&self, user_id: &str, prekeys: Vec<Prekey>) -> Result<(), KeyError>;
}
