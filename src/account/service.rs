use anyhow::Result;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use prism_client::PrismApi;
use std::sync::Arc;
use uuid::Uuid;

use crate::account::database::{AccountDatabase, AccountDatabaseError};
use crate::account::entities::AccountIdentity;

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

    pub async fn identity_exists(&self, identity: &AccountIdentity) -> Result<bool> {
        match identity {
            AccountIdentity::Username(username) => {
                let account_res = self.prism.clone().get_account(username).await?;
                Ok(account_res.account.is_some())
            }
        }
    }

    /// Fetches an account ID by its identity
    pub async fn get_account_id_by_identity(
        &self,
        identity: &AccountIdentity,
    ) -> Result<Uuid, AccountServiceError> {
        self.account_db
            .fetch_account_by_identity(identity)
            .await?
            .map(|acc| acc.id)
            .ok_or(AccountServiceError::AccountNotFound)
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

    use crate::account::database::{AccountDatabaseError, MockAccountDatabase};
    use crate::account::entities::AccountIdentity;
    use crate::account::service::{AccountService, AccountServiceError};
    use mockall::predicate::eq;
    use prism_client::{Account, AccountResponse, HashedMerkleProof, mock::MockPrismApi};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_identity_exists_returns_true_when_found() {
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
        let identity = AccountIdentity::Username("test".to_string());
        let exists = service.identity_exists(&identity).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_identity_exists_returns_false_when_not_found() {
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
        let identity = AccountIdentity::Username("test".to_string());
        let exists = service.identity_exists(&identity).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_get_account_id_by_identity() {
        let mock_client = MockPrismApi::new();
        let mut mock_db = MockAccountDatabase::new();
        let account_id = Uuid::new_v4();
        let identity = AccountIdentity::Username("test".to_string());

        mock_db
            .expect_fetch_account_by_identity()
            .once()
            .with(eq(identity.clone()))
            .returning(move |_| {
                Ok(Some(crate::account::entities::Account {
                    id: account_id,
                    identities: vec![AccountIdentity::Username("test".to_string())],
                    auth_password_hash: crate::crypto::salted_hash::SaltedHash::generate_from(
                        "password",
                    ),
                    apns_token: None,
                    gcm_token: None,
                }))
            });

        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let result = service.get_account_id_by_identity(&identity).await.unwrap();
        assert_eq!(result, account_id);
    }

    #[tokio::test]
    async fn test_update_apns_token_success() {
        let mock_client = MockPrismApi::new();
        let mut mock_db = MockAccountDatabase::new();

        let account_id = Uuid::new_v4();
        let token = vec![1, 2, 3, 4, 5];

        mock_db
            .expect_update_apns_token()
            .once()
            .with(eq(account_id), eq(token.clone()))
            .returning(|_, _| Ok(()));

        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let result = service.update_apns_token(account_id, token).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_apns_token_account_not_found() {
        let mock_client = MockPrismApi::new();
        let mut mock_db = MockAccountDatabase::new();

        let account_id = Uuid::new_v4();
        let token = vec![1, 2, 3, 4, 5];

        mock_db
            .expect_update_apns_token()
            .once()
            .with(eq(account_id), eq(token.clone()))
            .returning(|id, _| Err(AccountDatabaseError::NotFound(id.to_string())));

        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let result = service.update_apns_token(account_id, token).await;

        assert!(matches!(result, Err(AccountServiceError::AccountNotFound)));
    }

    #[tokio::test]
    async fn test_update_apns_token_database_error() {
        let mock_client = MockPrismApi::new();
        let mut mock_db = MockAccountDatabase::new();

        let account_id = Uuid::new_v4();
        let token = vec![1, 2, 3, 4, 5];

        mock_db
            .expect_update_apns_token()
            .once()
            .with(eq(account_id), eq(token.clone()))
            .returning(|_, _| Err(AccountDatabaseError::OperationFailed));

        let service = AccountService::new(Arc::new(mock_client), Arc::new(mock_db));
        let result = service.update_apns_token(account_id, token).await;

        assert!(matches!(result, Err(AccountServiceError::DatabaseError(_))));
    }
}
