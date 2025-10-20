use crate::config::Config;
use crate::handlers::websocket::{UserEventManager, WorkerEventManager};
use crate::models::{WorkerEvent, WorkerEventReward};
use crate::repositories::{
    UserEventRepository, WorkerEventRepository, WorkerEventRewardRepository,
};
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub struct ApiService {
    pub pool: PgPool,
    pub user_event_manager: UserEventManager,
    pub worker_event_manager: WorkerEventManager,
}

impl ApiService {
    pub fn new(pool: PgPool) -> Self {
        tracing::info!("Initializing ApiService with database pool");
        Self {
            pool,
            user_event_manager: UserEventManager {
                connections: Arc::new(RwLock::new(HashMap::new())),
            },
            worker_event_manager: WorkerEventManager {
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

/// Reward calculation service
pub struct RewardService;

impl RewardService {
    const REWARD_PER_GUTA_PSY: i64 = 5_000_000_000; // 5 * 10^9 psy per GUTA

    /// Process worker event rewards for unprocessed events
    pub async fn process_pending_rewards(pool: &PgPool) -> crate::Result<()> {
        tracing::info!("Starting reward processing task");

        // Get max checkpoint from worker_events
        let max_worker_checkpoint = WorkerEventRepository::get_max_checkpoint(pool)
            .await?
            .unwrap_or(0);
        info!("Max worker checkpoint: {}", max_worker_checkpoint);
        // Process unprocessed GUTA worker events up to max_checkpoint - 1
        // (excluding current block to ensure it's finalized)
        let checkpoint_range = Some((0, max_worker_checkpoint - 1));
        let unprocessed_events =
            WorkerEventRepository::get_unprocessed_guta_worker_events(pool, checkpoint_range)
                .await?;

        if unprocessed_events.is_empty() {
            tracing::debug!("No unprocessed GUTA worker events found");
            return Ok(());
        }

        tracing::info!(
            "Processing rewards for {} unprocessed GUTA worker events",
            unprocessed_events.len()
        );

        // Group events by checkpoint for processing
        let mut events_by_checkpoint: std::collections::HashMap<i64, Vec<WorkerEvent>> =
            std::collections::HashMap::new();

        for event in unprocessed_events {
            events_by_checkpoint
                .entry(event.checkpoint_id)
                .or_insert_with(Vec::new)
                .push(event);
        }

        let mut total_processed = 0;

        // Process each checkpoint
        for (checkpoint_id, checkpoint_events) in events_by_checkpoint {
            match Self::process_checkpoint_rewards(pool, checkpoint_id, checkpoint_events).await {
                Ok(rewards_count) => {
                    tracing::info!(
                        "Successfully processed {} rewards for checkpoint {}",
                        rewards_count,
                        checkpoint_id
                    );
                    total_processed += rewards_count;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to process rewards for checkpoint {}: {}",
                        checkpoint_id,
                        e
                    );
                    // Continue processing other checkpoints even if one fails
                }
            }
        }

        tracing::info!("Total processed rewards: {}", total_processed);
        Ok(())
    }

    /// Process rewards for worker events in a specific checkpoint
    async fn process_checkpoint_rewards(
        pool: &PgPool,
        checkpoint_id: i64,
        worker_events: Vec<WorkerEvent>,
    ) -> crate::Result<usize> {
        tracing::debug!(
            "Processing rewards for checkpoint {} with {} worker events",
            checkpoint_id,
            worker_events.len()
        );

        if worker_events.is_empty() {
            return Ok(0);
        }

        // Step 1: Get all GUTA user events for this checkpoint to calculate total reward pool
        let guta_user_events =
            UserEventRepository::get_guta_events_by_checkpoint(pool, checkpoint_id).await?;
        let total_guta_rewards = guta_user_events.len() as i64 * Self::REWARD_PER_GUTA_PSY;

        tracing::debug!(
            "Found {} GUTA user events for checkpoint {} (total reward pool: {} psy)",
            guta_user_events.len(),
            checkpoint_id,
            total_guta_rewards
        );

        if guta_user_events.is_empty() {
            tracing::debug!(
                "No GUTA user events found for checkpoint {}, no rewards to distribute",
                checkpoint_id
            );
            return Ok(0);
        }

        // Step 2: Calculate reward per worker event
        // Each GUTA worker event gets an equal share of the total reward pool
        let total_worker_events = worker_events.len() as i64;
        let reward_per_event = if total_worker_events > 0 {
            total_guta_rewards / total_worker_events
        } else {
            0
        };

        tracing::debug!(
            "Reward per worker event: {} psy (total pool: {} / {} events)",
            reward_per_event,
            total_guta_rewards,
            total_worker_events
        );

        // Step 3: Create reward records for each worker event
        let mut rewards = Vec::new();
        let now = Utc::now();

        for event in worker_events {
            let event_id = event.id.unwrap(); // We know it exists from the query
            let reward = WorkerEventReward {
                id: event_id,                                             // Same as worker event id
                public_key: event.public_key.clone().unwrap_or_default(), // From worker event
                checkpoint_id: event.checkpoint_id,                       // From worker event
                reward_amount: reward_per_event,
                timestamp: now,
                created_at: now,
                updated_at: now,
            };

            rewards.push(reward);

            tracing::debug!(
                "Created reward for worker event {} (worker: {}, checkpoint: {}): {} psy",
                event_id,
                event.public_key.as_deref().unwrap_or("unknown"),
                event.checkpoint_id,
                reward_per_event
            );
        }

        // Step 4: Insert rewards in database
        WorkerEventRewardRepository::create_rewards(pool, &rewards).await?;

        Ok(rewards.len())
    }

    /// Background task that runs reward processing every 10 seconds
    pub async fn start_reward_processing_task(pool: PgPool) {
        tracing::info!("Starting reward processing background task (interval: 10s)");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            match Self::process_pending_rewards(&pool).await {
                Ok(()) => {
                    tracing::debug!("Reward processing task completed successfully");
                }
                Err(e) => {
                    tracing::error!("Reward processing task failed: {}", e);
                }
            }
        }
    }
}

/// Job status aggregation service
pub struct JobStatusService;

impl JobStatusService {
    /// Refresh the latest_job_status materialized view
    pub async fn refresh_materialized_view(pool: &PgPool) -> crate::Result<()> {
        tracing::debug!("Refreshing latest_job_status materialized view");

        let start = std::time::Instant::now();

        // Use CONCURRENTLY to avoid locking the view during refresh
        let result = sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY latest_job_status")
            .execute(pool)
            .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                tracing::info!(
                    "Successfully refreshed latest_job_status materialized view in {:?}",
                    duration
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to refresh latest_job_status materialized view: {}",
                    e
                );
                Err(anyhow::anyhow!("Failed to refresh materialized view: {}", e))
            }
        }
    }

