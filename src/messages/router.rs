use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::entities::{MarkDeliveredRequest, SendMessageRequest};
use crate::{
    messages::entities::{Message, SendMessageResponse},
    state::AppState,
};

const MESSAGING_TAG: &str = "messaging";

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(send_message))
        .routes(routes!(fetch_messages))
        .routes(routes!(mark_delivered))
}

#[utoipa::path(
    post,
    path = "/send",
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent successfully", body = SendMessageResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .send_message(req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(
    get,
    path = "/get/{user_id}",
    responses(
        (status = 200, description = "Messages fetched successfully", body = Vec<Message>),
        (status = 500, description = "Internal server error")
    ),
    tag = MESSAGING_TAG
)]
async fn fetch_messages(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .get_messages(&user_id)
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
    Json(request): Json<MarkDeliveredRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .messaging_service
        .mark_delivered(request)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
