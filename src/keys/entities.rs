use prism_keys::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Prekey {
    pub key_idx: u32,
    pub key: VerifyingKey,
}

/// The complete key bundle contains the long-term identity key,
/// the signed pre-key (with its signature), and a list of one-time pre-keys.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
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
