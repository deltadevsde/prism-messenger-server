use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::{
    entities::{KeyBundle, Prekey},
    service::KeyBundleResponse,
};
use crate::state::AppState;

const KEY_TAG: &str = "keys";

#[derive(Deserialize, ToSchema)]
pub struct UploadKeyBundleRequest {
    pub user_id: String,
    pub keybundle: KeyBundle,
}

#[derive(Deserialize, ToSchema)]
pub struct UploadPrekeysRequest {
    pub user_id: String,
    pub prekeys: Vec<Prekey>,
}

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(post_keybundle))
        .routes(routes!(get_keybundle))
        .routes(routes!(post_prekeys))
}

#[utoipa::path(
    post,
    path = "/upload_bundle",
    request_body = UploadKeyBundleRequest,
    responses(
        (status = 200, description = "Bundle upload successful"),
        (status = 500, description = "Bundle upload failed unexpectedly")
    ),
    tag = KEY_TAG
)]
async fn post_keybundle(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadKeyBundleRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .key_service
        .upload_key_bundle(&req.user_id, req.keybundle)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/upload_prekeys",
    request_body = UploadPrekeysRequest,
    responses(
        (status = 200, description = "Prekey upload successful"),
        (status = 500, description = "Prekey upload failed unexpectedly")
    ),
    tag = KEY_TAG
)]
async fn post_prekeys(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadPrekeysRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .key_service
        .add_prekeys(&req.user_id, req.prekeys)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/bundle/{user_id}",
    params(
        ("user_id" = String, Path, description = "User identifier")
    ),
    responses(
        (status = 200, description = "Key bundle retrieved successfully", body = KeyBundleResponse),
        (status = 500, description = "Key bundle retrieval failed unexpectedly")
    ),
    tag = KEY_TAG
)]
async fn get_keybundle(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .key_service
        .get_keybundle(&user_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
