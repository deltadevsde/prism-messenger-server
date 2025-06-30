use std::sync::Arc;
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::{
    account::database::AccountDatabase,
    notifications::{gateway::NotificationGateway, service::NotificationService},
    presence::database::PresenceDatabase,
};

use super::{
    database::MessageDatabase,
    entities::{DoubleRatchetMessage, Message, MessageReceipt},
    error::MessagingError,
};

pub struct MessagingService<M, P, A, N>
where
    M: MessageDatabase,
    P: PresenceDatabase,
    A: AccountDatabase,
    N: NotificationGateway,
{
    messages_db: Arc<M>,
    presence_db: Arc<P>,
    notification_service: Arc<NotificationService<A, N>>,
}

impl<M, P, A, N> MessagingService<M, P, A, N>
where
    M: MessageDatabase,
    P: PresenceDatabase,
    A: AccountDatabase,
    N: NotificationGateway,
{
    pub fn new(
        messages_db: Arc<M>,
        presence_db: Arc<P>,
        notification_service: Arc<NotificationService<A, N>>,
    ) -> MessagingService<M, P, A, N> {
        MessagingService {
            messages_db,
            presence_db,
            notification_service,
        }
    }

    #[instrument(skip(self, message), fields(sender_id, recipient_id))]
    pub async fn send_message(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        message: DoubleRatchetMessage,
    ) -> Result<MessageReceipt, MessagingError> {
        let is_recipient_present = match self.presence_db.is_present(&recipient_id).await {
            Ok(present) => {
                debug!("Recipient {} presence status: {}", recipient_id, present);
                present
            }
            Err(e) => {
                error!(
                    "Failed to check recipient presence for {}: {}",
                    recipient_id, e
                );
                true
            }
        };

        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: Uuid::new_v4(),
            sender_id,
            recipient_id,
            message: message.clone(),
            timestamp,
        };

        self.messages_db.insert_message(message.clone())?;

        if !is_recipient_present {
            self.notification_service
                .send_wakeup_notification(recipient_id)
                .await?;
        }

        Ok(MessageReceipt {
            message_id: message.message_id,
            timestamp,
        })
    }

    pub async fn get_pending_messages(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<Message>, MessagingError> {
        self.messages_db.get_messages_for_account(account_id)
    }

    pub async fn mark_delivered(
        &self,
        account_id: Uuid,
        message_ids: Vec<Uuid>,
    ) -> Result<(), MessagingError> {
        self.messages_db.remove_messages(account_id, message_ids)
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use prism_client::SigningKey;
    use std::sync::Arc;
    use uuid::Uuid;

    use super::MessagingService;
    use crate::account::database::MockAccountDatabase;
    use crate::database::inmemory::InMemoryDatabase;
    use crate::messages::{
        database::MockMessageDatabase,
        entities::{DoubleRatchetHeader, DoubleRatchetMessage},
        error::MessagingError,
    };
    use crate::notifications::{gateway::MockNotificationGateway, service::NotificationService};
    use crate::presence::database::MockPresenceDatabase;
    use crate::presence::error::PresenceError;

    #[tokio::test]
    async fn test_send_and_get_message() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Ok(true));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let bob_ephemeral_key = SigningKey::new_secp256r1().verifying_key();

        let header = DoubleRatchetHeader {
            ephemeral_key: bob_ephemeral_key,
            message_number: 0,
            previous_message_number: 0,
            one_time_prekey_id: Some(0),
        };

        let message = DoubleRatchetMessage {
            header,
            ciphertext: "Hello, Alice".as_bytes().to_vec(),
            nonce: vec![0; 12],
        };

        let receipt = service
            .send_message(bob_id, alice_id, message)
            .await
            .expect("Could not send Bob's message to Alice");

        // Verify receipt contains message_id and timestamp
        assert!(receipt.timestamp > 0);

        let retrieved_messages = service
            .get_pending_messages(alice_id)
            .await
            .expect("Could not fetch message for Alice");

        assert_eq!(retrieved_messages.len(), 1);
        let alices_msg = retrieved_messages.first().unwrap();
        assert_eq!(alices_msg.message_id, receipt.message_id);
        assert_eq!(alices_msg.sender_id, bob_id);
        assert_eq!(alices_msg.recipient_id, alice_id);
        assert_eq!(alices_msg.timestamp, receipt.timestamp);
        assert_eq!(
            alices_msg.message.ciphertext,
            "Hello, Alice".as_bytes().to_vec()
        );
    }

    #[tokio::test]
    async fn test_mark_messages_delivered() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(20)
            .returning(|_| Ok(true));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let bob_ephemeral_key = SigningKey::new_secp256r1().verifying_key();

        let mut message_ids = Vec::new();
        for i in 0..20 {
            let new_header = DoubleRatchetHeader {
                ephemeral_key: bob_ephemeral_key.clone(),
                message_number: i + 1,
                previous_message_number: i,
                one_time_prekey_id: None,
            };
            let new_message = DoubleRatchetMessage {
                header: new_header,
                ciphertext: format!("Hello, Alice {}", i).as_bytes().to_vec(),
                nonce: vec![0; 12],
            };

            let receipt = service
                .send_message(bob_id, alice_id, new_message)
                .await
                .expect("Could not send message");

            message_ids.push(receipt.message_id);
        }

        let retrieved = service
            .get_pending_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");

        let ids: Vec<Uuid> = retrieved.iter().map(|msg| msg.message_id).collect();
        let delivered = ids[5..10].to_vec();
        service
            .mark_delivered(alice_id, delivered.clone())
            .await
            .expect("Could not set messages delivered");

        let rest = service
            .get_pending_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");
        let rest_ids: Vec<Uuid> = rest.iter().map(|msg| msg.message_id).collect();

        // 15 messages left to mark as delivered
        assert_eq!(rest_ids.len(), 15);

        // rest_uuids shouldn't contain any from delivered
        assert!(!rest_ids.iter().any(|uuid| delivered.contains(uuid)));

        service
            .mark_delivered(alice_id, rest_ids)
            .await
            .expect("Could not set messages delivered");
        let final_messages = service
            .get_pending_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");
        assert_eq!(final_messages.len(), 0);
    }

    // Helper function to create a test message
    fn create_test_message() -> DoubleRatchetMessage {
        let ephemeral_key = SigningKey::new_secp256r1().verifying_key();
        let header = DoubleRatchetHeader {
            ephemeral_key,
            message_number: 0,
            previous_message_number: 0,
            one_time_prekey_id: None,
        };

        DoubleRatchetMessage {
            header,
            ciphertext: "Test message".as_bytes().to_vec(),
            nonce: vec![0; 12],
        }
    }

    #[tokio::test]
    async fn test_send_message_database_error() {
        // Setup mock message database that returns an error
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_insert_message()
            .times(1)
            .returning(|_| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Create the service with our mocks
        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db.expect_is_present().returning(|_| Ok(true));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            Arc::new(mock_message_db),
            presence_db_arc,
            Arc::new(notification_service),
        );

        // Create a test message
        let message = create_test_message();

        // Call service.send_message
        let result = service
            .send_message(Uuid::new_v4(), Uuid::new_v4(), message)
            .await;

        // Verify we get DatabaseError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::DatabaseError(_)));
    }

    #[tokio::test]
    async fn test_get_pending_messages_database_error() {
        let account_id = Uuid::new_v4();
        // Setup mock message database that returns an error
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_get_messages_for_account()
            .with(eq(account_id))
            .times(1)
            .returning(|_| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Create the service with our mocks
        let mock_presence_db = MockPresenceDatabase::new();
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            Arc::new(mock_message_db),
            presence_db_arc,
            Arc::new(notification_service),
        );

        // Call service.get_pending_messages
        let result = service.get_pending_messages(account_id).await;

        // Verify we get DatabaseError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::DatabaseError(_)));
    }

    #[tokio::test]
    async fn test_mark_delivered_database_error() {
        let account_id = Uuid::new_v4();
        // Setup mock message database that returns an error
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_remove_messages()
            .with(eq(account_id), eq(vec![Uuid::nil()]))
            .times(1)
            .returning(|_, _| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Create the service with our mocks
        let mock_presence_db = MockPresenceDatabase::new();
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            Arc::new(mock_message_db),
            presence_db_arc,
            Arc::new(notification_service),
        );

        // Call service.mark_delivered
        let result = service.mark_delivered(account_id, vec![Uuid::nil()]).await;

        // Verify we get DatabaseError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::DatabaseError(_)));
    }

    #[tokio::test]
    async fn test_get_all_messages() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db.expect_is_present().returning(|_| Ok(true));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let bob_ephemeral_key = SigningKey::new_secp256r1().verifying_key();

        // Send multiple messages from Bob to Alice
        for i in 0..5 {
            let header = DoubleRatchetHeader {
                ephemeral_key: bob_ephemeral_key.clone(),
                message_number: i,
                previous_message_number: if i > 0 { i - 1 } else { 0 },
                one_time_prekey_id: None,
            };

            let message = DoubleRatchetMessage {
                header,
                ciphertext: format!("Message {} from Bob to Alice", i)
                    .as_bytes()
                    .to_vec(),
                nonce: vec![0; 12],
            };

            service
                .send_message(bob_id, alice_id, message)
                .await
                .expect("Could not send message");
        }

        // Send multiple messages from Alice to Bob
        let alice_ephemeral_key = SigningKey::new_secp256r1().verifying_key();
        for i in 0..3 {
            let header = DoubleRatchetHeader {
                ephemeral_key: alice_ephemeral_key.clone(),
                message_number: i,
                previous_message_number: if i > 0 { i - 1 } else { 0 },
                one_time_prekey_id: None,
            };

            let message = DoubleRatchetMessage {
                header,
                ciphertext: format!("Message {} from Alice to Bob", i)
                    .as_bytes()
                    .to_vec(),
                nonce: vec![0; 12],
            };

            service
                .send_message(alice_id, bob_id, message)
                .await
                .expect("Could not send message");
        }

        // Get all messages for Alice
        let alice_messages = service
            .get_pending_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");

        // Get all messages for Bob
        let bob_messages = service
            .get_pending_messages(bob_id)
            .await
            .expect("Could not fetch messages for Bob");

        // Verify Alice received 5 messages from Bob
        assert_eq!(alice_messages.len(), 5);
        for (i, msg) in alice_messages.iter().enumerate() {
            assert_eq!(msg.sender_id, bob_id);
            assert_eq!(msg.recipient_id, alice_id);
            assert_eq!(
                msg.message.ciphertext,
                format!("Message {} from Bob to Alice", i)
                    .as_bytes()
                    .to_vec()
            );
        }

        // Verify Bob received 3 messages from Alice
        assert_eq!(bob_messages.len(), 3);
        for (i, msg) in bob_messages.iter().enumerate() {
            assert_eq!(msg.sender_id, alice_id);
            assert_eq!(msg.recipient_id, bob_id);
            assert_eq!(
                msg.message.ciphertext,
                format!("Message {} from Alice to Bob", i)
                    .as_bytes()
                    .to_vec()
            );
        }
    }

    #[tokio::test]
    async fn test_send_message_with_recipient_present_no_notification() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock presence database to return that recipient is present
        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Ok(true));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock notification gateway should not be called
        let mock_notification_gateway = MockNotificationGateway::new();
        let notification_gateway_arc = Arc::new(mock_notification_gateway);

        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let message = create_test_message();

        let result = service.send_message(bob_id, alice_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_with_recipient_absent_sends_notification() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock presence database to return that recipient is not present
        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Ok(false));
        let presence_db_arc = Arc::new(mock_presence_db);

        // Mock account database for notification service
        let mut mock_account_db = MockAccountDatabase::new();
        mock_account_db
            .expect_fetch_account()
            .with(eq(alice_id))
            .times(1)
            .returning(|id| {
                Ok(Some(crate::account::entities::Account {
                    id,
                    auth_password_hash: crate::crypto::salted_hash::SaltedHash::generate_from(
                        "password",
                    ),
                    apns_token: Some(vec![1, 2, 3, 4, 5]),
                    gcm_token: None,
                }))
            });
        let account_db_arc = Arc::new(mock_account_db);

        // Mock notification gateway to expect a call
        let mut mock_notification_gateway = MockNotificationGateway::new();
        mock_notification_gateway
            .expect_send_silent_notification()
            .times(1)
            .returning(|_| Ok(()));
        let notification_gateway_arc = Arc::new(mock_notification_gateway);

        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let message = create_test_message();

        let result = service.send_message(bob_id, alice_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_presence_db_error_defaults_to_present() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock presence database to return an error
        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Err(PresenceError::Database("DB error".to_string())));
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock notification gateway should not be called (defaults to present)
        let mock_notification_gateway = MockNotificationGateway::new();
        let notification_gateway_arc = Arc::new(mock_notification_gateway);

        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let message = create_test_message();

        let result = service.send_message(bob_id, alice_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_notification_service_error_still_sends_message() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        // Mock presence database to return that recipient is not present
        let mut mock_presence_db = MockPresenceDatabase::new();
        mock_presence_db
            .expect_is_present()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Ok(false));
        let presence_db_arc = Arc::new(mock_presence_db);

        // Mock account database that returns no account
        let mut mock_account_db = MockAccountDatabase::new();
        mock_account_db
            .expect_fetch_account()
            .with(eq(alice_id))
            .times(1)
            .returning(|_| Ok(None));
        let account_db_arc = Arc::new(mock_account_db);

        let mock_notification_gateway = MockNotificationGateway::new();
        let notification_gateway_arc = Arc::new(mock_notification_gateway);

        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let message = create_test_message();

        // Should fail when notification fails
        let result = service.send_message(bob_id, alice_id, message).await;
        assert!(result.is_err()); // Should fail due to notification error
        assert!(matches!(
            result.unwrap_err(),
            MessagingError::NotificationError(_)
        ));
    }

    #[tokio::test]
    async fn test_get_pending_messages_empty() {
        let account_id = Uuid::new_v4();

        let message_db_arc = Arc::new(InMemoryDatabase::new());

        let mock_presence_db = MockPresenceDatabase::new();
        let presence_db_arc = Arc::new(mock_presence_db);

        let account_db_arc = Arc::new(InMemoryDatabase::new());
        let notification_gateway_arc = Arc::new(MockNotificationGateway::new());
        let notification_service =
            NotificationService::new(account_db_arc, notification_gateway_arc);
        let service = MessagingService::new(
            message_db_arc,
            presence_db_arc,
            Arc::new(notification_service),
        );

        let result = service.get_pending_messages(account_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 0);
    }
}
