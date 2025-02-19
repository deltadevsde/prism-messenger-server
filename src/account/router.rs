use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

const ACCOUNTS_TAG: &str = "accounts";

pub fn router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(head_account))
}

#[utoipa::path(
    head,
    path = "/accounts/{username}",
    tag = ACCOUNTS_TAG,
    params(("id" = String, Path, description = "Account identifier")),
    responses(
        (status = 200, description = "Account exists"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal error while getting account")
    )
)]
async fn head_account(
    Path(username): Path<String>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    let Ok(username_exists) = state.account_service.username_exists(&username).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    match username_exists {
        true => StatusCode::OK,
        false => StatusCode::NOT_FOUND,
    }
}
