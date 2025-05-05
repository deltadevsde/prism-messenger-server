use always_send::FutureExt;
use prism_client::{PrismApi, Signature, SignatureBundle, SigningKey, VerifyingKey};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, trace};

use super::{entities::RegistrationChallenge, error::RegistrationError};
use crate::{
    PRISM_MESSENGER_SERVICE_ID,
    account::{database::AccountDatabase, entities::Account},
    profiles::{database::ProfileDatabase, entities::Profile},
};

pub struct RegistrationService<P, AD, PD>
where
    P: PrismApi,
    AD: AccountDatabase,
    PD: ProfileDatabase,
{
    prism: Arc<P>,
    signing_key: SigningKey,
    account_database: Arc<AD>,
    profile_database: Arc<PD>,
}

impl<P, AD, PD> RegistrationService<P, AD, PD>
where
    P: PrismApi,
    AD: AccountDatabase,
    PD: ProfileDatabase,
{
    pub fn new(
        prism: Arc<P>,
        signing_key: SigningKey,
        account_database: Arc<AD>,
        profile_database: Arc<PD>,
    ) -> Self {
        Self {
            prism,
            signing_key,
            account_database,
            profile_database,
        }
    }

    #[instrument(skip_all, fields(username = username, key = %user_identity_verifying_key))]
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

    #[instrument(skip_all, fields(username = username, key = %user_identity_verifying_key, signature = %registration_signature))]
    pub async fn finalize_registration(
        &self,
        username: String,
        user_identity_verifying_key: VerifyingKey,
        registration_signature: Signature,
        auth_password: &str,
        apns_token: Option<Vec<u8>>,
        gcm_token: Option<Vec<u8>>,
    ) -> Result<Account, RegistrationError> {
        debug!("Starting registration finalization");

        if apns_token.is_none() && gcm_token.is_none() {
            error!("Missing push token");
            return Err(RegistrationError::MissingPushToken);
        }

        let signature_bundle =
            SignatureBundle::new(user_identity_verifying_key.clone(), registration_signature);

        trace!("Sending request to prism API");
        self.prism
            .clone()
            .build_request()
            .create_account()
            .with_id(username.clone())
            .with_key(user_identity_verifying_key)
            .for_service_with_id(PRISM_MESSENGER_SERVICE_ID.to_string())
            .meeting_signed_challenge(&self.signing_key)?
            .with_external_signature(signature_bundle)
            .send()
            // working around rust #100031 with always_send()
            .always_send()
            .await?;
        info!(username, "Successfully created account on prism");

        let account = Account::new(username.clone(), auth_password, apns_token, gcm_token);

        trace!(?account, "Saving created account in local database");
        self.account_database
            .clone()
            .upsert_account(account.clone())
            .await?;
        info!("Successfully saved account");

        let profile = Profile::new(username);

        trace!(?profile, "Saving account profile in local database");
        self.profile_database
            .clone()
            .upsert_profile(profile)
            .await?;

        info!("Registration completed successfully");
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use prism_client::{
        Account, SigningKey,
        mock::{MockPrismApi, MockPrismPendingTransaction},
    };
    use std::sync::Arc;

    use crate::{
        account::database::MockAccountDatabase,
        profiles::database::MockProfileDatabase,
        registration::{error::RegistrationError, service::RegistrationService},
    };

    #[tokio::test]
    async fn test_create_account() -> Result<()> {
        let mut mock_prism = MockPrismApi::new();
        let mut mock_account_db = MockAccountDatabase::new();
        let mut mock_profile_db = MockProfileDatabase::new();
        let service_signing_key = SigningKey::new_ed25519();

        let username = "test_user".to_string();
        let user_identity_signing_key = SigningKey::new_secp256r1();
        let user_identity_verifying_key = user_identity_signing_key.verifying_key();

        mock_account_db
            .expect_upsert_account()
            .times(1)
            .returning(|_| Ok(()));

        mock_profile_db
            .expect_upsert_profile()
            .times(1)
            .returning(|_| Ok(()));

        mock_prism
            .expect_post_transaction()
            .times(1)
            .returning(|_| {
                Ok(MockPrismPendingTransaction::with_result(Ok(
                    Account::default(),
                )))
            });

        // Wrap the configured mocks in Arc and create the service
        let service = RegistrationService::new(
            Arc::new(mock_prism),
            service_signing_key.clone(),
            Arc::new(mock_account_db),
            Arc::new(mock_profile_db),
        );

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
            .finalize_registration(
                username,
                user_identity_verifying_key,
                challenge_signature,
                "auth_password",
                Some(b"apns_token".to_vec()),
                Some(b"gcm_token".to_vec()),
            )
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_push_tokens_validation() -> Result<()> {
        // Setup test components
        let mut mock_prism = MockPrismApi::new();
        let mut mock_account_db = MockAccountDatabase::new();
        let mut mock_profile_db = MockProfileDatabase::new();
        let service_signing_key = SigningKey::new_ed25519();

        let username = "push_token_test_user".to_string();
        let user_signing_key = SigningKey::new_secp256r1();
        let user_verifying_key = user_signing_key.verifying_key();

        mock_prism.expect_post_transaction().returning(|_| {
            Ok(MockPrismPendingTransaction::with_result(Ok(
                Account::default(),
            )))
        });

        mock_account_db
            .expect_upsert_account()
            .returning(|_| Ok(()));

        mock_profile_db
            .expect_upsert_profile()
            .returning(|_| Ok(()));

        // Create the service
        let service = RegistrationService::new(
            Arc::new(mock_prism),
            service_signing_key.clone(),
            Arc::new(mock_account_db),
            Arc::new(mock_profile_db),
        );

        // Request registration to get challenge
        let registration_challenge = service
            .request_registration(username.clone(), user_verifying_key.clone())
            .await?;

        // Sign the challenge
        let challenge_signature = user_signing_key.sign(registration_challenge).unwrap();

        // Case 1: Test with no push tokens (should fail)
        let result = service
            .finalize_registration(
                username.clone(),
                user_verifying_key.clone(),
                challenge_signature.clone(),
                "auth_password",
                None,
                None,
            )
            .await;

        assert!(
            result.is_err(),
            "Registration should fail with no push tokens"
        );
        assert!(
            matches!(result.unwrap_err(), RegistrationError::MissingPushToken),
            "Expected MissingPushToken error"
        );

        // Case 2: Test with only APNS token
        let result = service
            .finalize_registration(
                username.clone(),
                user_verifying_key.clone(),
                challenge_signature.clone(),
                "auth_password",
                Some(b"apns_token".to_vec()),
                None,
            )
            .await;

        assert!(
            result.is_ok(),
            "Registration should succeed with only APNS token"
        );

        // Case 3: Test with only GCM token
        let result = service
            .finalize_registration(
                username.clone(),
                user_verifying_key.clone(),
                challenge_signature.clone(),
                "auth_password",
                None,
                Some(b"gcm_token".to_vec()),
            )
            .await;

        assert!(
            result.is_ok(),
            "Registration should succeed with only GCM token"
        );

        // Case 4: Test with both tokens
        let result = service
            .finalize_registration(
                username,
                user_verifying_key,
                challenge_signature,
                "auth_password",
                Some(b"apns_token".to_vec()),
                Some(b"gcm_token".to_vec()),
            )
            .await;

        assert!(
            result.is_ok(),
            "Registration should succeed with both tokens"
        );

        Ok(())
    }
}
