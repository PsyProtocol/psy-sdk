use axum::Router;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber;

use qed_api_services::{
    config::Config,
    db::DatabaseConnections,
    handlers,
    services::ApiService,
    telemetry,
    websocket,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env().unwrap_or_else(|_| {
        tracing::warn!("Failed to load config from env, using defaults");
        Config::default()
    });

    // TODO: Initialize real database connections after schema is ready
    tracing::info!("Starting API service in development mode (without database connections)");
    
    // Create a mock service for now
    let api_service = ApiService::new_mock();

    // Create application router
    let app = Router::new()
        .merge(handlers::create_router(api_service.clone()))
        .merge(telemetry::create_telemetry_router(api_service.clone()))
        .merge(websocket::create_websocket_router(api_service))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&format!("{}:{}", config.server.host, config.server.port))
        .await?;

    tracing::info!("Server starting on {}:{}", config.server.host, config.server.port);

    axum::serve(listener, app).await?;

    Ok(())
}
