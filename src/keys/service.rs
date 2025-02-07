use anyhow::Result;
use prism_common::account::Account;
use prism_tree::proofs::HashedMerkleProof;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::common::prism_client::PrismClient;

use super::{
    database::KeyDatabase,
    entities::{KeyBundle, Prekey},
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct KeyBundleResponse {
    pub key_bundle: Option<KeyBundle>,
    pub account: Option<Account>,
    pub proof: HashedMerkleProof,
}

pub struct KeyService<C, D>
where
    C: PrismClient,
    D: KeyDatabase,
{
    client: Arc<C>,
    db: Arc<D>,
}

impl<C, D> KeyService<C, D>
where
    C: PrismClient,
    D: KeyDatabase,
{
    pub fn new(client: Arc<C>, db: Arc<D>) -> Self {
        Self { client, db }
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
        let account_response = self.client.get_account(user_id).await?;

        let response = KeyBundleResponse {
            key_bundle: keybundle,
            account: account_response.account,
            proof: account_response.proof,
        };
        Ok(response)
    }
}
