use axum::{
    Extension, Json, extract::State, middleware::from_fn_with_state, response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::entities::DoubleRatchetMessage;
use crate::{
    account::{auth::middleware::require_auth, entities::Account},
    context::AppContext,
    messages::entities::{Message, MessageReceipt},
};

const MESSAGING_TAG: &str = "messaging";

/// When sending a message, the sender includes a full double ratchet message.
/// The server attaches the sender's identity based on the auth token.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub recipient_id: Uuid,
    pub message: DoubleRatchetMessage,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkDeliveredRequest {
    pub message_ids: Vec<Uuid>,
}

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .routes(routes!(send_message))
        .routes(routes!(fetch_messages))
        .routes(routes!(mark_delivered))
        .layer(from_fn_with_state(context.clone(), require_auth))
}

#[utoipa::path(
    post,
    path = "/send",
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent successfully", body = MessageReceipt),
        (status = 400, description = "Bad rquest"),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn send_message(
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .messaging_service
        .send_message(account.id, req.recipient_id, req.message)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/get",
    responses(
        (status = 200, description = "Messages fetched successfully", body = Vec<Message>),
        (status = 400, description = "Bad rquest"),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn fetch_messages(
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .messaging_service
        .get_messages(account.id)
        .await
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/mark-delivered",
    request_body = MarkDeliveredRequest,
    responses(
        (status = 200, description = "Messages marked as delivered successfully", body = bool),
        (status = 400, description = "Bad rquest"),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn mark_delivered(
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
    Json(request): Json<MarkDeliveredRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .messaging_service
        .mark_delivered(account.id, request.message_ids)
        .await
        .map(Json)
}
