use anyhow::{Result, anyhow};
use prism_client::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Prekey {
    pub key_idx: u64,
    pub key: VerifyingKey,
}

/// The complete key bundle contains the long-term identity key,
/// the signed pre-key (with its signature), and a list of one-time pre-keys.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyBundle {
    pub identity_key: VerifyingKey,
    pub signed_prekey: VerifyingKey,
    pub signed_prekey_signature: Signature,
    pub prekeys: Vec<Prekey>,
}

impl KeyBundle {
    pub fn verify(&self) -> Result<()> {
        // Ensure signature is signed_prekey signed by identity_key
        let msg = self.signed_prekey.to_spki_der()?;
        self.identity_key
            .verify_signature(&msg, &self.signed_prekey_signature)?;
        // Ensure prekeys have no duplicate IDs
        for prekey in &self.prekeys {
            if self
                .prekeys
                .iter()
                .filter(|k| k.key_idx == prekey.key_idx)
                .count()
                > 1
            {
                return Err(anyhow!("Duplicate prekey ID"));
            }
        }
        Ok(())
    }
}
