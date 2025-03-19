use prism_client::VerifyingKey;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The header provides the recipient with the context needed to update
/// its ratchet state. It includes the sender’s ephemeral public key and
/// message counters.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoubleRatchetMessage {
    pub header: DoubleRatchetHeader,
    /// AEAD-encrypted payload (includes authentication tag)
    pub ciphertext: Vec<u8>,
}

/// When sending a message, the sender includes a full double ratchet message.
/// The server attaches the sender's identity based on the auth token.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// User ID - TODO: Should come from auth, not request body
    pub sender_id: String,
    pub recipient_id: String,
    pub message: DoubleRatchetMessage,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    /// UUID
    pub message_id: uuid::Uuid,
    /// Server timestamp (epoch milliseconds)
    pub timestamp: u64,
}

/// The message delivered to a client includes sender/recipient metadata.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: uuid::Uuid,
    pub sender_id: String,
    pub recipient_id: String,
    pub message: DoubleRatchetMessage,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkDeliveredRequest {
    /// User ID - TODO: Should come from auth, not request body
    pub user_id: String,
    pub message_ids: Vec<uuid::Uuid>,
}
