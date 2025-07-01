use async_trait::async_trait;
use uuid::Uuid;

use super::error::PresenceError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PresenceDatabase: Send + Sync {
    async fn is_present(&self, account_id: &Uuid) -> Result<bool, PresenceError>;
}
