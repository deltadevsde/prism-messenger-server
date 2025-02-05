use anyhow::Result;
use prism_prover::prover::AccountResponse::*;
use prism_prover::Prover;
use std::sync::Arc;

pub struct AccountService {
    prover: Arc<Prover>,
}

impl AccountService {
    pub fn new(prover: Arc<Prover>) -> Self {
        Self { prover }
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let account_res = self.prover.clone().get_account(username).await?;

        let exists = match account_res {
            Found(_, _) => true,
            NotFound(_) => false,
        };

        Ok(exists)
    }
}
