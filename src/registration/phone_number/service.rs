use always_send::FutureExt;
use prism_client::{PrismApi, Signature, SignatureBundle, SigningKey, VerifyingKey};
use sha3::{Digest, Sha3_256};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};
use phonenumber::{parse, Mode};

use crate::registration::{entities::RegistrationChallenge, error::RegistrationError};
use crate::{
    PRISM_PHONE_SERVICE_ID,
    account::{database::AccountDatabase, entities::Account},
    profiles::{database::ProfileDatabase, entities::Profile},
};

pub struct PhoneRegistrationService<P, AD, PD>
where
    P: PrismApi,
    AD: AccountDatabase,
    PD: ProfileDatabase,
{
    prism: Arc<P>,
    signing_key: SigningKey,
    account_database: Arc<AD>,
    profile_database: Arc<PD>,
    pending_registrations: Arc<RwLock<HashSet<String>>>,
    twilio_account_sid: String,
    twilio_auth_token: String,
    twilio_verify_service_sid: String,
}

impl<P, AD, PD> PhoneRegistrationService<P, AD, PD>
where
    P: PrismApi,
    AD: AccountDatabase,
    PD: ProfileDatabase,
{
    pub fn new(
        prism: Arc<P>,
        account_database: Arc<AD>,
        profile_database: Arc<PD>,
        signing_key: SigningKey,
        twilio_account_sid: String,
        twilio_auth_token: String,
        twilio_verify_service_sid: String,
    ) -> Self {
        if twilio_account_sid.is_empty() || twilio_auth_token.is_empty() || twilio_verify_service_sid.is_empty() {
            tracing::warn!("Twilio credentials are empty - phone registration will not work");
        }
        
        Self {
            prism,
            account_database,
            profile_database,
            signing_key,
            pending_registrations: Arc::new(RwLock::new(HashSet::new())),
            twilio_account_sid,
            twilio_auth_token,
            twilio_verify_service_sid,
        }
    }

    fn validate_phone_number(&self, phone_number: &str) -> Result<String, RegistrationError> {
        let parsed = parse(None, phone_number)
            .map_err(|_| RegistrationError::InvalidPhoneNumber)?;
        
        if !parsed.is_valid() {
            return Err(RegistrationError::InvalidPhoneNumber);
        }

        Ok(parsed.format().mode(Mode::E164).to_string())
    }

    fn generate_prism_identifier(&self, phone_number: &str) -> String {
        let input = format!("{}:{}", PRISM_PHONE_SERVICE_ID, phone_number);
        let mut hasher = Sha3_256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    #[instrument(skip_all, fields(phone_number = phone_number))]
    pub async fn request_phone_registration(&self, phone_number: String) -> Result<(), RegistrationError> {
        let normalized_phone = self.validate_phone_number(&phone_number)?;
        
        debug!("Sending OTP via Twilio - Service SID: {}", self.twilio_verify_service_sid);
        
        // Send OTP via Twilio Verify
        let client = reqwest::Client::new();
        let url = format!(
            "https://verify.twilio.com/v2/Services/{}/Verifications",
            self.twilio_verify_service_sid
        );

        let response = client
            .post(&url)
            .basic_auth(&self.twilio_account_sid, Some(&self.twilio_auth_token))
            .form(&[
                ("To", normalized_phone.as_str()),
                ("Channel", "sms"),
            ])
            .send()
            .await
            .map_err(|e| RegistrationError::TwilioError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Twilio API error - Status: {}, Response: {}", status, error_text);
            return Err(RegistrationError::TwilioError(format!("Failed to send OTP ({}): {}", status, error_text)));
        }

        // Store pending registration
        let mut pending_registrations = self.pending_registrations.write().await;
        pending_registrations.insert(normalized_phone);

        info!("OTP sent successfully to phone number");
        Ok(())
    }

    #[instrument(skip_all, fields(phone_number = phone_number, key = %user_identity_verifying_key))]
    pub async fn verify_phone_registration(
        &self,
        phone_number: String,
        otp_code: String,
        user_identity_verifying_key: VerifyingKey,
    ) -> Result<RegistrationChallenge, RegistrationError> {
        let normalized_phone = self.validate_phone_number(&phone_number)?;

        // Check if we have a pending registration
        {
            let pending_registrations = self.pending_registrations.read().await;
            if !pending_registrations.contains(&normalized_phone) {
                return Err(RegistrationError::PhoneSessionNotFound);
            }
        }

        // Verify OTP with Twilio
        debug!("Verifying OTP via Twilio - Service SID: {}", self.twilio_verify_service_sid);
        
        let client = reqwest::Client::new();
        let url = format!(
            "https://verify.twilio.com/v2/Services/{}/VerificationCheck",
            self.twilio_verify_service_sid
        );

        let response = client
            .post(&url)
            .basic_auth(&self.twilio_account_sid, Some(&self.twilio_auth_token))
            .form(&[
                ("To", normalized_phone.as_str()),
                ("Code", otp_code.as_str()),
            ])
            .send()
            .await
            .map_err(|e| RegistrationError::TwilioError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Twilio verification API error - Status: {}, Response: {}", status, error_text);
            return Err(RegistrationError::OtpVerificationFailed);
        }

        // Parse response to check status
        let verification_result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RegistrationError::TwilioError(e.to_string()))?;

        if verification_result["status"] != "approved" {
            return Err(RegistrationError::OtpVerificationFailed);
        }

        // Generate prism identifier
        let prism_identifier = self.generate_prism_identifier(&normalized_phone);

        // Create signed challenge
        let bytes_to_be_signed = self
            .prism
            .clone()
            .build_request()
            .create_account()
            .with_id(prism_identifier)
            .with_key(user_identity_verifying_key)
            .for_service_with_id(PRISM_PHONE_SERVICE_ID.to_string())
            .meeting_signed_challenge(&self.signing_key)?
            .transaction()
            .signing_payload()?;

        // Remove from pending registrations
        let mut pending_registrations = self.pending_registrations.write().await;
        pending_registrations.remove(&normalized_phone);

        info!("Phone verification successful, returning challenge");
        Ok(RegistrationChallenge(bytes_to_be_signed))
    }

    #[instrument(skip_all, fields(phone_number = phone_number, key = %user_identity_verifying_key, signature = %registration_signature))]
    pub async fn finalize_phone_registration(
        &self,
        phone_number: String,
        user_identity_verifying_key: VerifyingKey,
        registration_signature: Signature,
        auth_password: &str,
        apns_token: Option<Vec<u8>>,
        gcm_token: Option<Vec<u8>>,
    ) -> Result<Account, RegistrationError> {
        debug!("Starting phone registration finalization");

        if apns_token.is_none() && gcm_token.is_none() {
            error!("Missing push token");
            return Err(RegistrationError::MissingPushToken);
        }

        let normalized_phone = self.validate_phone_number(&phone_number)?;
        let prism_identifier = self.generate_prism_identifier(&normalized_phone);

        let signature_bundle =
            SignatureBundle::new(user_identity_verifying_key.clone(), registration_signature);

        debug!("Sending request to prism API");
        self.prism
            .clone()
            .build_request()
            .create_account()
            .with_id(prism_identifier.clone())
            .with_key(user_identity_verifying_key)
            .for_service_with_id(PRISM_PHONE_SERVICE_ID.to_string())
            .meeting_signed_challenge(&self.signing_key)?
            .with_external_signature(signature_bundle)
            .send()
            .always_send()
            .await?;

        info!(phone_number = normalized_phone, "Successfully created account on prism");
        let account = Account::new(auth_password, apns_token, gcm_token);

        debug!(?account, "Saving created account in local database");
        self.account_database
            .clone()
            .upsert_account(account.clone())
            .await?;
        info!("Successfully saved account");

        // For every new account, a profile is created using the phone number as username
        let profile = Profile::new(account.id, normalized_phone);

        debug!(?profile, "Saving account profile in local database");
        self.profile_database
            .clone()
            .upsert_profile(profile)
            .await?;

        info!("Phone registration completed successfully");
        Ok(account)
    }
}