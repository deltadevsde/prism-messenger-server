use anyhow::{Result, bail};
use prism_client::{PendingTransaction, PrismApi, PrismHttpClient, SigningKey};
use std::{path::Path, sync::Arc};

use crate::{
    MESSAGE_SENDER_POLL_INTERVAL, PRISM_MESSENGER_SERVICE_ID,
    account::{auth::service::AuthService, service::AccountService},
    database::{
        inmemory::InMemoryDatabase, pool::create_sqlite_pool, s3::S3Storage, sqlite::SqliteDatabase,
    },
    keys::service::KeyService,
    messages::{messaging_service::MessagingService, sender_service::MessageSenderService},
    notifications::{gateway::apns::ApnsNotificationGateway, service::NotificationService},
    presence::{service::PresenceService, update_service::PresenceUpdateService},
    profiles::service::ProfileService,
    registration::service::RegistrationService,
    settings::{AssetsDatabaseSettings, CoreDatabaseSettings, EphemeralDatabaseSettings, Settings},
    websocket::center::WebSocketCenter,
};

pub struct AppContext {
    pub account_service: AccountService<PrismHttpClient, SqliteDatabase>,
    pub auth_service: AuthService<SqliteDatabase>,
    pub key_service: KeyService<PrismHttpClient, SqliteDatabase>,
    pub messaging_service: MessagingService<
        InMemoryDatabase,
        WebSocketCenter,
        SqliteDatabase,
        ApnsNotificationGateway,
    >,
    pub presence_service: PresenceService<WebSocketCenter>,
    pub profile_service: ProfileService<SqliteDatabase, S3Storage>,
    pub registration_service: RegistrationService<PrismHttpClient, SqliteDatabase, SqliteDatabase>,
    pub websocket_center: Arc<WebSocketCenter>,
}

/// Creates and initializes the application context, including network setup
pub async fn start_application(settings: &Settings) -> Result<AppContext> {
    let signing_key = read_or_create_signing_key(&settings.prism.signing_key_path)?;

    // Initialize prism client
    let prism = PrismHttpClient::new(
        format!("http://{}:{}", settings.prism.host, settings.prism.port).as_str(),
    )?;
    let prism_arc = Arc::new(prism);

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

    // Notifications
    let apns_gateway = ApnsNotificationGateway::from_file(
        &settings.apns.team_id,
        &settings.apns.key_id,
        &settings.apns.private_key_path,
        &settings.apns.bundle_id,
        !settings.development,
    )?;
    let apns_gateway_arc = Arc::new(apns_gateway);

    // Services
    let account_service = AccountService::new(prism_arc.clone(), core_db.clone());
    let auth_service = AuthService::new(core_db.clone());

    let notification_service = NotificationService::new(core_db.clone(), apns_gateway_arc.clone());
    let notification_service_arc = Arc::new(notification_service);

    let registration_service = RegistrationService::new(
        prism_arc.clone(),
        signing_key.clone(),
        core_db.clone(),
        core_db.clone(),
    );
    let key_service = KeyService::new(prism_arc.clone(), core_db.clone());

    let websocket_center = WebSocketCenter::new();
    let websocket_center_arc = Arc::new(websocket_center);

    // Create messaging service with WebSocket manager
    let messaging_service = MessagingService::new(
        ephemeral_db.clone(),
        websocket_center_arc.clone(),
        notification_service_arc.clone(),
    );

    let message_sender_service = MessageSenderService::new(
        ephemeral_db.clone(),
        websocket_center_arc.clone(),
        MESSAGE_SENDER_POLL_INTERVAL,
    );
    let message_sender_service_arc = Arc::new(message_sender_service);

    let presence_service = PresenceService::new(websocket_center_arc.clone());
    let presence_update_service = PresenceUpdateService::new(websocket_center_arc.clone());
    let presence_update_service_arc = Arc::new(presence_update_service);

    let profile_service = ProfileService::new(core_db.clone(), assets_db.clone());

    message_sender_service_arc.spawn_message_sender();
    presence_update_service_arc
        .clone()
        .handle_presence_updates()
        .await;

    register_messenger_service(prism_arc.clone(), &signing_key).await?;

    Ok(AppContext {
        account_service,
        auth_service,
        registration_service,
        key_service,
        messaging_service,
        presence_service,
        profile_service,
        websocket_center: websocket_center_arc,
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

async fn register_messenger_service<P: PrismApi>(
    prism: Arc<P>,
    signing_key: &SigningKey,
) -> Result<()> {
    tracing::info!("Initializing messenger service");
    let service_acc_response = prism.get_account(PRISM_MESSENGER_SERVICE_ID).await?;

    if service_acc_response.account.is_some() {
        tracing::info!("Messenger service already registered in prism");
        return Ok(());
    }

    prism
        .register_service(
            PRISM_MESSENGER_SERVICE_ID.to_string(),
            signing_key.verifying_key(),
            signing_key,
        )
        .await?
        .wait()
        .await?;

    Ok(())
}
