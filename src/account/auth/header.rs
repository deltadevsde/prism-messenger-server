use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug)]
pub struct AuthHeader {
    pub username: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthHeaderError {
    #[error("Invalid Basic auth format")]
    InvalidFormat,
    #[error("Failed to decode Base64")]
    Base64Error,
    #[error("Invalid UTF-8 in credentials")]
    Utf8Error,
    #[error("Missing username or password")]
    MissingCredentials,
}

impl AuthHeader {
    /// Parse HTTP Basic Authentication header into AuthHeader struct
    pub fn parse(auth_header: &str) -> Result<Self, AuthHeaderError> {
        // Check if it's Basic auth
        if !auth_header.starts_with("Basic ") {
            return Err(AuthHeaderError::InvalidFormat);
        }

        // Decode the Base64 credentials
        let credentials = auth_header.trim_start_matches("Basic ");
        let decoded = BASE64
            .decode(credentials)
            .map_err(|_| AuthHeaderError::Base64Error)?;

        let decoded_str = String::from_utf8(decoded).map_err(|_| AuthHeaderError::Utf8Error)?;

        // Split username and password
        let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AuthHeaderError::MissingCredentials);
        }

        Ok(AuthHeader {
            username: parts[0].to_string(),
            password: parts[1].to_string(),
        })
    }
}
