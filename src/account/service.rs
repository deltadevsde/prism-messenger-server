use anyhow::Result;
use std::sync::Arc;

use crate::common::prism_client::PrismClient;

pub struct AccountService<C: PrismClient> {
    client: Arc<C>,
}

impl<C: PrismClient> AccountService<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let account_res = self.client.clone().get_account(username).await?;

        Ok(account_res.account.is_some())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        account::service::AccountService,
        common::prism_client::{AccountResponse, MockPrismClient},
    };
    use mockall::predicate::eq;
    use prism_common::account::Account;
    use prism_tree::proofs::HashedMerkleProof;

    #[tokio::test]
    async fn test_username_exists_returns_true_when_found() {
        let mut mock_client = MockPrismClient::new();
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

        let service = AccountService::new(Arc::new(mock_client));
        let exists = service.username_exists("test").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_username_exists_returns_false_when_not_found() {
        let mut mock_client = MockPrismClient::new();
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
        let service = AccountService::new(Arc::new(mock_client));
        let exists = service.username_exists("test").await.unwrap();
        assert!(!exists);
    }
}
