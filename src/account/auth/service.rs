use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    account::{
        database::{AccountDatabase, AccountDatabaseError},
        entities::Account,
    },
    crypto::salted_hash::SaltedHashError,
};

pub struct AuthService<D: AccountDatabase> {
    // Repository for account data
    account_db: Arc<D>,
}

impl<D: AccountDatabase> AuthService<D> {
    pub fn new(account_db: Arc<D>) -> Self {
        Self { account_db }
    }

    /// Authenticates an account ID and password
    pub async fn authenticate(&self, id: &str, password: &str) -> Result<Account, AuthError> {
        let account_id = Uuid::parse_str(id)?;

        // Look up the account
        let account = self
            .account_db
            .fetch_account(account_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        // Verify the password against stored hash
        account.auth_password_hash.verify_password(password)?;
        Ok(account)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Processing auth failed")]
    ProcessingFailed,

    #[error("Database error: {0}")]
    DatabaseError(#[from] AccountDatabaseError),
}

impl From<uuid::Error> for AuthError {
    fn from(_: uuid::Error) -> Self {
        AuthError::InvalidCredentials
    }
}

impl From<SaltedHashError> for AuthError {
    fn from(err: SaltedHashError) -> Self {
        match err {
            SaltedHashError::InvalidPassword => AuthError::InvalidCredentials,
            SaltedHashError::HashParseError(_) => AuthError::ProcessingFailed,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AuthError::ProcessingFailed => StatusCode::INTERNAL_SERVER_ERROR,
            AuthError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}
