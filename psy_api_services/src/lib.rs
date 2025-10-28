pub mod config;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod services;

pub use config::Config;

pub type Result<T> = anyhow::Result<T>;

use axum::Router;
use services::{create_database_pool, ApiService, RewardService};
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use crate::services::JobStatusService;

/// Run the API service with the given configuration.
///
/// This function initializes the database connection pool, starts background
/// tasks, sets up the HTTP server with all routes, and runs until a shutdown
/// signal is received.
pub async fn run(config: Config) -> anyhow::Result<()> {
    tracing::info!("config: {:#?}", config);

    let pool = create_database_pool(&config).await?;
    let api_service = ApiService::new(pool.clone());

    // Start background reward processing task
    tracing::info!("Starting reward processing background task");
    let pool_for_rewards = pool.clone();
    tokio::spawn(async move {
        RewardService::start_reward_processing_task(pool_for_rewards).await;
    });

    // Start job status refresh task (refresh every 10 seconds)
    tracing::info!("Starting job status refresh background task");
    let pool_for_job_status = pool.clone();
    tokio::spawn(async move {
        JobStatusService::start_refresh_task(pool_for_job_status, 10).await;
    });

    // Create application router
    let app = Router::new()
        .merge(handlers::create_router(api_service.clone()))
        .merge(handlers::create_telemetry_router(api_service.clone()))
        .merge(handlers::create_websocket_router(api_service))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&format!("{}:{}", config.server.host, config.server.port)).await?;

    tracing::info!("Server starting on {}:{}", config.server.host, config.server.port);

    // Run server with graceful shutdown
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Received shutdown signal, starting graceful shutdown");
}
