use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{entities::PresenceStatus, error::PresenceError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub account_id: Uuid,
    pub status: PresenceStatus,
}

impl PresenceUpdate {
    pub fn new(account_id: Uuid, status: PresenceStatus) -> Self {
        Self { account_id, status }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PresenceGateway: Send + Sync {
    /// Send a presence update to interested parties
    async fn send_presence_update(
        &self,
        presence_update: &PresenceUpdate,
    ) -> Result<(), PresenceError>;

    async fn register_presence_handler<H>(&self, handler: H)
    where
        H: Fn(PresenceUpdate) + Send + Sync + 'static;
}
