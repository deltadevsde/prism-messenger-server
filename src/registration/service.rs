use prism_client::{binary::ToBinary as _, PrismApi, SigningKey, VerifyingKey};
use std::sync::Arc;

use crate::PRISM_MESSENGER_SERVICE_ID;

use super::{entities::RegistrationChallenge, error::RegistrationError};

pub struct RegistrationService<P>
where
    P: PrismApi,
{
    prism: Arc<P>,
    signing_key: SigningKey,
}

impl<P> RegistrationService<P>
where
    P: PrismApi,
{
    pub fn new(prism: Arc<P>, signing_key: SigningKey) -> Self {
        Self { prism, signing_key }
    }

    pub async fn request_registration(
        &self,
        username: String,
        user_identity_verifying_key: VerifyingKey,
    ) -> Result<RegistrationChallenge, RegistrationError> {
        let bytes_to_be_signed = self
            .prism
            .clone()
            .build_request()
            .create_account()
            .with_id(username)
            .with_key(user_identity_verifying_key)
            .for_service_with_id(PRISM_MESSENGER_SERVICE_ID.to_string())
            .meeting_signed_challenge(&self.signing_key)?
            .transaction()
            .encode_to_bytes()
            .map_err(|_| RegistrationError::ProcessingFailed)?;

        Ok(RegistrationChallenge(bytes_to_be_signed))
    }
}

#[cfg(test)]
mod tests {
    use crate::registration::service::RegistrationService;
    use anyhow::Result;
    use prism_client::{mock::MockPrismApi, SigningKey};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_account() -> Result<()> {
        let mock_prism = MockPrismApi::new();
        let signing_key = SigningKey::new_ed25519();

        let username = "test_user".to_string();
        let user_key = SigningKey::new_secp256r1().verifying_key();

        // Wrap the configured mock in an Arc and create the service
        let service = RegistrationService::new(Arc::new(mock_prism), signing_key.clone());

        // Execute the test
        service.request_registration(username, user_key).await?;

        Ok(())
    }
}
