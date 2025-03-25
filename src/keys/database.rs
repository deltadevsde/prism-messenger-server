use anyhow::Result;

use super::entities::{KeyBundle, Prekey};

pub trait KeyDatabase {
    fn insert_keybundle(&self, user_id: &str, key_bundle: KeyBundle) -> Result<bool>;
    fn get_keybundle(&self, user_id: &str) -> Result<Option<KeyBundle>>;
    fn add_prekeys(&self, user_id: &str, prekeys: Vec<Prekey>) -> Result<bool>;
}
