use anyhow::Result;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use prism_client::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

use super::entities::RegistrationChallenge;

const REGISTRATION_TAG: &str = "registration";

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestRegistrationRequest {
    pub username: String,
    pub key: VerifyingKey,
}

#[serde_as]
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestRegistrationResponse {
    #[serde_as(as = "Base64")]
    pub challenge: Vec<u8>,
}

impl From<RegistrationChallenge> for RequestRegistrationResponse {
    fn from(challenge: RegistrationChallenge) -> Self {
        Self {
            challenge: challenge.into_bytes(),
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeRegistrationRequest {
    pub username: String,
    pub key: VerifyingKey,
    pub signature: Signature,
}

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(post_request_registration))
        .routes(routes!(post_finalize_registration))
}

#[utoipa::path(
    post,
    path = "/request",
    request_body = RequestRegistrationRequest,
    responses(
        (status = 200, description = "Registration requested successfully", body = RequestRegistrationResponse),
        (status = 500, description = "Registration request failed on server-side")
    ),
    tag = REGISTRATION_TAG
)]
async fn post_request_registration(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RequestRegistrationRequest>,
) -> Result<Json<RequestRegistrationResponse>, StatusCode> {
    let challenge = state
        .registration_service
        .request_registration(req.username, req.key)
        .await?;
    Ok(Json(challenge.into()))
}

#[utoipa::path(
    post,
    path = "/finalize",
    request_body = FinalizeRegistrationRequest,
    responses(
        (status = 200, description = "Registered successfully"),
        (status = 500, description = "Registration failed on server-side")
    ),
    tag = REGISTRATION_TAG
)]
async fn post_finalize_registration(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinalizeRegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .registration_service
        .finalize_registration(req.username, req.key, req.signature)
        .await?;
    Ok(())
}
