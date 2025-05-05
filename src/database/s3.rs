use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    presigning::PresigningConfig,
};
use std::time::Duration;
use uuid::Uuid;

use crate::profiles::{database::ProfilePictureStorage, error::ProfileError};
pub struct S3Storage {
    client: Client,
    bucket: String,
    region: String,
    endpoint: Option<String>,
}

impl S3Storage {
    pub async fn new(
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        endpoint: Option<String>,
    ) -> Result<Self, ProfileError> {
        // Create the AWS SDK config
        let mut config_builder = aws_config::defaults(BehaviorVersion::v2025_01_17())
            .region(Region::new(region.clone()));

        // Add credentials
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None, // session token
            None, // expiry time
            "prism-messenger",
        );
        config_builder = config_builder.credentials_provider(credentials);

        // Set custom endpoint if provided (useful for Minio, etc)
        if let Some(endpoint_url) = &endpoint {
            config_builder = config_builder.endpoint_url(endpoint_url.clone());
        }

        // Load the config
        let sdk_config = config_builder.load().await;

        // Create the S3 client
        let client = Client::new(&sdk_config);

        Ok(Self {
            client,
            bucket,
            region,
            endpoint,
        })
    }
}

#[async_trait]
impl ProfilePictureStorage for S3Storage {
    async fn generate_upload_url(
        &self,
        profile_id: Uuid,
    ) -> Result<(String, String, u64), ProfileError> {
        let object_key = format!("profiles/{}/profile.jpg", profile_id);
        let expires_in = 300; // 5 minutes in seconds

        // Create a presigned PUT URL for uploading
        let presign_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in))
            .build()
            .map_err(|err| {
                ProfileError::Database(format!("Failed to create presign config: {}", err))
            })?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .content_type("image/jpeg") // Assuming JPEG format
            .presigned(presign_config)
            .await
            .map_err(|err| {
                ProfileError::Database(format!("Failed to generate presigned URL: {}", err))
            })?;

        let upload_url = presigned_request.uri().to_string();

        // Generate the URL where the image will be accessible after upload
        let picture_url = if let Some(endpoint) = &self.endpoint {
            // For custom endpoints (like MinIO), use the endpoint URL directly
            format!("{}/{}/{}", endpoint, self.bucket, object_key)
        } else {
            // For AWS S3, use the standard URL format
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.bucket, self.region, object_key
            )
        };

        Ok((upload_url, picture_url, expires_in))
    }

    async fn delete_profile_picture(&self, profile_id: Uuid) -> Result<(), ProfileError> {
        let object_key = format!("profile-pictures/{}.jpg", profile_id);

        // Delete the object from S3
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|err| {
                ProfileError::Database(format!("Failed to delete profile picture: {}", err))
            })?;

        Ok(())
    }
}
