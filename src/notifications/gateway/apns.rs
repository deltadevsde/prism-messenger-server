use a2::{
    Client, ClientConfig, DefaultNotificationBuilder, Endpoint, Error as A2Error,
    NotificationBuilder, NotificationOptions, Priority,
};
use async_trait::async_trait;
use std::{fs::File, io::Read, path::Path};
use tracing::instrument;

use super::{NotificationError, NotificationGateway};

/// APNS (Apple Push Notification Service) Gateway implementation
pub struct ApnsNotificationGateway {
    client: Client,
    bundle_id: String,
}

impl ApnsNotificationGateway {
    /// Create a new APNS gateway instance from raw private key data
    pub fn new(
        team_id: &str,
        key_id: &str,
        private_key: &[u8],
        bundle_id: &str,
        is_production: bool,
    ) -> Result<Self, NotificationError> {
        let endpoint = if is_production {
            Endpoint::Production
        } else {
            Endpoint::Sandbox
        };

        let config = ClientConfig::new(endpoint);
        let client = Client::token(private_key, key_id, team_id, config)
            .map_err(|err| NotificationError::InitializationFailed(err.to_string()))?;

        Ok(ApnsNotificationGateway {
            client,
            bundle_id: bundle_id.to_string(),
        })
    }

    /// Create a new APNS gateway instance from a private key file
    pub fn from_file<P: AsRef<Path>>(
        team_id: &str,
        key_id: &str,
        private_key_path: P,
        bundle_id: &str,
        is_production: bool,
    ) -> Result<Self, NotificationError> {
        let mut private_key = Vec::new();
        File::open(private_key_path)
            .map_err(|err| NotificationError::InitializationFailed(err.to_string()))?
            .read_to_end(&mut private_key)
            .map_err(|err| NotificationError::InitializationFailed(err.to_string()))?;

        Self::new(team_id, key_id, &private_key, bundle_id, is_production)
    }
}

#[async_trait]
impl NotificationGateway for ApnsNotificationGateway {
    #[instrument(skip_all)]
    async fn send_silent_notification(&self, device_token: &[u8]) -> Result<(), NotificationError> {
        let options = NotificationOptions {
            apns_topic: Some(&self.bundle_id),
            apns_priority: Some(Priority::Normal),
            ..Default::default()
        };
        let device_token_hex = hex::encode(device_token);

        let payload = DefaultNotificationBuilder::new().build(&device_token_hex, options);

        let response = self.client.send(payload).await?;
        tracing::debug!("APNS response: {:?}", response);
        Ok(())
    }
}

impl From<A2Error> for NotificationError {
    fn from(err: A2Error) -> Self {
        NotificationError::SendFailure(format!("APNS error: {:?}", err))
    }
}
