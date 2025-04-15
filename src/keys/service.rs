use prism_client::{Account as PrismAccount, HashedMerkleProof, PrismApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::{
    database::KeyDatabase,
    entities::{KeyBundle, Prekey},
    error::KeyError,
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

    pub async fn upload_key_bundle(
        &self,
        username: &str,
        bundle: KeyBundle,
    ) -> Result<(), KeyError> {
        bundle
            .verify()
            .map_err(|e| KeyError::ValidationError(e.to_string()))?;

        // A key bundle can be inserted before the user has been successfully
        // added to prism's state.
        self.db.insert_keybundle(username, bundle).await
    }

    // Note: There is no extra security assumption here: Even if the server is
    // malicious and adds extra prekeys for a user, the server will still be
    // unable to decrypt anything, and the receiver simply won't be able to
    // decrypt the messages either.
    pub async fn add_prekeys(&self, username: &str, prekeys: Vec<Prekey>) -> Result<(), KeyError> {
        let key_bundle = self.db.get_keybundle(username).await?;
        if key_bundle.is_none() {
            return Err(KeyError::NotFound(username.to_string()));
        }

        // ensure no duplicate prekeys
        let existing_ids = key_bundle
            .unwrap()
            .prekeys
            .iter()
            .map(|prekey| prekey.key_idx)
            .collect::<Vec<_>>();

        let potential_duplicate_key_idx = prekeys
            .iter()
            .map(|prekey| prekey.key_idx)
            .find(|key_idx| existing_ids.contains(key_idx));

        if let Some(duplicate_key_idx) = potential_duplicate_key_idx {
            return Err(KeyError::DuplicatePrekey(duplicate_key_idx));
        }
        self.db.add_prekeys(username, prekeys).await
    }

    pub async fn get_keybundle(&self, username: &str) -> Result<KeyBundleResponse, KeyError> {
        let keybundle = self.db.get_keybundle(username).await?;
        // TODO: clarify whether prism will store user_id or username
        let account_response = self
            .prism
            .get_account(username)
            .await
            .map_err(|e| KeyError::PrismClientError(e.to_string()))?;

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
