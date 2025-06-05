use anyhow::Result;
use axum::{Json, extract::State, response::IntoResponse};
use prism_client::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use uuid::Uuid;

use crate::context::AppContext;
use crate::registration::entities::RegistrationChallenge;

const REGISTRATION_TAG: &str = "username_registration";

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

#[serde_as]
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeRegistrationRequest {
    pub username: String,
    pub key: VerifyingKey,
    pub signature: Signature,
    #[schema(example = "MDEyMzQ1Njc4OWFiY2RlZg==")]
    pub auth_password: String,
    #[schema(example = "device-token-for-apns")]
    #[serde_as(as = "Option<Base64>")]
    pub apns_token: Option<Vec<u8>>,
    #[schema(example = "device-token-for-gcm")]
    #[serde_as(as = "Option<Base64>")]
    pub gcm_token: Option<Vec<u8>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeRegistrationResponse {
    pub id: Uuid,
}

pub fn router() -> OpenApiRouter<Arc<AppContext>> {
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
    State(context): State<Arc<AppContext>>,
    Json(req): Json<RequestRegistrationRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .username_registration_service
        .request_registration(req.username, req.key)
        .await
        .map(RequestRegistrationResponse::from)
        .map(Json)
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
    State(context): State<Arc<AppContext>>,
    Json(req): Json<FinalizeRegistrationRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .username_registration_service
        .finalize_registration(
            req.username,
            req.key,
            req.signature,
            &req.auth_password,
            req.apns_token,
            req.gcm_token,
        )
        .await
        .map(|new_acc| FinalizeRegistrationResponse { id: new_acc.id })
        .map(Json)
}
