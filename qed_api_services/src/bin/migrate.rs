use qed_api_services::db::migrations::MigrationManager;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/postgres".to_string());

    tracing::info!("Connecting to database: {}", database_url);
    let pool = PgPool::connect(&database_url).await?;

    let migration_manager = MigrationManager::new(pool);
    migration_manager.run_migrations().await?;

    tracing::info!("Migrations completed successfully!");
    Ok(())
}
