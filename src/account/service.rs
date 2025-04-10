use anyhow::Result;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use prism_client::PrismApi;
use std::sync::Arc;
use uuid::Uuid;

use crate::account::database::{AccountDatabase, AccountDatabaseError};

#[derive(Debug, thiserror::Error)]
pub enum AccountServiceError {
    #[error("Account not found")]
    AccountNotFound,

    #[error("Database error: {0}")]
    DatabaseError(#[from] AccountDatabaseError),
}

impl IntoResponse for AccountServiceError {
    fn into_response(self) -> Response {
        let status = match self {
            AccountServiceError::AccountNotFound => StatusCode::NOT_FOUND,
            AccountServiceError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}

pub struct AccountService<P: PrismApi, D: AccountDatabase> {
    prism: Arc<P>,
    account_db: Arc<D>,
}

impl<P: PrismApi, D: AccountDatabase> AccountService<P, D> {
    pub fn new(prism: Arc<P>, account_db: Arc<D>) -> Self {
        Self { prism, account_db }
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let account_res = self.prism.clone().get_account(username).await?;

        Ok(account_res.account.is_some())
    }

    /// Updates an account's APNS token
    pub async fn update_apns_token(
        &self,
        account_id: Uuid,
        token: Vec<u8>,
    ) -> Result<(), AccountServiceError> {
        self.account_db
            .update_apns_token(account_id, token)
            .await
            .map_err(|err| match err {
                AccountDatabaseError::NotFound(_) => AccountServiceError::AccountNotFound,
                _ => AccountServiceError::DatabaseError(err),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::account::database::MockAccountDatabase;
    use crate::account::service::AccountService;
    use mockall::predicate::eq;
    use prism_client::{Account, AccountResponse, HashedMerkleProof, mock::MockPrismApi};

    #[tokio::test]
    async fn test_username_exists_returns_true_when_found() {
        let mut mock_client = MockPrismApi::new();
        mock_client
            .expect_get_account()
            .once()
            .with(eq("test"))
            .returning(|_| {
                Ok(AccountResponse {
                    account: Some(Account::default()),
                    proof: HashedMerkleProof::empty(),
                })
            });

        let mock_db = MockAccountDatabase::new();
        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let exists = service.username_exists("test").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_username_exists_returns_false_when_not_found() {
        let mut mock_client = MockPrismApi::new();
        mock_client
            .expect_get_account()
            .once()
            .with(eq("test"))
            .returning(|_| {
                Ok(AccountResponse {
                    account: None,
                    proof: HashedMerkleProof::empty(),
                })
            });
        let mock_db = MockAccountDatabase::new();
        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let exists = service.username_exists("test").await.unwrap();
        assert!(!exists);
    }
}
