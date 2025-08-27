use crate::config::Config;
use crate::handlers::websocket::WebSocketManager;
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
        tracing::info!("Initializing ApiService with database pool");
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
    tracing::info!(
        "Creating database connection pool {} with max_connections={}",
        config.database.url,
        config.database.max_connections
    );

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            e
        })?;

    tracing::info!("Database connection pool created successfully");

    // Test the connection
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => {
            tracing::info!("Database connection test successful");
        }
        Err(e) => {
            tracing::warn!("Database connection test failed: {}", e);
        }
    }

    Ok(pool)
}
