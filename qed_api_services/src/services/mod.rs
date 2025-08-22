use crate::config::Config;
use sqlx::PgPool;

#[derive(Clone)]
pub struct ApiService {
    pub pool: PgPool,
}

impl ApiService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
