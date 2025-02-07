use anyhow::Result;

use super::entities::{KeyBundle, Prekey};

pub trait KeyDatabase {
    fn insert_keybundle(&self, user: String, key_bundle: KeyBundle) -> Result<bool>;
    fn get_keybundle(&self, user: String) -> Result<Option<KeyBundle>>;
    fn add_prekeys(&self, user: String, prekeys: Vec<Prekey>) -> Result<bool>;
}
