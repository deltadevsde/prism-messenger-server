use anyhow::Result;
use prism_client::{PrismHttpClient, SigningKey};
use std::{path::Path, sync::Arc};

use crate::{
    account::{auth::service::AuthService, service::AccountService},
    database::inmemory::InMemoryDatabase,
    initialization::InitializationService,
    keys::service::KeyService,
    messages::service::MessagingService,
    notifications::gateway::apns::ApnsNotificationGateway,
    registration::service::RegistrationService,
    settings::Settings,
};

pub struct AppContext {
    pub account_service: AccountService<PrismHttpClient, InMemoryDatabase>,
    pub auth_service: AuthService<InMemoryDatabase>,
    pub key_service: KeyService<PrismHttpClient, InMemoryDatabase>,
    pub messaging_service:
        MessagingService<InMemoryDatabase, InMemoryDatabase, ApnsNotificationGateway>,
    pub registration_service: RegistrationService<PrismHttpClient, InMemoryDatabase>,
    pub initialization_service: InitializationService<PrismHttpClient>,
}

impl AppContext {
    /// Creates and initializes the application context, including network setup
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let signing_key = Self::read_or_create_signing_key(&settings.prism.signing_key_path)?;

        // Initialize prism client
        let prism = PrismHttpClient::new(
            format!("http://{}:{}", settings.prism.host, settings.prism.port).as_str(),
        )?;
        let prism_arc = Arc::new(prism);

        let db = Arc::new(InMemoryDatabase::new());

        // Notifications
        let apns_gateway = ApnsNotificationGateway::from_file(
            &settings.apns.team_id,
            &settings.apns.key_id,
            &settings.apns.private_key_path,
            &settings.apns.bundle_id,
            !settings.development,
        )?;
        let apns_gateway_arc = Arc::new(apns_gateway);

        let account_service = AccountService::new(prism_arc.clone(), db.clone());
        let auth_service = AuthService::new(db.clone());
        let registration_service =
            RegistrationService::new(prism_arc.clone(), db.clone(), signing_key.clone());
        let key_service = KeyService::new(prism_arc.clone(), db.clone());
        let messaging_service = MessagingService::new(db.clone(), db.clone(), apns_gateway_arc);
        let initialization_service = InitializationService::new(prism_arc.clone(), signing_key);

        Ok(Self {
            account_service,
            auth_service,
            registration_service,
            key_service,
            messaging_service,
            initialization_service,
        })
    }

    /// Gets the signing key from file or creates a new one if the file doesn't exist
    fn read_or_create_signing_key(path: impl AsRef<Path>) -> Result<SigningKey> {
        if path.as_ref().exists() {
            Ok(SigningKey::from_pkcs8_pem_file(path)?)
        } else {
            let key = SigningKey::new_ed25519();
            key.to_pkcs8_pem_file(path)?;
            Ok(key)
        }
    }
}
