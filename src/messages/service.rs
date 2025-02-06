use std::sync::Arc;

use anyhow::{anyhow, Result};

use super::{
    database::MessageDatabase,
    entities::{Message, SendMessageRequest, SendMessageResponse},
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
        user_id: &str,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = Message {
            message_id: uuid::Uuid::new_v4(),
            sender_id: user_id.to_string(),
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

    pub async fn mark_delivered(&self, user_id: &str, msg_ids: Vec<uuid::Uuid>) -> Result<bool> {
        self.db.mark_delivered(user_id.to_string(), msg_ids)
    }
}
