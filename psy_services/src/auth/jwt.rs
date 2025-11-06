use std::fmt;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (service or client identifier)
    pub sub: String,
    /// Expiration time (as UTC timestamp)
    pub exp: i64,
    /// Issued at (as UTC timestamp)
    pub iat: i64,
    /// Optional service name
    pub service: Option<String>,
}

impl Claims {
    /// Create new claims with defaults
    pub fn new(sub: String, service: Option<String>, expiration_hours: u32) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours as i64);

        Self {
            sub,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            service,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }
}

/// JWT Manager using shared secret from .env
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,

    validation: Validation,
}

impl JwtManager {
    /// Create a new JWT manager with the shared secret from .env
    pub fn new(secret: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.set_required_spec_claims(&["exp", "sub"]);

        Self {
            encoding_key,
            decoding_key,
            validation,
        }
    }

    /// Create a new JWT manager from environment variable
    pub fn from_env() -> Result<Self, AuthError> {
        dotenv::dotenv().ok();
        let secret = std::env::var("JWT_SECRET").map_err(|_| {
            tracing::error!("JWT_SECRET not found in environment variables or .env file");
            AuthError::TokenCreationError
        })?;

        if secret.len() < 32 {
            tracing::warn!("JWT_SECRET is less than 32 characters. Consider using a longer secret for production.");
        }

        Ok(Self::new(&secret))
    }

    /// Generate a JWT token (clients with the same secret can use this too)
    pub fn generate_token(&self, claims: &Claims) -> Result<String, AuthError> {
        encode(&Header::default(), claims, &self.encoding_key).map_err(|e| {
            tracing::error!("Failed to encode token: {}", e);
            AuthError::TokenCreationError
        })
    }

    /// Generate a simple service token
    pub fn generate_service_token(&self, service_name: &str, expiration_hours: u32) -> Result<String, AuthError> {
        let claims = Claims::new(service_name.to_string(), Some(service_name.to_string()), expiration_hours);

        self.generate_token(&claims)
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>, AuthError> {
        let insecure_decode = decode::<Claims>(token, &self.decoding_key, &Validation::new(jsonwebtoken::Algorithm::HS256));

        match insecure_decode {
            Ok(token_data) => {
                if token_data.claims.is_expired() {
                    return Err(AuthError::ExpiredToken);
                }
                Ok(token_data)
            }
            Err(err) => {
                tracing::debug!("Token validation failed: {:?}", err);
                match err.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(AuthError::ExpiredToken),
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => Err(AuthError::InvalidSignature),
                    _ => Err(AuthError::InvalidToken),
                }
            }
        }
    }

    /// Extract token from Authorization header
    pub fn extract_token_from_header(auth_header: &str) -> Option<String> {
        if auth_header.starts_with("Bearer ") {
            Some(auth_header[7..].to_string())
        } else {
            None
        }
    }
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidToken,
    ExpiredToken,
    MissingToken,
    InvalidSignature,
    TokenCreationError,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::InvalidToken => write!(f, "Invalid token"),
            AuthError::ExpiredToken => write!(f, "Token has expired"),
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InvalidSignature => write!(f, "Invalid token signature"),
            AuthError::TokenCreationError => write!(f, "Failed to create token"),
        }
    }
}

impl std::error::Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_validation() {
        // Set up test secret
        std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");

        let manager = JwtManager::from_env().unwrap();
        let claims = Claims::new("test-service".to_string(), Some("telemetry".to_string()), 1);

        let token = manager.generate_token(&claims).unwrap();
        assert!(!token.is_empty());

        let decoded = manager.validate_token(&token).unwrap();
        assert_eq!(decoded.claims.sub, "test-service");
        assert_eq!(decoded.claims.service, Some("telemetry".to_string()));
    }

    #[test]
    fn test_expired_token() {
        std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");

        let manager = JwtManager::from_env().unwrap();
        let mut claims = Claims::new("test".to_string(), None, 0);
        claims.exp = Utc::now().timestamp() - 3600; // Expired 1 hour ago

        let token = manager.generate_token(&claims).unwrap();
        let result = manager.validate_token(&token);

        assert!(matches!(result, Err(AuthError::ExpiredToken)));
    }
}
