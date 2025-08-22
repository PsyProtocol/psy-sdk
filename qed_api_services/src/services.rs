use crate::config::Config;
use crate::websocket::WebSocketManager;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ApiService {
    pub pool: PgPool,
    pub websocket_manager: WebSocketManager,
}

impl ApiService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            websocket_manager: WebSocketManager {
                connections: Arc::new(RwLock::new(HashMap::new())),
            },
        }
    }
}

/// Database connection helper
pub async fn create_database_pool(config: &Config) -> crate::Result<PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;
    Ok(pool)
}
