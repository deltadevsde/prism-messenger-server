use crate::PRISM_MESSENGER_SERVICE_ID;
use prism_client::{PendingTransaction, PrismApi, PrismApiError, SigningKey};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitializationError {
    #[error("Failed to register service: {0}")]
    ServiceRegistrationError(#[from] PrismApiError),
}

pub struct InitializationService<P: PrismApi> {
    prism: Arc<P>,
    signing_key: SigningKey,
}

impl<P: PrismApi> InitializationService<P> {
    pub fn new(prism: Arc<P>, signing_key: SigningKey) -> Self {
        Self { prism, signing_key }
    }

    /// Initialize the messenger server
    pub async fn initialize_messenger_server(&self) -> Result<(), InitializationError> {
        tracing::info!("Initializing messenger service");
        self.register_messenger_service().await?;
        tracing::info!("Messenger service initialization completed");
        Ok(())
    }

    async fn register_messenger_service(&self) -> Result<(), InitializationError> {
        let service_acc_response = self.prism.get_account(PRISM_MESSENGER_SERVICE_ID).await?;

        if service_acc_response.account.is_some() {
            tracing::info!("Messenger service already registered in prism");
            return Ok(());
        }

        self.prism
            .register_service(
                PRISM_MESSENGER_SERVICE_ID.to_string(),
                self.signing_key.verifying_key(),
                &self.signing_key,
            )
            .await?
            .wait()
            .await?;

        Ok(())
    }
}
