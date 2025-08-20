use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub coordinator: NodeConfig,
    pub realm: NodeConfig,
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
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    pub base_url: String,
    pub auth_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
            },
            database: DatabaseConfig {
                url: "postgresql://postgres:password@localhost:5432/qed_api".to_string(),
                max_connections: 10,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 10,
            },
            coordinator: NodeConfig {
                base_url: "http://localhost:8080".to_string(),
                auth_token: None,
            },
            realm: NodeConfig {
                base_url: "http://localhost:8081".to_string(),
                auth_token: None,
            },
        }
    }
}

impl Config {
    pub fn from_env() -> crate::Result<Self> {
        dotenvy::dotenv().ok();
        
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .build()?;
            
        Ok(config.try_deserialize()?)
    }
}