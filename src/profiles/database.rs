use async_trait::async_trait;
use uuid::Uuid;

use super::entities::Profile;
use super::error::ProfileError;

/// Database operations for user profiles
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ProfileDatabase {
    /// Get a user profile by ID
    async fn get_profile_by_id(&self, id: Uuid) -> Result<Option<Profile>, ProfileError>;

    /// Get a user profile by username
    async fn get_profile_by_username(
        &self,
        username: &str,
    ) -> Result<Option<Profile>, ProfileError>;

    /// Create or update a profile
    async fn upsert_profile(&self, profile: Profile) -> Result<Profile, ProfileError>;
}

/// S3 storage operations for profile pictures
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ProfilePictureStorage {
    /// Generate a presigned URL for uploading a profile picture
    ///
    /// Returns:
    /// - upload_url: The URL to upload the image to
    /// - picture_url: The URL where the image will be accessible after upload
    /// - expires_in: Expiration time in seconds for the upload URL
    async fn generate_upload_url(
        &self,
        profile_id: Uuid,
    ) -> Result<(String, String, u64), ProfileError>;

    /// Delete a profile picture
    async fn delete_profile_picture(&self, profile_id: Uuid) -> Result<(), ProfileError>;
}
