use argon2::{
    Argon2,
    password_hash::{
        Error as Argon2Error, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        rand_core::OsRng,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SaltedHash {
    hash: String,
}

impl SaltedHash {
    pub fn generate_from(password: &str) -> Self {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Error hashing password")
            .to_string();

        Self { hash }
    }

    pub fn verify_password(&self, password: &str) -> Result<(), SaltedHashError> {
        let parsed_hash = PasswordHash::new(&self.hash)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)?;
        Ok(())
    }
}

impl Display for SaltedHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hash)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaltedHashError {
    #[error("Password hash validation failed")]
    InvalidPassword,
    #[error("Failed to parse password hash {0}")]
    HashParseError(String),
}

impl From<Argon2Error> for SaltedHashError {
    fn from(err: Argon2Error) -> Self {
        match err {
            Argon2Error::Password => SaltedHashError::InvalidPassword,
            _ => SaltedHashError::HashParseError(format!("{:?}", err)),
        }
    }
}