    /// Background task that refreshes the materialized view periodically
    pub async fn start_refresh_task(pool: PgPool, refresh_interval_secs: u64) {
        tracing::info!(
            "Starting job status materialized view refresh task (interval: {}s)",
            refresh_interval_secs
        );

        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(refresh_interval_secs)
        );

        // Skip the first tick to avoid immediate refresh on startup
        interval.tick().await;

        let mut consecutive_failures = 0;
        const MAX_CONSECUTIVE_FAILURES: u32 = 5;

        loop {
            interval.tick().await;

            match Self::refresh_materialized_view(&pool).await {
                Ok(()) => {
                    consecutive_failures = 0;
                    tracing::debug!("Job status refresh task completed successfully");
                }
                Err(e) => {
                    consecutive_failures += 1;
                    tracing::error!(
                        "Job status refresh task failed (attempt {}/{}): {}",
                        consecutive_failures,
                        MAX_CONSECUTIVE_FAILURES,
                        e
                    );

                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        tracing::error!(
                            "Job status refresh task failed {} times consecutively. \
                            Continuing with exponential backoff...",
                            MAX_CONSECUTIVE_FAILURES
                        );

                        // Implement exponential backoff
                        let backoff_secs = std::cmp::min(
                            refresh_interval_secs * 2_u64.pow(consecutive_failures - MAX_CONSECUTIVE_FAILURES),
                            300 // Max 5 minutes
                        );

                        tracing::warn!(
                            "Backing off for {} seconds before next refresh attempt",
                            backoff_secs
                        );

                        tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    }
                }
            }
        }
    }

    /// Manually trigger a refresh (useful for testing or admin endpoints)
    pub async fn force_refresh(pool: &PgPool) -> crate::Result<()> {
        tracing::info!("Manually triggered job status materialized view refresh");
        Self::refresh_materialized_view(pool).await
    }
}

