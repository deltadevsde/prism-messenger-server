use anyhow::Result;
use prism_common::{digest::Digest, operation::ServiceChallengeInput};
use prism_keys::{SigningKey, VerifyingKey};
use std::sync::Arc;

use crate::common::prism_client::PrismClient;
use crate::PRISM_MESSENGER_SERVICE_ID;

pub struct RegistrationService<C: PrismClient> {
    prism: Arc<C>,
    signing_key: SigningKey,
}

impl<C: PrismClient> RegistrationService<C> {
    pub fn new(prism: Arc<C>, signing_key: SigningKey) -> Self {
        Self { prism, signing_key }
    }

    pub async fn create_account(&self, username: String, key: VerifyingKey) -> Result<()> {
        let hash = Digest::hash_items(&[
            username.as_bytes(),
            PRISM_MESSENGER_SERVICE_ID.as_bytes(),
            &key.to_bytes(),
        ]);
        let signature = self.signing_key.sign(&hash.to_bytes());

        self.prism
            .create_account(
                username,
                PRISM_MESSENGER_SERVICE_ID.to_string(),
                ServiceChallengeInput::Signed(signature),
                key,
                &self.signing_key,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::common::prism_client::MockPrismClient;
    use crate::registration::service::RegistrationService;
    use crate::PRISM_MESSENGER_SERVICE_ID;
    use anyhow::Result;
    use mockall::predicate::eq;
    use prism_common::digest::Digest;
    use prism_common::operation::ServiceChallengeInput;
    use prism_keys::SigningKey;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_account() -> Result<()> {
        let mut mock_client = MockPrismClient::new();
        let signing_key = SigningKey::new_ed25519();

        let username = "test_user".to_string();
        let user_key = SigningKey::new_secp256r1().verifying_key();

        let hash = Digest::hash_items(&[
            username.as_bytes(),
            PRISM_MESSENGER_SERVICE_ID.as_bytes(),
            &user_key.to_bytes(),
        ]);
        let signature = signing_key.sign(&hash.to_bytes());

        mock_client
            .expect_create_account()
            .with(
                eq(username.clone()),
                eq(PRISM_MESSENGER_SERVICE_ID.to_string()),
                eq(ServiceChallengeInput::Signed(signature.clone())),
                eq(user_key.clone()),
                eq(signing_key.clone()),
            )
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        // Wrap the configured mock in an Arc and create the service
        let service = RegistrationService::new(Arc::new(mock_client), signing_key.clone());

        // Execute the test
        service.create_account(username, user_key).await?;

        Ok(())
    }
}
