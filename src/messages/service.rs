use anyhow::{Result, anyhow};
use std::sync::Arc;

use super::{
    database::MessageDatabase,
    entities::{MarkDeliveredRequest, Message, SendMessageRequest, SendMessageResponse},
};

pub struct MessagingService<D: MessageDatabase> {
    db: Arc<D>,
}

impl<D: MessageDatabase> MessagingService<D> {
    pub fn new(db: Arc<D>) -> MessagingService<D> {
        MessagingService { db }
    }

    pub async fn send_message(&self, request: SendMessageRequest) -> Result<SendMessageResponse> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: uuid::Uuid::new_v4(),
            sender_id: request.sender_id.clone(),
            recipient_id: request.recipient_id.clone(),
            message: request.message.clone(),
            timestamp,
        };

        let success = self.db.insert_message(message.clone())?;
        match success {
            true => Ok(SendMessageResponse {
                message_id: message.message_id,
                timestamp,
            }),
            false => Err(anyhow!("Failed to send message")),
        }
    }

    pub async fn get_messages(&self, user_id: &str) -> Result<Vec<Message>> {
        self.db.get_messages(user_id.to_string())
    }

    pub async fn mark_delivered(&self, request: MarkDeliveredRequest) -> Result<bool> {
        self.db
            .mark_delivered(request.user_id.to_string(), request.message_ids)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use prism_client::SigningKey;
    use uuid::Uuid;

    use crate::database::inmemory::InMemoryDatabase;

    use crate::messages::entities::{
        DoubleRatchetHeader, DoubleRatchetMessage, MarkDeliveredRequest, SendMessageRequest,
    };

    use super::MessagingService;

    fn init_service() -> Arc<MessagingService<InMemoryDatabase>> {
        let db = Arc::new(InMemoryDatabase::new());
        Arc::new(MessagingService::new(db))
    }

    #[tokio::test]
    async fn test_send_and_get_message() {
        let service = init_service();
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

        let request = SendMessageRequest {
            sender_id: "Bob".to_string(),
            recipient_id: "Alice".to_string(),
            message,
        };

        service
            .send_message(request)
            .await
            .expect("Could not send Bob's message to Alice");

        let retrieved_messages = service
            .get_messages("Alice")
            .await
            .expect("Could not fetch message for Alice");

        assert_eq!(retrieved_messages.len(), 1);
        let alices_msg = retrieved_messages.first().unwrap();
        assert_eq!(alices_msg.sender_id, "Bob");
        assert_eq!(alices_msg.recipient_id, "Alice");
        assert_eq!(
            alices_msg.message.ciphertext,
            "Hello, Alice".as_bytes().to_vec()
        )
    }

    #[tokio::test]
    async fn test_set_messages_delivered() {
        let service = init_service();
        let bob_ephemeral_key = SigningKey::new_secp256r1().verifying_key();

        let mut requests: Vec<SendMessageRequest> = Vec::new();
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

            let new_request = SendMessageRequest {
                sender_id: "Bob".to_string(),
                recipient_id: "Alice".to_string(),
                message: new_message,
            };
            requests.push(new_request);
        }

        for request in requests {
            service
                .send_message(request)
                .await
                .expect("Could not send message");
        }

        let retrieved = service
            .get_messages("Alice")
            .await
            .expect("Could not fetch messages for Alice");

        let uuids: Vec<Uuid> = retrieved.iter().map(|msg| msg.message_id).collect();
        let delivered = uuids[5..10].to_vec();
        service
            .mark_delivered(MarkDeliveredRequest {
                user_id: "Alice".to_string(),
                message_ids: delivered.clone(),
            })
            .await
            .expect("Could not set messages delivered");

        let rest = service
            .get_messages("Alice")
            .await
            .expect("Could not fetch messages for Alice");
        let rest_uuids: Vec<Uuid> = rest.iter().map(|msg| msg.message_id).collect();

        // 15 messages left to mark as delivered
        assert_eq!(rest_uuids.len(), 15);

        // rest_uuids shouldn't contain any from delivered
        assert!(!rest_uuids.iter().any(|uuid| delivered.contains(uuid)));

        service
            .mark_delivered(MarkDeliveredRequest {
                user_id: "Alice".to_string(),
                message_ids: rest_uuids,
            })
            .await
            .expect("Could not set messages delivered");
        let final_messages = service
            .get_messages("Alice")
            .await
            .expect("Could not fetch messages for Alice");
        assert_eq!(final_messages.len(), 0);
    }
}
