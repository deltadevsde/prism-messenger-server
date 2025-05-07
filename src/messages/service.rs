use std::sync::Arc;
use uuid::Uuid;

use super::{
    database::MessageDatabase,
    entities::{DoubleRatchetMessage, Message, MessageReceipt},
    error::MessagingError,
};
use crate::{account::database::AccountDatabase, notifications::gateway::NotificationGateway};

pub struct MessagingService<A, M, N>
where
    A: AccountDatabase,
    M: MessageDatabase,
    N: NotificationGateway,
{
    account_db: Arc<A>,
    messages_db: Arc<M>,
    notification_gateway: Arc<N>,
}

impl<A, M, N> MessagingService<A, M, N>
where
    A: AccountDatabase,
    M: MessageDatabase,
    N: NotificationGateway,
{
    pub fn new(
        account_db: Arc<A>,
        messages_db: Arc<M>,
        notification_gateway: Arc<N>,
    ) -> MessagingService<A, M, N> {
        MessagingService {
            account_db,
            messages_db,
            notification_gateway,
        }
    }

    pub async fn send_message(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        message: DoubleRatchetMessage,
    ) -> Result<MessageReceipt, MessagingError> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: Uuid::new_v4(),
            sender_id,
            recipient_id,
            message: message.clone(),
            timestamp,
        };

        self.messages_db.insert_message(message.clone())?;

        let recipient_account = self
            .account_db
            .fetch_account(recipient_id)
            .await?
            .ok_or(MessagingError::UserNotFound(recipient_id.to_string()))?;

        if let Some(device_token) = recipient_account.apns_token {
            match self
                .notification_gateway
                .send_silent_notification(&device_token)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to send notification: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(MessageReceipt {
            message_id: message.message_id,
            timestamp,
        })
    }

    pub async fn get_messages(&self, account_id: Uuid) -> Result<Vec<Message>, MessagingError> {
        self.messages_db.get_messages(account_id)
    }

    pub async fn mark_delivered(
        &self,
        account_id: Uuid,
        message_ids: Vec<Uuid>,
    ) -> Result<bool, MessagingError> {
        self.messages_db.mark_delivered(account_id, message_ids)
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use prism_client::SigningKey;
    use std::sync::Arc;
    use uuid::Uuid;

    use super::MessagingService;
    use crate::account::{
        database::{AccountDatabase, MockAccountDatabase},
        entities::{Account, AccountIdentity},
    };
    use crate::database::inmemory::InMemoryDatabase;
    use crate::messages::{
        database::MockMessageDatabase,
        entities::{DoubleRatchetHeader, DoubleRatchetMessage},
        error::MessagingError,
    };
    use crate::notifications::gateway::{
        MockNotificationGateway, NotificationError, dummy::DummyNotificationGateway,
    };

    const ALICE_USERNAME: &str = "alice";
    const BOB_USERNAME: &str = "bob";
    const ALICE_PUSH_TOKEN: &[u8] = b"alice_apns_token";
    const BOB_PUSH_TOKEN: &[u8] = b"bob_apns_token";

    #[tokio::test]
    async fn test_send_and_get_message() {
        let (acc_db, alice_id, bob_id) = db_with_alice_and_bob().await;
        let mut not_gw = MockNotificationGateway::new();

        not_gw
            .expect_send_silent_notification()
            .once()
            .with(eq(ALICE_PUSH_TOKEN))
            .returning(|_| Ok(()));

        let acc_db_arc = Arc::new(acc_db);
        let service = MessagingService::new(acc_db_arc.clone(), acc_db_arc, Arc::new(not_gw));

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

        service
            .send_message(bob_id, alice_id, message)
            .await
            .expect("Could not send Bob's message to Alice");

        let retrieved_messages = service
            .get_messages(alice_id)
            .await
            .expect("Could not fetch message for Alice");

        assert_eq!(retrieved_messages.len(), 1);
        let alices_msg = retrieved_messages.first().unwrap();
        assert_eq!(alices_msg.sender_id, bob_id);
        assert_eq!(alices_msg.recipient_id, alice_id);
        assert_eq!(
            alices_msg.message.ciphertext,
            "Hello, Alice".as_bytes().to_vec()
        )
    }

    #[tokio::test]
    async fn test_set_messages_delivered() {
        let (acc_db, alice_id, bob_id) = db_with_alice_and_bob().await;
        let not_gw = DummyNotificationGateway;

        let acc_db_arc = Arc::new(acc_db);
        let service = MessagingService::new(acc_db_arc.clone(), acc_db_arc, Arc::new(not_gw));

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
            .get_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");

        let ids: Vec<Uuid> = retrieved.iter().map(|msg| msg.message_id).collect();
        let delivered = ids[5..10].to_vec();
        service
            .mark_delivered(alice_id, delivered.clone())
            .await
            .expect("Could not set messages delivered");

        let rest = service
            .get_messages(alice_id)
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
            .get_messages(alice_id)
            .await
            .expect("Could not fetch messages for Alice");
        assert_eq!(final_messages.len(), 0);
    }

    async fn db_with_alice_and_bob() -> (InMemoryDatabase, Uuid, Uuid) {
        let db = InMemoryDatabase::new();

        let alice_account = Account::new(
            AccountIdentity::Username(ALICE_USERNAME.to_string()),
            "alice_auth_password",
            Some(ALICE_PUSH_TOKEN.to_vec()),
            None,
        );

        let bob_account = Account::new(
            AccountIdentity::Username(BOB_USERNAME.to_string()),
            "bob_auth_password",
            Some(BOB_PUSH_TOKEN.to_vec()),
            None,
        );

        // Store account UUIDs for test assertions
        let alice_id = alice_account.id;
        let bob_id = bob_account.id;

        db.upsert_account(alice_account).await.unwrap();
        db.upsert_account(bob_account).await.unwrap();

        (db, alice_id, bob_id)
    }

    #[tokio::test]
    async fn test_send_message_user_not_found() {
        // Setup mock account database that returns NotFound for the recipient
        let mut mock_account_db = MockAccountDatabase::new();
        let unknown_id = Uuid::new_v4();
        mock_account_db
            .expect_fetch_account()
            .with(eq(unknown_id))
            .times(1)
            .returning(|_| Ok(None));

        // Setup mock message database
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_insert_message()
            .times(1)
            .returning(|_| Ok(true));

        // Setup mock notification gateway (not expected to be called)
        let mock_notification_gw = MockNotificationGateway::new();

        // Create the service with our mocks
        let service = MessagingService::new(
            Arc::new(mock_account_db),
            Arc::new(mock_message_db),
            Arc::new(mock_notification_gw),
        );

        // Create a test message
        let message = create_test_message();

        // Call service.send_message with unknown recipient
        let result = service
            .send_message(Uuid::new_v4(), unknown_id, message)
            .await;

        // Verify we get UserNotFound error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::UserNotFound(id) if id == unknown_id.to_string()));
    }

    #[tokio::test]
    async fn test_send_message_database_error() {
        // Setup mock message database that returns an error
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_insert_message()
            .times(1)
            .returning(|_| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Setup mock account database (not expected to be called because db error happens first)
        let mock_account_db = MockAccountDatabase::new();

        // Setup mock notification gateway (not expected to be called)
        let mock_notification_gw = MockNotificationGateway::new();

        // Create the service with our mocks
        let service = MessagingService::new(
            Arc::new(mock_account_db),
            Arc::new(mock_message_db),
            Arc::new(mock_notification_gw),
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
    async fn test_send_message_notification_error() {
        // Setup mock account database that returns an account with a token
        let mut mock_account_db = MockAccountDatabase::new();
        let recipient_id = Uuid::new_v4();
        mock_account_db
            .expect_fetch_account()
            .with(eq(recipient_id))
            .times(1)
            .returning(|_| {
                Ok(Some(Account::new(
                    AccountIdentity::Username("recipient".to_string()),
                    "password",
                    Some(vec![1, 2, 3, 4]), // Device token
                    None,
                )))
            });

        // Setup mock message database
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_insert_message()
            .times(1)
            .returning(|_| Ok(true));

        // Setup mock notification gateway that returns an error
        let mut mock_notification_gw = MockNotificationGateway::new();
        mock_notification_gw
            .expect_send_silent_notification()
            .with(eq(vec![1, 2, 3, 4]))
            .times(1)
            .returning(|_| {
                Err(NotificationError::SendFailure(
                    "Failed to send notification".to_string(),
                ))
            });

        // Create the service with our mocks
        let service = MessagingService::new(
            Arc::new(mock_account_db),
            Arc::new(mock_message_db),
            Arc::new(mock_notification_gw),
        );

        // Create a test message
        let message = create_test_message();

        // Call service.send_message
        let result = service
            .send_message(Uuid::new_v4(), recipient_id, message)
            .await;

        // Verify we get NotificationError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::NotificationError(_)));
    }

    #[tokio::test]
    async fn test_get_messages_database_error() {
        let account_id = Uuid::new_v4();
        // Setup mock message database that returns an error
        let mut mock_message_db = MockMessageDatabase::new();
        mock_message_db
            .expect_get_messages()
            .with(eq(account_id))
            .times(1)
            .returning(|_| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Create the service with our mocks
        let service = MessagingService::new(
            Arc::new(MockAccountDatabase::new()),
            Arc::new(mock_message_db),
            Arc::new(MockNotificationGateway::new()),
        );

        // Call service.get_messages
        let result = service.get_messages(account_id).await;

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
            .expect_mark_delivered()
            .with(eq(account_id), eq(vec![Uuid::nil()]))
            .times(1)
            .returning(|_, _| Err(MessagingError::DatabaseError("Database error".to_string())));

        // Create the service with our mocks
        let service = MessagingService::new(
            Arc::new(MockAccountDatabase::new()),
            Arc::new(mock_message_db),
            Arc::new(MockNotificationGateway::new()),
        );

        // Call service.mark_delivered
        let result = service.mark_delivered(account_id, vec![Uuid::nil()]).await;

        // Verify we get DatabaseError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MessagingError::DatabaseError(_)));
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
}
