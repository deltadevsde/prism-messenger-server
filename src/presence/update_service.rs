use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use super::{
    entities::PresenceStatus,
    error::PresenceError,
    gateway::{PresenceGateway, PresenceUpdate},
};

pub struct PresenceUpdateService<G>
where
    G: PresenceGateway + 'static,
{
    presence_gateway: Arc<G>,
}

impl<G> PresenceUpdateService<G>
where
    G: PresenceGateway + 'static,
{
    pub fn new(presence_gateway: Arc<G>) -> Self {
        Self { presence_gateway }
    }

    #[instrument(skip(self))]
    pub async fn handle_connection_established(
        &self,
        account_id: Uuid,
    ) -> Result<(), PresenceError> {
        let presence_update = PresenceUpdate::new(account_id, PresenceStatus::Online);
        self.presence_gateway
            .send_presence_update(&presence_update)
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn handle_connection_closed(&self, account_id: Uuid) -> Result<(), PresenceError> {
        let presence_update = PresenceUpdate::new(account_id, PresenceStatus::Offline);
        self.presence_gateway
            .send_presence_update(&presence_update)
            .await?;
        Ok(())
    }

    pub async fn handle_presence_updates(&self) {
        {
            let gateway = self.presence_gateway.clone();
            self.presence_gateway
                .register_presence_handler(move |presence_update| {
                    let gateway = gateway.clone();
                    tokio::spawn(async move {
                        if let Err(e) = gateway.send_presence_update(&presence_update).await {
                            tracing::error!(
                                error = %e,
                                "Failed to send presence update"
                            );
                        }
                    });
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presence::gateway::MockPresenceGateway;

    #[tokio::test]
    async fn test_handle_connection_established() {
        let account_id = Uuid::new_v4();

        let mut mock_gateway = MockPresenceGateway::new();
        mock_gateway
            .expect_send_presence_update()
            .times(1)
            .returning(|_| Ok(()));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));

        let result = service.handle_connection_established(account_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_closed() {
        let account_id = Uuid::new_v4();

        let mut mock_gateway = MockPresenceGateway::new();
        mock_gateway
            .expect_send_presence_update()
            .times(1)
            .returning(|_| Ok(()));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));

        let result = service.handle_connection_closed(account_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_established_gateway_error() {
        let account_id = Uuid::new_v4();

        let mut mock_gateway = MockPresenceGateway::new();
        mock_gateway
            .expect_send_presence_update()
            .times(1)
            .returning(|_| Err(PresenceError::SendingFailed("Test error".to_string())));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));

        let result = service.handle_connection_established(account_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_closed_gateway_error() {
        let account_id = Uuid::new_v4();

        let mut mock_gateway = MockPresenceGateway::new();
        mock_gateway
            .expect_send_presence_update()
            .times(1)
            .returning(|_| Err(PresenceError::SendingFailed("Test error".to_string())));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));

        let result = service.handle_connection_closed(account_id).await;
        assert!(result.is_err());
    }
}
