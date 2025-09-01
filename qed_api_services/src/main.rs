use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{self, EnvFilter};

use qed_api_services::{
    config::Config,
    handlers,
    services::{create_database_pool, ApiService},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Initialize tracing with configurable log level
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().unwrap_or_else(|_| {
        tracing::warn!("Failed to load config from env, using defaults");
        Config::default()
    });

    tracing::info!("config: {:#?}", config);

    let pool = create_database_pool(&config).await?;
    let api_service = ApiService::new(pool);

    // Create application router
    let app = Router::new()
        .merge(handlers::create_router(api_service.clone()))
        .merge(handlers::create_telemetry_router(api_service.clone()))
        .merge(handlers::create_websocket_router(api_service))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener =
        tokio::net::TcpListener::bind(&format!("{}:{}", config.server.host, config.server.port))
            .await?;

    tracing::info!(
        "Server starting on {}:{}",
        config.server.host,
        config.server.port
    );

    axum::serve(listener, app).await?;

    Ok(())
}
