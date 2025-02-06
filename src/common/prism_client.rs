use anyhow::Result;
use mockall::automock;
use prism_common::{
    account::Account,
    operation::{Operation, ServiceChallenge, ServiceChallengeInput},
};
use prism_keys::{SigningKey, VerifyingKey};
use prism_prover::{prover::AccountResponse::*, Prover};
use prism_tree::proofs::HashedMerkleProof;

pub struct AccountResponse {
    pub account: Option<Account>,
    pub proof: HashedMerkleProof,
}

#[cfg_attr(test, automock)]
pub trait PrismClient {
    async fn register_service(
        &self,
        id: String,
        challenge: ServiceChallenge,
        key: VerifyingKey,
        signing_key: &SigningKey,
    ) -> Result<()>;

    async fn create_account(
        &self,
        username: String,
        service_id: String,
        challenge: ServiceChallengeInput,
        key: VerifyingKey,
        signing_key: &SigningKey,
    ) -> Result<()>;

    async fn get_account(&self, username: &str) -> Result<AccountResponse>;
}

impl PrismClient for Prover {
    async fn register_service(
        &self,
        id: String,
        challenge: ServiceChallenge,
        key: VerifyingKey,
        signing_key: &SigningKey,
    ) -> Result<()> {
        let op = Operation::RegisterService {
            id: id.to_string(),
            creation_gate: challenge,
            key,
        };

        let tx = Account::default().prepare_transaction(id, op, signing_key)?;
        self.validate_and_queue_update(tx).await
    }

    async fn create_account(
        &self,
        id: String,
        service_id: String,
        challenge: ServiceChallengeInput,
        key: VerifyingKey,
        signing_key: &SigningKey,
    ) -> Result<()> {
        let op = Operation::CreateAccount {
            id: id.clone(),
            service_id,
            challenge,
            key,
        };

        let tx = Account::default().prepare_transaction(id, op, signing_key)?;
        self.validate_and_queue_update(tx).await
    }

    async fn get_account(&self, username: &str) -> Result<AccountResponse> {
        let prism_account_res = Prover::get_account(self, username).await?;

        let account_res = match prism_account_res {
            Found(account, merkle_proof) => AccountResponse {
                account: Some(*account),
                proof: merkle_proof.hashed(),
            },
            NotFound(merkle_proof) => AccountResponse {
                account: None,
                proof: merkle_proof.hashed(),
            },
        };

        Ok(account_res)
    }
}
