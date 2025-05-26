use anyhow::Result;
use axum::{Json, extract::State, response::IntoResponse};
use prism_client::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::entities::RegistrationChallenge;
use crate::context::AppContext;

const PHONE_REGISTRATION_TAG: &str = "phone_registration";

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestPhoneRegistrationRequest {
    pub phone_number: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestPhoneRegistrationResponse {
    pub success: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPhoneRegistrationRequest {
    pub phone_number: String,
    pub otp_code: String,
    pub key: VerifyingKey,
}

#[serde_as]
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPhoneRegistrationResponse {
    #[serde_as(as = "Base64")]
    pub challenge: Vec<u8>,
}

impl From<RegistrationChallenge> for VerifyPhoneRegistrationResponse {
    fn from(challenge: RegistrationChallenge) -> Self {
        Self {
            challenge: challenge.into_bytes(),
        }
    }
}

#[serde_as]
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinalizePhoneRegistrationRequest {
    pub phone_number: String,
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
pub struct FinalizePhoneRegistrationResponse {
    pub id: Uuid,
}

pub fn phone_router() -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .routes(routes!(post_request_phone_registration))
        .routes(routes!(post_verify_phone_registration))
        .routes(routes!(post_finalize_phone_registration))
}

#[utoipa::path(
    post,
    path = "/phone/request",
    request_body = RequestPhoneRegistrationRequest,
    responses(
        (status = 200, description = "OTP sent successfully", body = RequestPhoneRegistrationResponse),
        (status = 400, description = "Invalid phone number"),
        (status = 500, description = "Failed to send OTP")
    ),
    tag = PHONE_REGISTRATION_TAG
)]
async fn post_request_phone_registration(
    State(context): State<Arc<AppContext>>,
    Json(req): Json<RequestPhoneRegistrationRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .phone_registration_service
        .request_phone_registration(req.phone_number)
        .await
        .map(|_| Json(RequestPhoneRegistrationResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/phone/verify",
    request_body = VerifyPhoneRegistrationRequest,
    responses(
        (status = 200, description = "OTP verified successfully", body = VerifyPhoneRegistrationResponse),
        (status = 400, description = "Invalid OTP or phone number"),
        (status = 500, description = "Verification failed")
    ),
    tag = PHONE_REGISTRATION_TAG
)]
async fn post_verify_phone_registration(
    State(context): State<Arc<AppContext>>,
    Json(req): Json<VerifyPhoneRegistrationRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .phone_registration_service
        .verify_phone_registration(req.phone_number, req.otp_code, req.key)
        .await
        .map(VerifyPhoneRegistrationResponse::from)
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/phone/finalize",
    request_body = FinalizePhoneRegistrationRequest,
    responses(
        (status = 200, description = "Phone registration completed successfully", body = FinalizePhoneRegistrationResponse),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Registration failed")
    ),
    tag = PHONE_REGISTRATION_TAG
)]
async fn post_finalize_phone_registration(
    State(context): State<Arc<AppContext>>,
    Json(req): Json<FinalizePhoneRegistrationRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .phone_registration_service
        .finalize_phone_registration(
            req.phone_number,
            req.key,
            req.signature,
            &req.auth_password,
            req.apns_token,
            req.gcm_token,
        )
        .await
        .map(|new_acc| FinalizePhoneRegistrationResponse { id: new_acc.id })
        .map(Json)
}