use std::str::FromStr;

pub use tracing::Level;
use tracing_subscriber::{prelude::*, EnvFilter};

use chrono::{Duration, Utc};

#[cfg(not(target_arch = "wasm32"))]
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

pub const JWT_COMPANY: &str = "QEDProtocol";
pub const JWT_SUBJECT: &str = "qedlang-rust";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub company: String,
    pub sub: String,
    pub realm_id: u64,
    pub exp: i64,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn generate_jwt_token(
    secret_key: &str,
    realm_id: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now() + Duration::seconds(604800);
    let claims = Claims {
        company: JWT_COMPANY.to_string(),
        sub: JWT_SUBJECT.to_string(),
        realm_id: realm_id,
        exp: expiration.timestamp(),
    };

    let header = Header::default();
    let encoding_key = EncodingKey::from_secret(secret_key.as_bytes());

    encode(&header, &claims, &encoding_key)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn decrypt_jwt_token(
    secret_key: &str,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_secret(secret_key.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

pub fn setup_logging(log_level: String) -> anyhow::Result<()> {
    let log_level = Level::from_str(&log_level).unwrap_or(Level::INFO);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}", log_level)));

    let fmt_layer = if log_level < Level::INFO {
        tracing_subscriber::fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_ansi(true)
    } else {
        tracing_subscriber::fmt::layer()
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_target(false)
            .with_ansi(true)
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    #[test]
    fn test_jwt_token() {
        let secret_key = "qed-jwt-sk-test";
        let realm_id = 1;
        let token = super::generate_jwt_token(secret_key, realm_id).unwrap();
        let claims = super::decrypt_jwt_token(secret_key, &token).unwrap();
        assert_eq!(claims.company, "QEDProtocol");
        assert_eq!(claims.sub, "qedlang-rust");
        assert_eq!(claims.realm_id, realm_id);
    }
}
