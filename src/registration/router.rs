use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use prism_keys::VerifyingKey;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

const REGISTRATION_TAG: &str = "registration";

#[derive(Deserialize, ToSchema)]
pub struct RegistrationRequest {
    pub username: String,
    pub key: VerifyingKey,
}

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(post_registration))
}

#[utoipa::path(
    post,
    path = "/register",
    request_body = RegistrationRequest,
    responses(
        (status = 200, description = "Registered successfully"),
        (status = 500, description = "Registration failed on server-side")
    ),
    tag = REGISTRATION_TAG
)]
async fn post_registration(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .registration_service
        .create_account(req.username, req.key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}
