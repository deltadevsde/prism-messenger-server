use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::sync::{
    RwLock,
    mpsc::{self, error::SendError},
};
use tracing::warn;
use uuid::Uuid;

use crate::{
    messages::{
        entities::Message,
        gateway::{MessageGateway, MessageGatewayError},
    },
    presence::{
        database::PresenceDatabase,
        entities::PresenceStatus,
        error::PresenceError,
        gateway::{PresenceGateway, PresenceGatewayError, PresenceUpdate},
    },
};

/// Errors that can occur during WebSocket operations
#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("No WebSocket connection found for {0}")]
    ConnectionNotFound(String),
    #[error("Failed to serialize message: {0}")]
    SerializationFailed(String),
    #[error("Failed to send message: {0}")]
    SendingFailed(String),
}

impl From<SendError<Vec<u8>>> for WebSocketError {
    fn from(err: SendError<Vec<u8>>) -> Self {
        WebSocketError::SendingFailed(err.to_string())
    }
}

impl From<WebSocketError> for MessageGatewayError {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::ConnectionNotFound(account_id) => {
                MessageGatewayError::ConnectionNotFound(account_id.parse().unwrap_or_default())
            }
            WebSocketError::SerializationFailed(msg) => {
                MessageGatewayError::SerializationError(msg)
            }
            WebSocketError::SendingFailed(msg) => MessageGatewayError::SendFailed(msg),
        }
    }
}

impl From<WebSocketError> for PresenceGatewayError {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::ConnectionNotFound(account_id) => {
                PresenceGatewayError::ConnectionNotFound(account_id.parse().unwrap_or_default())
            }
            WebSocketError::SerializationFailed(msg) => {
                PresenceGatewayError::SerializationError(msg)
            }
            WebSocketError::SendingFailed(msg) => PresenceGatewayError::SendFailed(msg),
        }
    }
}

/// JSON struct representing a WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub data: serde_json::Value,
}

/// Represents a WebSocket connection for a specific account
#[derive(Debug)]
pub struct WebSocketConnection {
    pub account_id: Uuid,
    pub sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl WebSocketConnection {
    pub fn new(account_id: Uuid, sender: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        Self { account_id, sender }
    }

    /// Send binary data to the WebSocket connection
    pub fn send(&self, data: Vec<u8>) -> Result<(), WebSocketError> {
        self.sender.send(data)?;
        Ok(())
    }
}

type WebSocketHandler =
    Box<dyn Fn(Uuid, &serde_json::Value) -> Result<(), Box<dyn Error>> + Send + Sync>;
type DisconnectHandler = Box<dyn Fn(Uuid) -> Result<(), Box<dyn Error>> + Send + Sync>;
type ConnectHandler = Box<dyn Fn(Uuid) -> Result<(), Box<dyn Error>> + Send + Sync>;

#[derive(Clone)]
pub struct WebSocketCenter {
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnection>>>,
    handlers: Arc<RwLock<HashMap<String, WebSocketHandler>>>,
    disconnect_handlers: Arc<RwLock<Vec<DisconnectHandler>>>,
    connect_handlers: Arc<RwLock<Vec<ConnectHandler>>>,
}

impl WebSocketCenter {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            disconnect_handlers: Arc::new(RwLock::new(Vec::new())),
            connect_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn send_to_account<T>(
        &self,
        account_id: Uuid,
        message: &T,
    ) -> Result<(), WebSocketError>
    where
        T: Serialize + Send + Sync,
    {
        let data = serde_json::to_vec(message)
            .map_err(|e| WebSocketError::SerializationFailed(e.to_string()))?;

        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&account_id) {
            connection.send(data)
        } else {
            Err(WebSocketError::ConnectionNotFound(account_id.to_string()))
        }
    }

    /// Broadcast a message to all connected accounts
    pub async fn broadcast_to_all<T>(&self, message: &T) -> Result<(), WebSocketError>
    where
        T: Serialize + Send + Sync,
    {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if let Err(e) = self.send_to_account(connection.account_id, message).await {
                warn!(
                    "Failed to broadcast message to {}: {}",
                    connection.account_id, e
                );
            }
        }
        Ok(())
    }

    pub async fn register_handler<T, H>(&self, message_type: &str, handler: H)
    where
        T: serde::de::DeserializeOwned + 'static,
        H: Fn(Uuid, T) -> Result<(), Box<dyn Error>> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;

        let wrapper =
            move |sender_id: Uuid, value: &serde_json::Value| -> Result<(), Box<dyn Error>> {
                let typed_msg: T = serde_json::from_value(value.clone())?;
                handler(sender_id, typed_msg)
            };

        handlers.insert(message_type.to_string(), Box::new(wrapper));
    }

    /// Register a handler for when connections are disconnected
    pub async fn register_disconnect_handler<H>(&self, handler: H)
    where
        H: Fn(Uuid) -> Result<(), Box<dyn Error>> + Send + Sync + 'static,
    {
        let mut handlers = self.disconnect_handlers.write().await;
        handlers.push(Box::new(handler));
    }

    /// Register a handler for when connections are established
    pub async fn register_connect_handler<H>(&self, handler: H)
    where
        H: Fn(Uuid) -> Result<(), Box<dyn Error>> + Send + Sync + 'static,
    {
        let mut handlers = self.connect_handlers.write().await;
        handlers.push(Box::new(handler));
    }

    pub async fn has_connection(&self, account_id: &Uuid) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(account_id)
    }

    /// Add a new WebSocket connection for an account
    pub async fn add_connection(&self, account_id: Uuid, sender: mpsc::UnboundedSender<Vec<u8>>) {
        let connection = WebSocketConnection::new(account_id, sender);
        let mut connections = self.connections.write().await;
        connections.insert(account_id, connection);
        drop(connections); // Release the lock before calling handlers

        // Notify connect handlers
        let handlers = self.connect_handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler(account_id) {
                warn!("Connect handler failed for account {}: {}", account_id, e);
            }
        }
    }

    /// Remove a WebSocket connection for an account
    pub async fn remove_connection(&self, account_id: &Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(account_id);

        // Notify disconnect handlers
        let handlers = self.disconnect_handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler(*account_id) {
                warn!(
                    "Disconnect handler failed for account {}: {}",
                    account_id, e
                );
            }
        }
    }

    pub async fn on_message_received(&self, sender_id: Uuid, raw_data: &[u8]) {
        let Ok(msg) = serde_json::from_slice::<serde_json::Value>(raw_data) else {
            warn!("Failed to parse message from: {}", sender_id);
            return;
        };

        let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) else {
            warn!("Invalid message format from: {}", sender_id);
            return;
        };

        let handlers = self.handlers.read().await;

        let Some(handler) = handlers.get(msg_type) else {
            warn!("No handler for message type: {:?}", msg_type);
            return;
        };

        if let Err(e) = handler(sender_id, &msg) {
            warn!("Handler failed for message type '{}': {}", msg_type, e);
        }
    }
}

