use anyhow::{Result, anyhow};
use std::sync::Arc;
use uuid::Uuid;

use super::{
    database::MessageDatabase,
    entities::{DoubleRatchetMessage, Message, MessageReceipt},
};

pub struct MessagingService<D: MessageDatabase> {
    db: Arc<D>,
}

impl<D: MessageDatabase> MessagingService<D> {
    pub fn new(db: Arc<D>) -> MessagingService<D> {
        MessagingService { db }
    }

    pub async fn send_message(
        &self,
        sender_username: String,
        recipient_username: String,
        message: DoubleRatchetMessage,
    ) -> Result<MessageReceipt> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: uuid::Uuid::new_v4(),
            sender_username,
            recipient_username,
            message: message.clone(),
            timestamp,
        };

        let success = self.db.insert_message(message.clone())?;
        match success {
            true => Ok(MessageReceipt {
                message_id: message.message_id,
                timestamp,
            }),
            false => Err(anyhow!("Failed to send message")),
        }
    }

    pub async fn get_messages(&self, username: &str) -> Result<Vec<Message>> {
        self.db.get_messages(username)
    }

    pub async fn mark_delivered(&self, username: &str, message_ids: Vec<Uuid>) -> Result<bool> {
        self.db.mark_delivered(username, message_ids)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use prism_client::SigningKey;
    use uuid::Uuid;

    use crate::database::inmemory::InMemoryDatabase;

    use crate::messages::entities::{DoubleRatchetHeader, DoubleRatchetMessage};

    use super::MessagingService;

    fn init_service() -> Arc<MessagingService<InMemoryDatabase>> {
        let db = Arc::new(InMemoryDatabase::new());
        Arc::new(MessagingService::new(db))
    }

    #[tokio::test]
    async fn test_send_and_get_message() {
        let service = init_service();
        let alice_username = "alice";
        let bob_username = "bob";
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
                bob_username.to_string(),
                alice_username.to_string(),
                message,
            )
            .await
            .expect("Could not send Bob's message to Alice");

        let retrieved_messages = service
            .get_messages(alice_username)
            .await
            .expect("Could not fetch message for Alice");

        assert_eq!(retrieved_messages.len(), 1);
        let alices_msg = retrieved_messages.first().unwrap();
        assert_eq!(alices_msg.sender_username, bob_username);
        assert_eq!(alices_msg.recipient_username, alice_username);
        assert_eq!(
            alices_msg.message.ciphertext,
            "Hello, Alice".as_bytes().to_vec()
        )
    }

    #[tokio::test]
    async fn test_set_messages_delivered() {
        let service = init_service();
        let bob_ephemeral_key = SigningKey::new_secp256r1().verifying_key();

        let alice_username = "alice";
        let bob_username = "bob";

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
                    bob_username.to_string(),
                    alice_username.to_string(),
                    new_message,
                )
                .await
                .expect("Could not send message");

            message_ids.push(receipt.message_id);
        }

        let retrieved = service
            .get_messages(alice_username)
            .await
            .expect("Could not fetch messages for Alice");

        let ids: Vec<Uuid> = retrieved.iter().map(|msg| msg.message_id).collect();
        let delivered = ids[5..10].to_vec();
        service
            .mark_delivered(alice_username, delivered.clone())
            .await
            .expect("Could not set messages delivered");

        let rest = service
            .get_messages(alice_username)
            .await
            .expect("Could not fetch messages for Alice");
        let rest_ids: Vec<Uuid> = rest.iter().map(|msg| msg.message_id).collect();

        // 15 messages left to mark as delivered
        assert_eq!(rest_ids.len(), 15);

        // rest_uuids shouldn't contain any from delivered
        assert!(!rest_ids.iter().any(|uuid| delivered.contains(uuid)));

        service
            .mark_delivered(alice_username, rest_ids)
            .await
            .expect("Could not set messages delivered");
        let final_messages = service
            .get_messages(alice_username)
            .await
            .expect("Could not fetch messages for Alice");
        assert_eq!(final_messages.len(), 0);
    }
}
