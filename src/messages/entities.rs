use prism_client::VerifyingKey;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use utoipa::ToSchema;
use uuid::Uuid;

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
#[serde_as]
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoubleRatchetMessage {
    pub header: DoubleRatchetHeader,
    /// AEAD-encrypted payload (includes authentication tag)
    #[serde_as(as = "Base64")]
    pub ciphertext: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageReceipt {
    /// UUID
    pub message_id: Uuid,
    /// Server timestamp (epoch milliseconds)
    pub timestamp: u64,
}

/// The message delivered to a client includes sender/recipient metadata.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: uuid::Uuid,
    pub sender_username: String,
    pub recipient_username: String,
    pub message: DoubleRatchetMessage,
    pub timestamp: u64,
}