impl Default for WebSocketCenter {
    fn default() -> Self {
        Self::new()
    }
}

// Messages

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageWebSocketMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(flatten)]
    pub message: Message,
}

impl MessageWebSocketMessage {
    pub fn new(message: Message) -> Self {
        Self {
            message_type: "message".to_string(),
            message,
        }
    }
}

#[async_trait]
impl MessageGateway for WebSocketCenter {
    async fn send_message(&self, message: Message) -> Result<(), MessageGatewayError> {
        let recipient_id = message.recipient_id;
        let ws_message = MessageWebSocketMessage::new(message);
        self.send_to_account(recipient_id, &ws_message).await?;
        Ok(())
    }
}

// Presence

#[async_trait]
impl PresenceDatabase for WebSocketCenter {
    async fn is_present(&self, account_id: &Uuid) -> Result<bool, PresenceError> {
        Ok(self.has_connection(account_id).await)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresenceWebSocketMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub account_id: Uuid,
    pub status: String,
}

impl PresenceWebSocketMessage {
    pub fn new(presence_update: &PresenceUpdate) -> Self {
        let status = match presence_update.status {
            PresenceStatus::Online => "online".to_string(),
            PresenceStatus::Offline => "offline".to_string(),
        };

        Self {
            message_type: "presence".to_string(),
            account_id: presence_update.account_id,
            status,
        }
    }
}

#[async_trait]
impl PresenceGateway for WebSocketCenter {
    async fn send_presence_update(
        &self,
        presence_update: &PresenceUpdate,
    ) -> Result<(), PresenceGatewayError> {
        let ws_message = PresenceWebSocketMessage::new(presence_update);

        // For now, we'll broadcast to all connections
        // In a real implementation, you might want to send only to interested parties
        self.broadcast_to_all(&ws_message).await?;

        Ok(())
    }

    async fn register_presence_handler<H>(&self, handler: H)
    where
        H: Fn(PresenceUpdate) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);

        // Register connect handler for Online status
        {
            let handler = Arc::clone(&handler);
            self.register_connect_handler(move |account_id| {
                let presence_update = PresenceUpdate::new(account_id, PresenceStatus::Online);
                handler(presence_update);
                Ok(())
            })
            .await;
        }

        // Register disconnect handler for Offline status
        {
            let handler = Arc::clone(&handler);
            self.register_disconnect_handler(move |account_id| {
                let presence_update = PresenceUpdate::new(account_id, PresenceStatus::Offline);
                handler(presence_update);
                Ok(())
            })
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_send_to_account_with_serializable_message() {
        let center = WebSocketCenter::new();
        let account_id = Uuid::new_v4();

        // Create a mock sender channel
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Add a connection
        center.add_connection(account_id, tx).await;

        // Test sending a serializable message
        let test_message = json!({
            "type": "test",
            "data": "hello world"
        });

        let result = center.send_to_account(account_id, &test_message).await;
        assert!(result.is_ok());

        // Verify the message was sent
        let received_data = rx.try_recv().unwrap();
        let received_json: serde_json::Value = serde_json::from_slice(&received_data).unwrap();
        assert_eq!(received_json, test_message);
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_account() {
        let center = WebSocketCenter::new();
        let account_id = Uuid::new_v4();

        let test_message = json!({"test": "data"});
        let result = center.send_to_account(account_id, &test_message).await;

        assert!(matches!(result, Err(WebSocketError::ConnectionNotFound(_))));
    }

    #[tokio::test]
    async fn test_connection_management() {
        let center = WebSocketCenter::new();
        let account_id = Uuid::new_v4();

        // Initially no connection
        assert!(!center.has_connection(&account_id).await);

        // Add connection
        let (tx, _rx) = mpsc::unbounded_channel::<Vec<u8>>();
        center.add_connection(account_id, tx).await;

        // Should now have connection
        assert!(center.has_connection(&account_id).await);

        // Remove connection
        center.remove_connection(&account_id).await;

        // Should no longer have connection
        assert!(!center.has_connection(&account_id).await);
    }
}
