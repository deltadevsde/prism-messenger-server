use anyhow::{Result, anyhow};
use prism_client::{Account as PrismAccount, HashedMerkleProof, PrismApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::{
    database::KeyDatabase,
    entities::{KeyBundle, Prekey},
};

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyBundleResponse {
    pub key_bundle: Option<KeyBundle>,
    pub account: Option<PrismAccount>,
    pub proof: HashedMerkleProof,
}

pub struct KeyService<P, D>
where
    P: PrismApi,
    D: KeyDatabase,
{
    prism: Arc<P>,
    db: Arc<D>,
}

impl<P, D> KeyService<P, D>
where
    P: PrismApi,
    D: KeyDatabase,
{
    pub fn new(prism: Arc<P>, db: Arc<D>) -> Self {
        Self { prism, db }
    }

    pub fn upload_key_bundle(&self, username: &str, bundle: KeyBundle) -> Result<bool> {
        bundle.verify()?;

        // A key bundle can be inserted before the user has been successfully
        // added to prism's state.
        self.db.insert_keybundle(username, bundle)
    }

    // Note: There is no extra security assumption here: Even if the server is
    // malicious and adds extra prekeys for a user, the server will still be
    // unable to decrypt anything, and the receiver simply won't be able to
    // decrypt the messages either.
    pub fn add_prekeys(&self, username: &str, prekeys: Vec<Prekey>) -> Result<bool> {
        let key_bundle = self.db.get_keybundle(username)?;
        if key_bundle.is_none() {
            return Err(anyhow!(
                "User either does not exist or has not uploaded a key bundle"
            ));
        }

        // ensure no duplicate prekeys
        let existing_ids = key_bundle
            .unwrap()
            .prekeys
            .iter()
            .map(|prekey| prekey.key_idx)
            .collect::<Vec<_>>();
        if prekeys
            .iter()
            .any(|prekey| existing_ids.contains(&prekey.key_idx))
        {
            return Err(anyhow!("Duplicate prekey ID"));
        }
        self.db.add_prekeys(username, prekeys)
    }

    pub async fn get_keybundle(&self, username: &str) -> Result<KeyBundleResponse> {
        let keybundle = self.db.get_keybundle(username)?;
        // TODO: clarify whether prism will store user_id or username
        let account_response = self.prism.get_account(username).await?;

        let response = KeyBundleResponse {
            key_bundle: keybundle,
            account: account_response.account,
            proof: account_response.proof,
        };
        Ok(response)
    }
}

#[cfg(test)]
mod tests {}
