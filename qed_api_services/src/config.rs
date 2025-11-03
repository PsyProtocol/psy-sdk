use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u32,
}

impl Default for Config {
    fn default() -> Self {
        dotenv::dotenv().ok();
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
            },
            database: DatabaseConfig {
                url: "postgres://postgres:password@localhost:5432/postgres".to_string(),
                max_connections: 10,
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| {
                        tracing::warn!("JWT_SECRET not set in environment, using default (INSECURE!)");
                        "your-secret-key-change-this-in-production".to_string()
                    }),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "3".to_string()).parse::<u32>().unwrap_or(3),
            },
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        Self::default()
    }
}
