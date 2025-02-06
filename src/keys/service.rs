use std::sync::Arc;

use anyhow::{anyhow, Result};
use prism_keys::{Signature, VerifyingKey};
use prism_prover::{prover::AccountResponse, Prover};

use crate::database::Database;

#[derive(Clone)]
pub struct Prekey {
    pub key_idx: u32,
    pub key: VerifyingKey,
}

/// The complete key bundle contains the long-term identity key,
/// the signed pre-key (with its signature), and a list of one-time pre-keys.
#[derive(Clone)]
pub struct KeyBundle {
    pub identity_key: VerifyingKey,
    pub signed_prekey: VerifyingKey,
    pub signed_prekey_signature: Signature,
    pub prekeys: Vec<Prekey>,
}

impl KeyBundle {
    pub fn verify(&self) {
        // Ensure signature is signed_prekey signed by identity_key
        // Ensure prekeys have no duplicate IDs
        todo!()
    }
}

pub struct KeyBundleResponse {
    pub key_bundle: KeyBundle,
    pub account: AccountResponse,
}

pub struct KeyService {
    prover: Arc<Prover>,
    db: Arc<dyn Database>,
}

impl KeyService {
    pub fn new(prover: Arc<Prover>, db: Arc<dyn Database>) -> Self {
        Self { prover, db }
    }

    pub fn upload_key_bundle(&self, user_id: &str, bundle: KeyBundle) -> Result<bool> {
        bundle.verify();
        self.db.insert_keybundle(user_id.to_string(), bundle)
    }

    // Note: There is no extra security assumption here: Even if the server is
    // malicious and adds extra prekeys for a user, the server will still be
    // unable to decrypt anything, and the receiver simply won't be able to
    // decrypt the messages either.
    pub fn add_prekeys(&self, user_id: &str, prekeys: Vec<Prekey>) -> Result<bool> {
        self.db.add_prekeys(user_id.to_string(), prekeys)
    }

    pub async fn get_keybundle(&self, user_id: &str) -> Result<KeyBundleResponse> {
        let keybundle = self.db.get_keybundle(user_id.to_string())?;
        match keybundle {
            Some(bundle) => {
                let account = self.prover.get_account(user_id).await?;
                Ok(KeyBundleResponse {
                    key_bundle: bundle,
                    account,
                })
            }
            None => Err(anyhow!("Key bundle not found")),
        }
    }
}
