use axum::{
    Extension, Json, extract::State, http::StatusCode, middleware::from_fn_with_state,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::entities::DoubleRatchetMessage;
use crate::{
    account::{auth::middleware::require_auth, entities::Account},
    messages::entities::{Message, MessageReceipt},
    state::AppState,
};

const MESSAGING_TAG: &str = "messaging";

/// When sending a message, the sender includes a full double ratchet message.
/// The server attaches the sender's identity based on the auth token.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub recipient_username: String,
    pub message: DoubleRatchetMessage,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkDeliveredRequest {
    pub message_ids: Vec<Uuid>,
}

pub fn router(state: Arc<AppState>) -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(send_message))
        .routes(routes!(fetch_messages))
        .routes(routes!(mark_delivered))
        .layer(from_fn_with_state(state.clone(), require_auth))
}

#[utoipa::path(
    post,
    path = "/send",
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent successfully", body = MessageReceipt),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn send_message(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .send_message(account.username, req.recipient_username, req.message)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(
    get,
    path = "/get",
    responses(
        (status = 200, description = "Messages fetched successfully", body = Vec<Message>),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn fetch_messages(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .get_messages(&account.username)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(
    post,
    path = "/mark-delivered",
    request_body = MarkDeliveredRequest,
    responses(
        (status = 200, description = "Messages marked as delivered successfully", body = bool),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn mark_delivered(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Json(request): Json<MarkDeliveredRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .mark_delivered(&account.username, request.message_ids)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
