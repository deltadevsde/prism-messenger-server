use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use super::database::{ProfileDatabase, ProfilePictureStorage};
use super::entities::{
    Profile, ProfilePictureAction, ProfilePictureUploadResponse, ProfileResponse,
    UpdateProfileRequest,
};
use super::error::ProfileError;

pub struct ProfileService<D, S>
where
    D: ProfileDatabase,
    S: ProfilePictureStorage,
{
    profile_db: Arc<D>,
    picture_storage: Arc<S>,
}

impl<D, S> ProfileService<D, S>
where
    D: ProfileDatabase,
    S: ProfilePictureStorage,
{
    pub fn new(profile_db: Arc<D>, picture_storage: Arc<S>) -> Self {
        Self {
            profile_db,
            picture_storage,
        }
    }

    /// Get a profile response by account ID
    pub async fn get_profile_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<ProfileResponse, ProfileError> {
        let profile = self
            .profile_db
            .get_profile_by_account_id(account_id)
            .await?
            .ok_or(ProfileError::NotFound)?;

        // Create and return the profile response
        Ok(ProfileResponse::new(profile))
    }

    pub async fn get_profile_by_username(
        &self,
        username: &str,
    ) -> Result<ProfileResponse, ProfileError> {
        let profile = self
            .profile_db
            .get_profile_by_username(username)
            .await?
            .ok_or(ProfileError::NotFound)?;

        // Create and return the profile response
        Ok(ProfileResponse::new(profile))
    }

    /// Updates a user's profile. If profile picture shall be updated, creates a new upload URL.
    pub async fn update_profile(
        &self,
        account_id: Uuid,
        update_req: UpdateProfileRequest,
    ) -> Result<Option<ProfilePictureUploadResponse>, ProfileError> {
        // Get the existing profile
        let mut profile = match self
            .profile_db
            .get_profile_by_account_id(account_id)
            .await?
        {
            Some(profile) => profile,
            None => return Err(ProfileError::NotFound),
        };

        // Update fields if provided
        if let Some(display_name) = update_req.display_name {
            profile.display_name = Some(display_name);
        }

        // Store the action for later use
        let action = update_req.profile_picture_action.clone();

        // Variable to store upload response if needed
        let mut upload_response = None;

        // Handle profile picture action
        match &action {
            ProfilePictureAction::NoChange => {
                // Do nothing with the profile picture
            }
            ProfilePictureAction::Clear => {
                // If there was a previous picture, delete it
                if profile.profile_picture_url.is_some() {
                    self.picture_storage
                        .delete_profile_picture(profile.id)
                        .await?;
                }
                profile.profile_picture_url = None;
            }
            ProfilePictureAction::Update => {
                // If there was a previous picture, delete it
                if profile.profile_picture_url.is_some() {
                    self.picture_storage
                        .delete_profile_picture(profile.id)
                        .await?;
                }

                // Generate upload URL once and use it for both updating profile and returning to client
                let (upload_url, picture_url, expires_in) =
                    self.picture_storage.generate_upload_url(profile.id).await?;

                // Update profile with the new picture URL
                profile.profile_picture_url = Some(picture_url.clone());

                // Store the response for later return
                upload_response = Some(ProfilePictureUploadResponse {
                    upload_url,
                    picture_url,
                    expires_in,
                });
            }
        }

        // Update the timestamp
        profile.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ProfileError::Internal(e.to_string()))?
            .as_millis() as u64;

        // Save the updated profile
        self.profile_db.upsert_profile(profile.clone()).await?;

        // Return the upload response if available
        Ok(upload_response)
    }

    /// Generate a pre-signed URL for uploading a profile picture
    pub async fn generate_profile_picture_upload_url(
        &self,
        account_id: Uuid,
    ) -> Result<ProfilePictureUploadResponse, ProfileError> {
        // Get the profile to ensure it exists and to get the ID
        let profile = match self
            .profile_db
            .get_profile_by_account_id(account_id)
            .await?
        {
            Some(profile) => profile,
            None => return Err(ProfileError::NotFound),
        };

        // Call the helper method to generate the upload URL
        self.generate_profile_picture_upload_url_from_profile(&profile)
            .await
    }

    /// Generate a pre-signed URL for uploading a profile picture from an existing profile
    async fn generate_profile_picture_upload_url_from_profile(
        &self,
        profile: &Profile,
    ) -> Result<ProfilePictureUploadResponse, ProfileError> {
        // Generate the upload URL from S3
        let (upload_url, picture_url, expires_in) =
            self.picture_storage.generate_upload_url(profile.id).await?;

        // Return the response
        Ok(ProfilePictureUploadResponse {
            upload_url,
            picture_url,
            expires_in,
        })
    }
}
