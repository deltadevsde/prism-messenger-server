use std::sync::Arc;

use anyhow::{anyhow, Result};
use prism_keys::VerifyingKey;

use crate::database::{inmemory::InMemoryDatabase, Database};

/// The header provides the recipient with the context needed to update
/// its ratchet state. It includes the sender’s ephemeral public key and
/// message counters.
#[derive(Clone)]
pub struct DoubleRatchetHeader {
    /// Sender's ephemeral DH public key for this message
    pub ephemeral_key: VerifyingKey,
    /// Message counter within the current chain
    pub message_number: u64,
    /// Last message number of the previous chain (for skipped keys)
    pub previous_message_number: u64,
    /// Identifier of the one-time prekey used in the handshake
    pub one_time_prekey_id: Option<u64>,
}

/// The complete double ratchet message.
/// The header is bound to the ciphertext via the AEAD process.
#[derive(Clone)]
pub struct DoubleRatchetMessage {
    pub header: DoubleRatchetHeader,
    /// AEAD-encrypted payload (includes authentication tag)
    pub ciphertext: Vec<u8>,
}

/// When sending a message, the sender includes a full double ratchet message.
/// The server attaches the sender's identity based on the auth token.
// TODO: How do we authenticate
pub struct SendMessageRequest {
    pub recipient_id: String,
    pub message: DoubleRatchetMessage,
}

pub struct SendMessageResponse {
    /// UUID
    pub message_id: uuid::Uuid,
    /// Server timestamp (epoch milliseconds)
    pub timestamp: u64,
}

/// The message delivered to a client includes sender/recipient metadata.
#[derive(Clone)]
pub struct Message {
    pub message_id: uuid::Uuid,
    pub sender_id: String,
    pub recipient_id: String,
    pub message: DoubleRatchetMessage,
    pub timestamp: u64,
}

pub struct MessagingService {
    db: Arc<InMemoryDatabase>,
}

impl MessagingService {
    pub fn new(db: Arc<InMemoryDatabase>) -> MessagingService {
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
