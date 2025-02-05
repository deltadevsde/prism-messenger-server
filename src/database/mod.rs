use anyhow::Result;

use crate::keys::service::{KeyBundle, Prekey};

pub trait Database {
    fn insert_keybundle(&self, user: String, key_bundle: KeyBundle) -> Result<()>;
    fn get_keybundle(&self, user: String) -> Result<Option<KeyBundle>>;
    fn add_prekeys(&self, user: String, prekeys: Vec<Prekey>) -> Result<()>;
    // TODO: Messages
}
