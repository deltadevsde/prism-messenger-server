use std::sync::Arc;
use tracing::debug;

use super::gateway::TypingGateway;

pub struct TypingService<G>
where
    G: TypingGateway + 'static,
{
    gateway: Arc<G>,
}

impl<G> TypingService<G>
where
    G: TypingGateway + 'static,
{
    pub fn new(gateway: Arc<G>) -> Self {
        Self { gateway }
    }

    pub async fn handle_typing_updates(&self) {
        let gateway = self.gateway.clone();
        self.gateway
            .register_typing_handler(move |typing_status| {
                debug!(
                    "Typing update {} -> {}: {}",
                    typing_status.recipient_id, typing_status.sender_id, typing_status.is_typing
                );
                let gateway = gateway.clone();
                tokio::spawn(async move {
                    if let Err(e) = gateway.send_typing_update(&typing_status).await {
                        tracing::error!("Failed to send typing update: {:?}", e);
                    }
                });
            })
            .await;
    }
}
