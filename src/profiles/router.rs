use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::{
    entities::{ProfilePictureUploadResponse, ProfileResponse, UpdateProfileRequest},
    error::ProfileError,
};
use crate::{
    account::{auth::middleware::require_auth, entities::Account},
    startup::AppContext,
};

const PROFILES_TAG: &str = "profiles";

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .routes(routes!(get_profile))
        .routes(routes!(get_profile_by_username))
        .routes(routes!(update_profile))
        .layer(from_fn_with_state(context.clone(), require_auth))
        .with_state(context)
}

#[utoipa::path(
    get,
    path = "/{account_id}",
    responses(
        (status = 200, description = "Profile fetched successfully", body = ProfileResponse),
        (status = 404, description = "Profile not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = PROFILES_TAG
)]
async fn get_profile(
    State(context): State<Arc<AppContext>>,
    Path(account_id): Path<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .profile_service
        .get_profile_by_account_id(account_id)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/by-username/{username}",
    responses(
        (status = 200, description = "Profile fetched successfully", body = ProfileResponse),
        (status = 404, description = "Profile not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = PROFILES_TAG
)]
async fn get_profile_by_username(
    State(context): State<Arc<AppContext>>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .profile_service
        .get_profile_by_username(&username)
        .await
        .map(Json)
}

#[utoipa::path(
    patch,
    path = "/",
    request_body = UpdateProfileRequest,
    responses(
        (status = 204, description = "Profile updated successfully (No Content)"),
        (status = 200, description = "Profile picture will be updated, contains upload URL", body = ProfilePictureUploadResponse),
        (status = 404, description = "Profile not found"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = PROFILES_TAG
)]
async fn update_profile(
    State(context): State<Arc<AppContext>>,
    Extension(account): Extension<Account>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<impl IntoResponse, ProfileError> {
    let upload_response = context
        .profile_service
        .update_profile(account.id, req)
        .await?;

    // Return 204 when no profile picture update, or 200 with upload info
    let Some(upload_info): Option<ProfilePictureUploadResponse> = upload_response else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };

    Ok(Json(upload_info).into_response())
}
