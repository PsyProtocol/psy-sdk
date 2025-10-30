pub mod config;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod services;
pub mod auth;

use std::sync::Arc;
pub use config::Config;

pub type Result<T> = anyhow::Result<T>;

use axum::Router;
use services::{create_database_pool, ApiService};
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use crate::services::{CheckpointRewardService, JobStatusService};
use auth::JwtManager;
use crate::handlers::tps::start_tps_broadcast_task;
// Import JWT manager

/// Run the API service with the given configuration.
///
/// This function initializes the database connection pool, starts background
/// tasks, sets up the HTTP server with all routes, and runs until a shutdown
/// signal is received.
pub async fn run(config: Config) -> anyhow::Result<()> {
    tracing::info!("config: {:#?}", config);

    // Load .env file (if not already loaded)
    dotenv::dotenv().ok();

    let pool = create_database_pool(&config).await?;
    let api_service = ApiService::new(pool.clone());

    // Initialize JWT manager with shared secret from .env
    tracing::info!("Initializing JWT authentication with shared secret from .env");
    let jwt_manager = match JwtManager::from_env() {
        Ok(manager) => {
            tracing::info!("JWT authentication initialized successfully");
            Arc::new(manager)
        },
        Err(e) => {
            tracing::error!("Failed to initialize JWT: {}. Make sure JWT_SECRET is set in .env file", e);
            return Err(anyhow::anyhow!("JWT initialization failed: {}", e));
        }
    };

    // Start job status refresh task (refresh every 10 seconds)
    tracing::info!("Starting job status refresh background task");
    let pool_for_job_status = pool.clone();
    tokio::spawn(async move {
        JobStatusService::start_refresh_task(pool_for_job_status, 10).await;
    });

    tracing::info!("Starting checkpoint reward processing background task");
    let pool_for_checkpoint_rewards = pool.clone();
    tokio::spawn(async move {
        CheckpointRewardService::start_checkpoint_reward_task(pool_for_checkpoint_rewards, 30).await;
    });

    // Start worker event processor task (converts worker_events -> worker_job_events)
    tracing::info!("Starting worker event processor background task");
    let pool_for_processor = pool.clone();
    tokio::spawn(async move {
        use repositories::worker_event_processor::{WorkerEventProcessor, EventProcessorConfig};
        WorkerEventProcessor::start_processing_task(pool_for_processor, EventProcessorConfig::default()).await;
    });

    // NEW: Start TPS broadcast task (broadcasts TPS data every 12 seconds)
    tracing::info!("Starting TPS broadcast background task");
    let api_service_for_tps = api_service.clone();
    tokio::spawn(async move {
        start_tps_broadcast_task(api_service_for_tps, 12).await;
    });

    // Create application router
    let app = Router::new()
        .merge(handlers::create_router(api_service.clone()))
        .merge(handlers::create_telemetry_router(api_service.clone(), jwt_manager))
        .merge(handlers::create_contracts_router(api_service.clone()))  // ADD THIS LINE
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
