use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::crypto::salted_hash::SaltedHash;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum AccountIdentity {
    Username(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub identities: Vec<AccountIdentity>,
    pub auth_password_hash: SaltedHash,
    pub apns_token: Option<Vec<u8>>,
    pub gcm_token: Option<Vec<u8>>,
}

impl Account {
    pub fn new(
        initial_identity: AccountIdentity,
        auth_password: &str,
        apns_token: Option<Vec<u8>>,
        gcm_token: Option<Vec<u8>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            identities: vec![initial_identity],
            auth_password_hash: SaltedHash::generate_from(auth_password),
            apns_token,
            gcm_token,
        }
    }
}
