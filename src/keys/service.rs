use std::sync::Arc;

use anyhow::Result;
use prism_keys::{Signature, VerifyingKey};
use prism_prover::{prover::AccountResponse, Prover};

use crate::database::Database;

pub struct Prekey {
    pub key_idx: u32,
    pub key: VerifyingKey,
}

/// The complete key bundle contains the long-term identity key,
/// the signed pre-key (with its signature), and a list of one-time pre-keys.
pub struct KeyBundle {
    pub identity_key: VerifyingKey,
    pub signed_prekey: VerifyingKey,
    pub signed_prekey_signature: Signature,
    pub prekeys: Vec<Prekey>,
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

    // TODO: How do we authenticate
    pub fn upload_key_bundle(&self, bundle: KeyBundle) -> Result<bool> {
        unimplemented!()
    }

    // Note: There is no extra security assumption here: Even if the server is
    // malicious and adds extra prekeys for a user, the server will still be
    // unable to decrypt anything, and the receiver simply won't be able to
    // decrypt the messages either.
    pub fn add_prekeys(&self, user: &VerifyingKey, prekeys: Vec<Prekey>) -> Result<bool> {
        unimplemented!()
    }

    pub fn get_keybundle(&self, user: String) -> Result<KeyBundleResponse> {
        unimplemented!()
    }
}
