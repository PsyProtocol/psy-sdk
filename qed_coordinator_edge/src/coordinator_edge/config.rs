use std::env;
use std::path::{ PathBuf};
use dotenvy::from_path;
use serde::Deserialize;
use tracing::{debug, error, info};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub redis_url: String,
    pub coordinator_edge_port: u16,
    pub coordinator_db_path: String,
}

impl AppConfig {
    // try to load .env from multiple paths
    // → default order: [CARGO_MANIFEST_DIR]/.env
    // → current directory
    // → custom path
    pub fn from_env() -> Self {
        let paths = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env"),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".env"),
        ];

        for env_path in &paths {
            if env_path.exists() {
                info!("📄 Loading .env from: {}", env_path.display());
                if let Err(e) = from_path(env_path) {
                    error!("⚠️ Failed to load .env: {}", e);
                } else {
                    break;
                }
            }
        }
        for (k, v) in std::env::vars() {
            debug!("🔍 ENV: {k} = {v}");
        }
        match envy::from_env::<AppConfig>() {
            Ok(config) => {
                info!("✅ Loaded config: {:#?}", config);
                config
            }
            Err(e) => {
                panic!("❌ Failed to parse config from env: {}", e);
            }
        }
    }
}

