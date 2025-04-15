use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::{
    entities::{KeyBundle, Prekey},
    service::KeyBundleResponse,
};
use crate::{
    account::{auth::middleware::require_auth, entities::Account},
    context::AppContext,
};

const KEY_TAG: &str = "keys";

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadKeyBundleRequest {
    pub key_bundle: KeyBundle,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadPrekeysRequest {
    pub prekeys: Vec<Prekey>,
}

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .routes(routes!(post_keybundle))
        .routes(routes!(get_keybundle))
        .routes(routes!(post_prekeys))
        .layer(from_fn_with_state(context.clone(), require_auth))
}

#[utoipa::path(
    post,
    path = "/bundle",
    request_body = UploadKeyBundleRequest,
    responses(
        (status = 200, description = "Bundle upload successful"),
        (status = 500, description = "Bundle upload failed unexpectedly")
    ),
    tag = KEY_TAG
)]
async fn post_keybundle(
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
    Json(req): Json<UploadKeyBundleRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    context
        .key_service
        .upload_key_bundle(&account.username, req.key_bundle)
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
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
    Json(req): Json<UploadPrekeysRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    context
        .key_service
        .add_prekeys(&account.username, req.prekeys)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/bundle/{username}",
    params(
        ("username" = String, Path, description = "User identifier")
    ),
    responses(
        (status = 200, description = "Key bundle retrieved successfully", body = KeyBundleResponse),
        (status = 500, description = "Key bundle retrieval failed unexpectedly")
    ),
    tag = KEY_TAG
)]
async fn get_keybundle(
    State(context): State<Arc<AppContext>>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    context
        .key_service
        .get_keybundle(&username)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
