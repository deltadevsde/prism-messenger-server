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
        sender_username: String,
        recipient_username: String,
        message: DoubleRatchetMessage,
    ) -> Result<MessageReceipt, MessagingError> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: uuid::Uuid::new_v4(),
            sender_username,
            recipient_username: recipient_username.clone(),
            message: message.clone(),
            timestamp,
        };

        self.messages_db.insert_message(message.clone())?;

        let recipient_account = self
            .account_db
            .fetch_account_by_username(&recipient_username)
            .await?;
        if let Some(device_token) = recipient_account.apns_token {
            self.notification_gateway
                .send_silent_notification(&device_token)
                .await?;
        }

        Ok(MessageReceipt {
            message_id: message.message_id,
            timestamp,
        })
    }

    pub async fn get_messages(&self, username: &str) -> Result<Vec<Message>, MessagingError> {
        self.messages_db.get_messages(username)
    }

    pub async fn mark_delivered(
        &self,
        username: &str,
        message_ids: Vec<Uuid>,
    ) -> Result<bool, MessagingError> {
        self.messages_db.mark_delivered(username, message_ids)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mockall::predicate::eq;
    use prism_client::SigningKey;
    use uuid::Uuid;

    use crate::account::database::AccountDatabase;
    use crate::account::entities::Account;
    use crate::database::inmemory::InMemoryDatabase;

    use crate::messages::entities::{DoubleRatchetHeader, DoubleRatchetMessage};
    use crate::notifications::gateway::MockNotificationGateway;
    use crate::notifications::gateway::dummy::DummyNotificationGateway;

    use super::MessagingService;

    const ALICE_USERNAME: &str = "alice";
    const BOB_USERNAME: &str = "bob";
    const ALICE_PUSH_TOKEN: &[u8] = b"alice_apns_token";
    const BOB_PUSH_TOKEN: &[u8] = b"bob_apns_token";

    #[tokio::test]
    async fn test_send_and_get_message() {
        let acc_db = db_with_alice_and_bob().await;
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
            .send_message(
                BOB_USERNAME.to_string(),
                ALICE_USERNAME.to_string(),
                message,
            )
            .await
            .expect("Could not send Bob's message to Alice");

        let retrieved_messages = service
            .get_messages(ALICE_USERNAME)
            .await
            .expect("Could not fetch message for Alice");

        assert_eq!(retrieved_messages.len(), 1);
        let alices_msg = retrieved_messages.first().unwrap();
        assert_eq!(alices_msg.sender_username, BOB_USERNAME);
        assert_eq!(alices_msg.recipient_username, ALICE_USERNAME);
        assert_eq!(
            alices_msg.message.ciphertext,
            "Hello, Alice".as_bytes().to_vec()
        )
    }

    #[tokio::test]
    async fn test_set_messages_delivered() {
        let acc_db = db_with_alice_and_bob().await;
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
                .send_message(
                    BOB_USERNAME.to_string(),
                    ALICE_USERNAME.to_string(),
                    new_message,
                )
                .await
                .expect("Could not send message");

            message_ids.push(receipt.message_id);
        }

        let retrieved = service
            .get_messages(ALICE_USERNAME)
            .await
            .expect("Could not fetch messages for Alice");

        let ids: Vec<Uuid> = retrieved.iter().map(|msg| msg.message_id).collect();
        let delivered = ids[5..10].to_vec();
        service
            .mark_delivered(ALICE_USERNAME, delivered.clone())
            .await
            .expect("Could not set messages delivered");

        let rest = service
            .get_messages(ALICE_USERNAME)
            .await
            .expect("Could not fetch messages for Alice");
        let rest_ids: Vec<Uuid> = rest.iter().map(|msg| msg.message_id).collect();

        // 15 messages left to mark as delivered
        assert_eq!(rest_ids.len(), 15);

        // rest_uuids shouldn't contain any from delivered
        assert!(!rest_ids.iter().any(|uuid| delivered.contains(uuid)));

        service
            .mark_delivered(ALICE_USERNAME, rest_ids)
            .await
            .expect("Could not set messages delivered");
        let final_messages = service
            .get_messages(ALICE_USERNAME)
            .await
            .expect("Could not fetch messages for Alice");
        assert_eq!(final_messages.len(), 0);
    }

    async fn db_with_alice_and_bob() -> InMemoryDatabase {
        let db = InMemoryDatabase::new();

        db.upsert_account(Account::new(
            ALICE_USERNAME.to_string(),
            "alice_auth_password",
            Some(ALICE_PUSH_TOKEN.to_vec()),
            None,
        ))
        .await
        .unwrap();

        db.upsert_account(Account::new(
            BOB_USERNAME.to_string(),
            "bob_auth_password",
            Some(BOB_PUSH_TOKEN.to_vec()),
            None,
        ))
        .await
        .unwrap();

        db
    }
}
