use std::sync::Arc;

use anyhow::Result;
use prism_keys::VerifyingKey;

use crate::database::Database;

/// The header provides the recipient with the context needed to update
/// its ratchet state. It includes the sender’s ephemeral public key and
/// message counters.
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
    pub message_id: u64,
    /// Server timestamp (epoch milliseconds)
    pub timestamp: u64,
}

/// The message delivered to a client includes sender/recipient metadata.
pub struct Message {
    pub message_id: u64,
    pub sender_id: String,
    pub recipient_id: String,
    pub message: DoubleRatchetMessage,
    pub timestamp: u64,
}

pub struct MessagingService {
    db: Arc<dyn Database>,
}

impl MessagingService {
    pub fn new(db: Arc<dyn Database>) -> MessagingService {
        MessagingService { db }
    }

    pub async fn send_message(&self, request: SendMessageRequest) -> Result<SendMessageResponse> {
        unimplemented!()
    }

    pub async fn get_messages(&self, user_id: String) -> Result<Vec<Message>> {
        // TODO: When do messages get cleared from the server? How does the client tell the server which messages it's missing?
        unimplemented!()
    }
}
