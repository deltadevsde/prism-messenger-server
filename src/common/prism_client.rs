use anyhow::Result;
use mockall::automock;
use prism_common::{
    account::Account,
    operation::{Operation, ServiceChallengeInput},
};
use prism_keys::{SigningKey, VerifyingKey};
use prism_prover::Prover;

#[cfg_attr(test, automock)]
pub trait PrismClient {
    async fn create_account(
        &self,
        username: String,
        service_id: String,
        challenge: ServiceChallengeInput,
        key: VerifyingKey,
        signing_key: &SigningKey,
    ) -> Result<()>;
}

impl PrismClient for Prover {
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
}
