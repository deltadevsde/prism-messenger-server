use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::sync::Arc;

use super::header::AuthHeader;
use crate::state::AppState;

// Basic Auth middleware
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // Extract the Authorization header
    let auth_header_str = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse Basic auth credentials
    let auth_header = AuthHeader::parse(auth_header_str).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify credentials against database
    let Ok(authenticated_account) = state
        .auth_service
        .authenticate(&auth_header.username, &auth_header.password)
        .await
    else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    request.extensions_mut().insert(authenticated_account);

    // Pass the request to the next handler
    Ok(next.run(request).await)
}
