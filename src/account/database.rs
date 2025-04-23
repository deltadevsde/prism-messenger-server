use async_trait::async_trait;
use uuid::Uuid;

use super::entities::Account;

#[derive(Debug, thiserror::Error)]
pub enum AccountDatabaseError {
    #[error("Database operation failed")]
    OperationFailed,
    #[error("Account not found: {0}")]
    NotFound(String),
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountDatabase {
    async fn upsert_account(&self, account: Account) -> Result<(), AccountDatabaseError>;
    async fn fetch_account(&self, id: Uuid) -> Result<Account, AccountDatabaseError>;
    async fn fetch_account_by_username(
        &self,
        username: &str,
    ) -> Result<Account, AccountDatabaseError>;
    async fn remove_account(&self, id: Uuid) -> Result<(), AccountDatabaseError>;
    async fn update_apns_token(&self, id: Uuid, token: Vec<u8>)
    -> Result<(), AccountDatabaseError>;
}
