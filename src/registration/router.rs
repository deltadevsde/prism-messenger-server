use anyhow::Result;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use prism_client::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{account::auth::header::AuthHeader, state::AppState};

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
    headers: HeaderMap,
    Json(req): Json<FinalizeRegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Basic auth header is used to set the new account's auth token
    let auth_header_str = headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let auth_header = AuthHeader::parse(auth_header_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if auth_header.username != req.username {
        return Err(StatusCode::BAD_REQUEST);
    }

    match state
        .registration_service
        .finalize_registration(req.username, req.key, req.signature, &auth_header.password)
        .await
    {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to finalize registration: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    Ok(())
}
