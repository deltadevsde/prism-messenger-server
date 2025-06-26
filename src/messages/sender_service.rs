use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, instrument, warn};

use super::{database::MessageDatabase, error::MessagingError, gateway::MessageGateway};

pub struct MessageSenderService<D, G>
where
    D: MessageDatabase + 'static,
    G: MessageGateway + 'static,
{
    messages_db: Arc<D>,
    message_gateway: Arc<G>,
    poll_interval: Duration,
}

impl<D, G> MessageSenderService<D, G>
where
    D: MessageDatabase + 'static,
    G: MessageGateway + 'static,
{
    pub fn new(messages_db: Arc<D>, message_gateway: Arc<G>, poll_interval: Duration) -> Self {
        Self {
            messages_db,
            message_gateway,
            poll_interval,
        }
    }

    /// Spawn the background task that continuously processes pending messages
    #[instrument(skip(self))]
    pub fn spawn_message_sender(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Starting MessageSenderService background task");
            let mut ticker = interval(self.poll_interval);

            loop {
                ticker.tick().await;
                if let Err(e) = self.try_send_pending_messages().await {
                    error!("Error processing pending messages: {}", e);
                }
            }
        })
    }

    /// Process all pending messages for all accounts
    #[instrument(skip(self))]
    async fn try_send_pending_messages(&self) -> Result<(), MessagingError> {
        let messages = self.messages_db.get_all_messages()?;

        for message in messages {
            let recipient_id = message.recipient_id;
            let message_id = message.message_id;
            let result = self.message_gateway.send_message(message).await;

            match result {
                Ok(()) => {
                    info!(
                        "Successfully sent message {} to recipient {}",
                        message_id, recipient_id
                    );
                    self.messages_db
                        .remove_messages(recipient_id, vec![message_id])?;
                }
                Err(MessagingError::UserNotFound(account_id)) => {
                    // Recipient is not connected, leave message in database for later delivery
                    debug!("Recipient {} not found. Doing nothing", account_id);
                    continue;
                }
                Err(e) => {
                    warn!(
                        "Failed to send message {} to recipient {}: {}",
                        message_id, recipient_id, e
                    );
                    continue;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{
        database::MockMessageDatabase, entities::*, gateway::MockMessageGateway,
    };
    use mockall::predicate::*;
    use prism_client::SigningKey;
    use std::time::Duration;
    use tokio::time::sleep;
    use uuid::Uuid;

    fn create_test_message(recipient_id: Uuid) -> Message {
        Message {
            message_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id,
            message: DoubleRatchetMessage {
                header: DoubleRatchetHeader {
                    ephemeral_key: SigningKey::new_secp256r1().verifying_key(),
                    message_number: 1,
                    previous_message_number: 0,
                    one_time_prekey_id: Some(1),
                },
                ciphertext: vec![1, 2, 3, 4],
                nonce: vec![5, 6, 7, 8],
            },
            timestamp: 1234567890,
        }
    }

    #[tokio::test]
    async fn test_successful_message_sending() {
        let recipient_id = Uuid::new_v4();
        let message = create_test_message(recipient_id);
        let message_id = message.message_id;

        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .once()
            .returning(move || Ok(vec![message.clone()]));
        mock_db
            .expect_remove_messages()
            .once()
            .with(eq(recipient_id), eq(vec![message_id]))
            .returning(|_, _| Ok(()));

        let mut mock_gateway = MockMessageGateway::new();
        mock_gateway
            .expect_send_message()
            .once()
            .with(always())
            .returning(|_| Ok(()));

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recipient_not_connected_leaves_message_in_queue() {
        let recipient_id = Uuid::new_v4();
        let message = create_test_message(recipient_id);

        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .once()
            .returning(move || Ok(vec![message.clone()]));

        let mut mock_gateway = MockMessageGateway::new();
        mock_gateway
            .expect_send_message()
            .once()
            .with(always())
            .returning(|_| Err(MessagingError::UserNotFound("test".to_string())));

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        // Should succeed because message stays in queue when recipient not connected
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_other_rtc_errors_continue_processing() {
        let recipient_id1 = Uuid::new_v4();
        let recipient_id2 = Uuid::new_v4();
        let message1 = create_test_message(recipient_id1);
        let message2 = create_test_message(recipient_id2);
        let message2_id = message2.message_id;

        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .once()
            .returning(move || Ok(vec![message1.clone(), message2.clone()]));
        mock_db
            .expect_remove_messages()
            .once()
            .with(eq(recipient_id2), eq(vec![message2_id]))
            .returning(|_, _| Ok(()));

        let mut mock_gateway = MockMessageGateway::new();
        mock_gateway
            .expect_send_message()
            .once()
            .with(always())
            .returning(|_| {
                Err(MessagingError::SendingFailed(
                    "Connection closed".to_string(),
                ))
            });
        mock_gateway
            .expect_send_message()
            .once()
            .with(always())
            .returning(|_| Ok(()));

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_error_propagates() {
        let mut mock_db = MockMessageDatabase::new();
        mock_db.expect_get_all_messages().once().returning(|| {
            Err(MessagingError::DatabaseError(
                "Connection failed".to_string(),
            ))
        });

        let mock_gateway = MockMessageGateway::new();

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::DatabaseError(_)
        ));
    }

    #[tokio::test]
    async fn test_empty_message_queue() {
        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .once()
            .returning(|| Ok(vec![]));

        let mock_gateway = MockMessageGateway::new();

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_spawn_message_sender_runs_continuously() {
        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .times(2..)
            .returning(|| Ok(vec![]));

        let mock_gateway = MockMessageGateway::new();

        let service = Arc::new(MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        ));

        let handle = service.spawn_message_sender();

        // Let it run for a short time to ensure it processes multiple iterations
        sleep(Duration::from_millis(150)).await;

        handle.abort();
        let result = handle.await;
        assert!(result.is_err()); // Expected because we aborted the task
    }

    #[tokio::test]
    async fn test_remove_message_failure_stops_processing() {
        let recipient_id1 = Uuid::new_v4();
        let recipient_id2 = Uuid::new_v4();
        let message1 = create_test_message(recipient_id1);
        let message2 = create_test_message(recipient_id2);
        let message1_id = message1.message_id;

        let mut mock_db = MockMessageDatabase::new();
        mock_db
            .expect_get_all_messages()
            .once()
            .returning(move || Ok(vec![message1.clone(), message2.clone()]));
        mock_db
            .expect_remove_messages()
            .once()
            .with(eq(recipient_id1), eq(vec![message1_id]))
            .returning(|_, _| Err(MessagingError::DatabaseError("Remove failed".to_string())));
        // Second remove_messages should NOT be called because the first one fails and returns early

        let mut mock_gateway = MockMessageGateway::new();
        mock_gateway
            .expect_send_message()
            .once()
            .with(always())
            .returning(|_| Ok(()));
        // Second send should NOT be called because processing stops after first remove failure

        let service = MessageSenderService::new(
            Arc::new(mock_db),
            Arc::new(mock_gateway),
            Duration::from_millis(100),
        );

        let result = service.try_send_pending_messages().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::DatabaseError(_)
        ));
    }
}
