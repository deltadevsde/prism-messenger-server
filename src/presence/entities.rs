use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Offline,
}

impl From<bool> for PresenceStatus {
    fn from(is_present: bool) -> Self {
        if is_present {
            PresenceStatus::Online
        } else {
            PresenceStatus::Offline
        }
    }
}
