use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::crypto::salted_hash::SaltedHash;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub auth_password_hash: SaltedHash,
}

impl Account {
    pub fn new(username: String, auth_password: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            auth_password_hash: SaltedHash::generate_from(auth_password),
        }
    }
}
