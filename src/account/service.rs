use anyhow::Result;
use prism_client::PrismApi;
use std::sync::Arc;

pub struct AccountService<P: PrismApi> {
    prism: Arc<P>,
}

impl<P: PrismApi> AccountService<P> {
    pub fn new(prism: Arc<P>) -> Self {
        Self { prism }
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let account_res = self.prism.clone().get_account(username).await?;

        Ok(account_res.account.is_some())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

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

        let service = AccountService::new(Arc::new(mock_client));
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
        let service = AccountService::new(Arc::new(mock_client));
        let exists = service.username_exists("test").await.unwrap();
        assert!(!exists);
    }
}
