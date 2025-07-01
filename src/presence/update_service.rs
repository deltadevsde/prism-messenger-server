use std::sync::Arc;

use super::gateway::PresenceGateway;

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

    #[tracing::instrument(skip(self))]
    pub async fn handle_presence_updates(&self) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presence::entities::PresenceStatus;
    use crate::presence::error::PresenceError;
    use crate::presence::gateway::{MockPresenceGateway, PresenceUpdate};
    use mockall::predicate::*;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_handle_presence_updates() {
        let mut mock_gateway = MockPresenceGateway::new();
        let (tx, mut rx) = mpsc::channel(1);

        mock_gateway
            .expect_register_presence_handler()
            .with(always())
            .times(1)
            .returning(move |handler| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let update = PresenceUpdate::new(Uuid::new_v4(), PresenceStatus::Online);
                    handler(update);
                    tx.send(()).await.unwrap();
                });
            });

        mock_gateway
            .expect_send_presence_update()
            .with(always())
            .times(1)
            .returning(|_| Ok(()));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));
        service.handle_presence_updates().await;
        rx.recv().await.unwrap();
    }

    #[tokio::test]
    async fn test_handle_presence_updates_error() {
        let mut mock_gateway = MockPresenceGateway::new();
        let (tx, mut rx) = mpsc::channel(1);

        mock_gateway
            .expect_register_presence_handler()
            .with(always())
            .times(1)
            .returning(move |handler| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let update = PresenceUpdate::new(Uuid::new_v4(), PresenceStatus::Online);
                    handler(update);
                    tx.send(()).await.unwrap();
                });
            });

        mock_gateway
            .expect_send_presence_update()
            .with(always())
            .times(1)
            .returning(|_| Err(PresenceError::SendingFailed("test error".to_string())));

        let service = PresenceUpdateService::new(Arc::new(mock_gateway));
        service.handle_presence_updates().await;
        rx.recv().await.unwrap();
    }
}
