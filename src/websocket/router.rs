use axum::{
    Extension,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use log::debug;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, instrument, trace, warn};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use super::center::WebSocketCenter;
use crate::{
    account::{auth::middleware::require_auth, entities::Account},
    startup::AppContext,
};

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .route("/", get(websocket_handler))
        .layer(from_fn_with_state(context.clone(), require_auth))
}

#[instrument(skip(ws_upgrade, context))]
async fn websocket_handler(
    ws_upgrade: WebSocketUpgrade,
    Extension(account): Extension<Account>,
    State(context): State<Arc<AppContext>>,
) -> Result<Response, StatusCode> {
    // Upgrade the connection to WebSocket
    Ok(ws_upgrade.on_upgrade(move |socket| {
        handle_websocket_connection(socket, account.id, context.websocket_center.clone())
    }))
}

#[instrument(skip(socket, websocket_center))]
async fn handle_websocket_connection(
    socket: WebSocket,
    account_id: Uuid,
    websocket_center: Arc<WebSocketCenter>,
) {
    // Split the WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a channel for sending messages to the WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Add the connection to the WebSocket center
    websocket_center.add_connection(account_id, tx).await;

    // Spawn a task to handle outgoing messages (from the channel to WebSocket)
    let outgoing_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Binary(message.into())).await {
                error!("Failed to send message to WebSocket: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages from WebSocket
    let websocket_center_clone = websocket_center.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Binary(data)) => {
                    websocket_center_clone
                        .on_message_received(account_id, &data)
                        .await;
                }
                Ok(Message::Text(text)) => {
                    websocket_center_clone
                        .on_message_received(account_id, text.as_bytes())
                        .await;
                }
                Ok(Message::Close(_)) => {
                    // Abort the loop when the connection is closed
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // Is handled automatically by axum
                    continue;
                }
                Ok(Message::Pong(_)) => {
                    // Pong received, nothing to do
                    continue;
                }
                Err(e) => {
                    warn!("WebSocket error for account {}: {}", account_id, e);
                    break;
                }
            }
        }

        // Remove the connection when the WebSocket closes
        websocket_center_clone.remove_connection(&account_id).await;
    });

    // Wait for either task to complete (connection closed or error)
    tokio::select! {
        _ = outgoing_task => {
            trace!("Outgoing task completed for account {}", account_id);
        }
        _ = incoming_task => {
            trace!("Incoming task completed for account {}", account_id);
        }
    }

    // Ensure the connection is removed
    websocket_center.remove_connection(&account_id).await;
    debug!("WebSocket connection closed for account {}", account_id);
}
