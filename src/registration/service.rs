use prism_client::{
    PendingTransaction, PrismApi, Signature, SignatureBundle, SigningKey, VerifyingKey,
};
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
            .signing_payload()?;

        Ok(RegistrationChallenge(bytes_to_be_signed))
    }

    pub async fn finalize_registration(
        &self,
        username: String,
        user_identity_verifying_key: VerifyingKey,
        registration_signature: Signature,
    ) -> Result<(), RegistrationError> {
        let signature_bundle =
            SignatureBundle::new(user_identity_verifying_key.clone(), registration_signature);

        self.prism
            .clone()
            .build_request()
            .create_account()
            .with_id(username)
            .with_key(user_identity_verifying_key)
            .for_service_with_id(PRISM_MESSENGER_SERVICE_ID.to_string())
            .meeting_signed_challenge(&self.signing_key)?
            .with_external_signature(signature_bundle)
            .send()
            .await
            .map_err(|_| RegistrationError::ProcessingFailed)?
            .wait()
            .await
            .map_err(|_| RegistrationError::ProcessingFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::registration::service::RegistrationService;
    use anyhow::Result;
    use prism_client::{
        Account, SigningKey,
        mock::{MockPrismApi, MockPrismPendingTransaction},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_account() -> Result<()> {
        let mut mock_prism = MockPrismApi::new();
        let service_signing_key = SigningKey::new_ed25519();

        let username = "test_user".to_string();
        let user_identity_signing_key = SigningKey::new_secp256r1();
        let user_identity_verifying_key = user_identity_signing_key.verifying_key();

        mock_prism
            .expect_post_transaction()
            .times(1)
            .returning(|_| {
                Ok(MockPrismPendingTransaction::with_result(Ok(
                    Account::default(),
                )))
            });

        // Wrap the configured mock in an Arc and create the service
        let service = RegistrationService::new(Arc::new(mock_prism), service_signing_key.clone());

        // Simulate a client requesting registration
        let registration_challenge = service
            .request_registration(username.clone(), user_identity_verifying_key.clone())
            .await?;

        // Simulate a client signing the challenge
        let challenge_signature = user_identity_signing_key
            .sign(registration_challenge)
            .unwrap();

        // Simulate a client finalizing the registration
        service
            .finalize_registration(username, user_identity_verifying_key, challenge_signature)
            .await?;

        Ok(())
    }
}
