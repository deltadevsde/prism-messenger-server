use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::account::entities::{Account, AccountIdentity};
use crate::context::AppContext;

use super::auth::middleware::require_auth;

const ACCOUNTS_TAG: &str = "accounts";

#[serde_as]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApnsTokenUpdateRequest {
    /// The new APNS token
    #[schema(example = "dGhpcyBpcyBub3QgYSByZWFsIEFQTlMgdG9rZW4=")]
    #[serde_as(as = "Base64")]
    pub token: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountInfoResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,
}

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    let public_router = OpenApiRouter::new().routes(routes!(head_account));
    let auth_router = OpenApiRouter::new()
        .routes(routes!(get_account))
        .routes(routes!(update_apns_token))
        .layer(from_fn_with_state(context.clone(), require_auth));

    public_router.merge(auth_router)
}

#[utoipa::path(
    head,
    path = "/account/{username}",
    tag = ACCOUNTS_TAG,
    params(("username" = String, Path, description = "Username of the account")),
    responses(
        (status = 200, description = "Account exists"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal error while getting account")
    )
)]
async fn head_account(
    Path(username): Path<String>,
    State(context): State<Arc<AppContext>>,
) -> StatusCode {
    let identity = AccountIdentity::Username(username);
    let Ok(identity_exists) = context.account_service.identity_exists(&identity).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    match identity_exists {
        true => StatusCode::OK,
        false => StatusCode::NOT_FOUND,
    }
}

#[utoipa::path(
    get,
    path = "/account/{username}",
    tag = ACCOUNTS_TAG,
    params(("username" = String, Path, description = "Username of the account")),
    responses(
        (status = 200, description = "Account found", body = AccountInfoResponse),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal error while getting account")
    )
)]
async fn get_account(
    Path(username): Path<String>,
    State(context): State<Arc<AppContext>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let identity = AccountIdentity::Username(username);
    context
        .account_service
        .get_account_id_by_identity(&identity)
        .await
        .map(|id| AccountInfoResponse { id })
        .map(Json)
}

#[utoipa::path(
    put,
    path = "/apns",
    tag = ACCOUNTS_TAG,
    request_body = ApnsTokenUpdateRequest,
    security(
        ("basic_auth" = [])
    ),
    responses(
        (status = 200, description = "APNS token updated successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to update APNS token")
    )
)]
async fn update_apns_token(
    Extension(account): Extension<Account>,
    State(context): State<Arc<AppContext>>,
    Json(request): Json<ApnsTokenUpdateRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .account_service
        .update_apns_token(account.id, request.token)
        .await
        .map(|_| StatusCode::OK)
}
