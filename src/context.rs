use anyhow::{Result, bail};
use prism_client::{PrismHttpClient, SigningKey};
use std::{path::Path, sync::Arc};

use crate::{
    account::{auth::service::AuthService, service::AccountService},
    database::{
        inmemory::InMemoryDatabase, pool::create_sqlite_pool, s3::S3Storage, sqlite::SqliteDatabase,
    },
    initialization::InitializationService,
    keys::service::KeyService,
    messages::service::MessagingService,
    notifications::gateway::apns::ApnsNotificationGateway,
    profiles::service::ProfileService,
    registration::{phone_number::PhoneRegistrationService, username::UsernameRegistrationService},
    settings::{AssetsDatabaseSettings, CoreDatabaseSettings, EphemeralDatabaseSettings, Settings},
};

pub struct AppContext {
    pub account_service: AccountService<PrismHttpClient, SqliteDatabase>,
    pub auth_service: AuthService<SqliteDatabase>,
    pub key_service: KeyService<PrismHttpClient, SqliteDatabase>,
    pub messaging_service:
        MessagingService<SqliteDatabase, InMemoryDatabase, ApnsNotificationGateway>,
    pub profile_service: ProfileService<SqliteDatabase, S3Storage>,
    pub username_registration_service:
        UsernameRegistrationService<PrismHttpClient, SqliteDatabase, SqliteDatabase>,
    pub phone_registration_service:
        PhoneRegistrationService<PrismHttpClient, SqliteDatabase, SqliteDatabase>,
    pub initialization_service: InitializationService<PrismHttpClient>,
}

impl AppContext {
    /// Creates and initializes the application context, including network setup
    pub async fn from_settings(settings: &Settings) -> Result<Self> {
        let signing_key = Self::read_or_create_signing_key(&settings.prism.signing_key_path)?;

        // Initialize prism client
        let prism = PrismHttpClient::new(
            format!("http://{}:{}", settings.prism.host, settings.prism.port).as_str(),
        )?;
        let prism_arc = Arc::new(prism);

        // Notifications
        let apns_gateway = ApnsNotificationGateway::from_file(
            &settings.apns.team_id,
            &settings.apns.key_id,
            &settings.apns.private_key_path,
            &settings.apns.bundle_id,
            !settings.development,
        )?;
        let apns_gateway_arc = Arc::new(apns_gateway);

        // Core Database
        let CoreDatabaseSettings::Sqlite { path: core_db_path } = &settings.database.core else {
            bail!("Unsupported core database type. Only sqlite supported for now");
        };

        let db_pool = create_sqlite_pool(core_db_path).await?;
        let core_db = Arc::new(SqliteDatabase::new(db_pool));
        core_db.init().await?;

        // Ephemeral Database
        let EphemeralDatabaseSettings::InMemory = &settings.database.ephemeral else {
            bail!("Unsupported ephemeral database type. Only in-memory supported for now");
        };
        let ephemeral_db = Arc::new(InMemoryDatabase::new());

        // Assets Database
        let assets_db = match &settings.database.assets {
            AssetsDatabaseSettings::S3 {
                bucket,
                region,
                access_key,
                secret_key,
                endpoint,
            } => Arc::new(
                S3Storage::new(
                    bucket.clone(),
                    region.clone(),
                    access_key.clone(),
                    secret_key.clone(),
                    endpoint.clone(),
                )
                .await?,
            ),
            _ => bail!("Unsupported assets database type. Only s3 supported for now"),
        };

        // Services
        let account_service = AccountService::new(prism_arc.clone(), core_db.clone());
        let auth_service = AuthService::new(core_db.clone());
        let username_registration_service = UsernameRegistrationService::new(
            prism_arc.clone(),
            signing_key.clone(),
            core_db.clone(),
            core_db.clone(),
        );
        let phone_registration_service = PhoneRegistrationService::new(
            prism_arc.clone(),
            core_db.clone(),
            core_db.clone(),
            signing_key.clone(),
            settings.twilio.account_sid.clone(),
            settings.twilio.auth_token.clone(),
            settings.twilio.verify_service_sid.clone(),
        );
        let key_service = KeyService::new(prism_arc.clone(), core_db.clone());
        let messaging_service =
            MessagingService::new(core_db.clone(), ephemeral_db.clone(), apns_gateway_arc);
        let profile_service = ProfileService::new(core_db.clone(), assets_db.clone());
        let initialization_service = InitializationService::new(prism_arc.clone(), signing_key);

        Ok(Self {
            account_service,
            auth_service,
            username_registration_service,
            phone_registration_service,
            key_service,
            messaging_service,
            profile_service,
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
