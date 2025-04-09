use async_trait::async_trait;
use tracing::info;

use super::{NotificationError, NotificationGateway};

pub struct DummyNotificationGateway;

#[async_trait]
impl NotificationGateway for DummyNotificationGateway {
    async fn send_silent_notification(&self, device_token: &[u8]) -> Result<(), NotificationError> {
        let device_token_hex = hex::encode(device_token);
        info!("Notification to {}", device_token_hex);
        Ok(())
    }
}
