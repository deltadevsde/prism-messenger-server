use axum::{
    Extension,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::account::auth::middleware::require_auth;
use crate::account::entities::Account;
use crate::startup::AppContext;

use super::entities::PresenceStatus;

const PRESENCE_TAG: &str = "presence";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresenceStatusResponse {
    /// The presence status
    #[schema(example = "online")]
    pub status: PresenceStatus,
}

impl PresenceStatusResponse {
    pub fn new(status: PresenceStatus) -> Self {
        Self { status }
    }
}

pub fn router(context: Arc<AppContext>) -> OpenApiRouter<Arc<AppContext>> {
    OpenApiRouter::new()
        .routes(routes!(get_presence_status))
        .layer(from_fn_with_state(context.clone(), require_auth))
}

#[utoipa::path(
    get,
    path = "/{account_id}",
    tag = PRESENCE_TAG,
    params(("account_id" = Uuid, Path, description = "Account ID to get presence status for")),
    security(
        ("basic_auth" = [])
    ),
    responses(
        (status = 200, description = "Presence status retrieved successfully", body = PresenceStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_presence_status(
    Extension(_account): Extension<Account>,
    Path(account_id): Path<Uuid>,
    State(context): State<Arc<AppContext>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    context
        .presence_service
        .get_presence_status(&account_id)
        .await
        .map(PresenceStatusResponse::new)
        .map(Json)
}
