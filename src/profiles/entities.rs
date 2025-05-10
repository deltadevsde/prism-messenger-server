use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User profile information
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// UUID of the profile
    pub id: Uuid,
    /// UUID of the associated account
    pub account_id: Uuid,
    /// Display name of the user
    pub display_name: String,
    /// URL to the profile picture, if one exists
    pub profile_picture_url: Option<String>,
    /// Last update timestamp (epoch milliseconds)
    pub updated_at: u64,
}

impl Profile {
    /// Creates a new profile with an account ID and display name
    ///
    /// Other fields are initialized with default values:
    /// - id: randomly generated UUID
    /// - profile_picture_url: None
    /// - updated_at: current timestamp in milliseconds
    pub fn new(account_id: Uuid, display_name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4(),
            account_id,
            display_name,
            profile_picture_url: None,
            updated_at: now,
        }
    }
}

/// Actions that can be performed on a profile picture
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfilePictureAction {
    /// Do not change the profile picture (default)
    NoChange,
    /// Remove the existing profile picture without setting a new one
    Clear,
    /// Update the profile picture (will need to get an upload URL)
    Update,
}

impl Default for ProfilePictureAction {
    fn default() -> Self {
        Self::NoChange
    }
}

/// Request to update a profile
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    /// New display name (optional)
    pub display_name: Option<String>,
    /// Action to perform on the profile picture
    #[serde(default)]
    pub profile_picture_action: ProfilePictureAction,
}

/// Response for profile picture upload URL
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePictureUploadResponse {
    /// Pre-signed URL for uploading to S3
    pub upload_url: String,
    /// URL where the picture will be accessible after upload
    pub picture_url: String,
    /// Expiration time for the upload URL in seconds
    pub expires_in: u64,
}
