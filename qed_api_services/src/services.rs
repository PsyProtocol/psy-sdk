use crate::config::Config;
use crate::handlers::websocket::{UserEventManager, WorkerEventManager};
use crate::models::{CheckpointRewardAggregation, CheckpointRewardDistribution, CheckpointRewardSummary, CheckpointStats, CreateCheckpointRewardDistribution, CreateCheckpointStats, CreateWorkerJobEvent, WorkerCheckpointRewardStats, WorkerEvent, WorkerEventReward, WorkerJobEvent};
use crate::repositories::{
    UserEventRepository, WorkerEventRepository, WorkerEventRewardRepository,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::repositories::checkpoint_state::{CheckpointRewardAggregationRepository, CheckpointRewardDistributionRepository, CheckpointStatsRepository, WorkerJobEventRepository};

#[derive(Clone)]
pub struct ApiService {
    pub pool: PgPool,
    pub user_event_manager: UserEventManager,
    pub worker_event_manager: WorkerEventManager,
}

impl ApiService {
    pub fn new(pool: PgPool) -> Self {
        info!("Initializing ApiService with database pool");
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
    info!(
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

    info!("Database connection pool created successfully");

    // Test the connection
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => {
            info!("Database connection test successful");
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
        info!("Starting reward processing task");

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
            debug!("No unprocessed GUTA worker events found");
            return Ok(());
        }

        info!(
            "Processing rewards for {} unprocessed GUTA worker events",
            unprocessed_events.len()
        );

        // Group events by checkpoint for processing
        let mut events_by_checkpoint: std::collections::HashMap<i64, Vec<WorkerEvent>> =
            std::collections::HashMap::new();

        for event in unprocessed_events {
            events_by_checkpoint
                .entry(event.checkpoint_id as i64)
                .or_insert_with(Vec::new)
                .push(event);
        }

        let mut total_processed = 0;

        // Process each checkpoint
        for (checkpoint_id, checkpoint_events) in events_by_checkpoint {
            match Self::process_checkpoint_rewards(pool, checkpoint_id, checkpoint_events).await {
                Ok(rewards_count) => {
                    info!(
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

        info!("Total processed rewards: {}", total_processed);
        Ok(())
    }

    /// Process rewards for worker events in a specific checkpoint
    async fn process_checkpoint_rewards(
        pool: &PgPool,
        checkpoint_id: i64,
        worker_events: Vec<WorkerEvent>,
    ) -> crate::Result<usize> {
        debug!(
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

        debug!(
            "Found {} GUTA user events for checkpoint {} (total reward pool: {} psy)",
            guta_user_events.len(),
            checkpoint_id,
            total_guta_rewards
        );

        if guta_user_events.is_empty() {
            debug!(
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

        debug!(
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
                checkpoint_id: event.checkpoint_id as i64,                       // From worker event
                reward_amount: reward_per_event,
                timestamp: now,
                created_at: now,
                updated_at: now,
            };

            rewards.push(reward);

            debug!(
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
        info!("Starting reward processing background task (interval: 10s)");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            match Self::process_pending_rewards(&pool).await {
                Ok(()) => {
                    debug!("Reward processing task completed successfully");
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
        debug!("Refreshing latest_job_status materialized view");

        let start = std::time::Instant::now();

        // Use CONCURRENTLY to avoid locking the view during refresh
        let result = sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY latest_job_status")
            .execute(pool)
            .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                info!(
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
        info!(
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
                    debug!("Job status refresh task completed successfully");
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
        info!("Manually triggered job status materialized view refresh");
        Self::refresh_materialized_view(pool).await
    }
}


/// Service for managing checkpoint reward calculations
pub struct CheckpointRewardService {
    pool: PgPool,
}

impl CheckpointRewardService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Report checkpoint stats (called by watcher after 3+ block confirmations)
    pub async fn report_checkpoint_stats(
        &self,
        stats: CreateCheckpointStats,
    ) -> anyhow::Result<CheckpointStats> {
        info!(
            "Reporting checkpoint stats for checkpoint {}: fees={}, transactions={}",
            stats.checkpoint_id,
            stats.fees_collected,
            stats.total_transactions
        );

        let checkpoint_stats = CheckpointStatsRepository::create(&self.pool, &stats).await?;

        info!(
            "Successfully reported checkpoint stats for checkpoint {}",
            checkpoint_stats.checkpoint_id
        );

        Ok(checkpoint_stats)
    }

    /// Report worker job events (called by watcher after 3+ block confirmations)
    pub async fn report_worker_job_events(
        &self,
        events: Vec<CreateWorkerJobEvent>,
    ) -> anyhow::Result<Vec<WorkerJobEvent>> {
        if events.is_empty() {
            return Ok(vec![]);
        }

        let checkpoint_id = events[0].checkpoint_id;
        info!(
            "Reporting {} worker job events for checkpoint {}",
            events.len(),
            checkpoint_id
        );

        let created_events = WorkerJobEventRepository::create_batch(&self.pool, &events).await?;

        info!(
            "Successfully reported {} worker job events for checkpoint {}",
            created_events.len(),
            checkpoint_id
        );

        Ok(created_events)
    }

    /// Calculate and distribute rewards for a checkpoint
    /// This should be called after checkpoint stats and worker job events are reported
    pub async fn calculate_and_distribute_rewards(
        &self,
        checkpoint_id: i64,
    ) -> anyhow::Result<Vec<CheckpointRewardDistribution>> {
        info!(
            "Starting reward calculation for checkpoint {}",
            checkpoint_id
        );

        // 1. Get checkpoint stats
        let checkpoint_stats = CheckpointStatsRepository::get_by_checkpoint_id(
            &self.pool,
            checkpoint_id,
        )
            .await?;

        let checkpoint_stats = match checkpoint_stats {
            Some(stats) => stats,
            None => {
                tracing::warn!("No checkpoint stats found for checkpoint {}", checkpoint_id);
                return Ok(vec![]);
            }
        };

        // 2. Check if there are any fees to distribute
        if checkpoint_stats.fees_collected == 0 {
            info!(
                "No fees collected at checkpoint {}, skipping reward distribution",
                checkpoint_id
            );
            return Ok(vec![]);
        }

        // 3. Get all worker job events for this checkpoint
        let job_events = WorkerJobEventRepository::get_by_checkpoint(&self.pool, checkpoint_id).await?;

        if job_events.is_empty() {
            tracing::warn!(
                "No job events found for checkpoint {} despite fees being collected",
                checkpoint_id
            );
            return Ok(vec![]);
        }

        let total_jobs = job_events.len() as i64;
        let total_fees = checkpoint_stats.fees_collected;
        let reward_per_job = total_fees / total_jobs;  // Integer division
        let remainder = total_fees % total_jobs;       // Remainder to distribute

        info!(
            "Checkpoint {}: {} total fees, {} total jobs, {} reward per job, {} remainder",
            checkpoint_id,
            total_fees,
            total_jobs,
            reward_per_job,
            remainder
        );

        // 4. Create one reward distribution per job
        let mut distributions = Vec::new();
        let timestamp = Utc::now();

        for (index, event) in job_events.iter().enumerate() {
            // Distribute remainder to first N jobs
            let reward_for_this_job = if (index as i64) < remainder {
                reward_per_job + 1
            } else {
                reward_per_job
            };

            let distribution = CreateCheckpointRewardDistribution {
                checkpoint_id,
                worker_public_key: event.worker_public_key.clone(),
                job_id: event.id,
                reward_amount: reward_for_this_job,
                total_fees_at_checkpoint: total_fees,
                total_jobs_at_checkpoint: total_jobs,
                metadata: None,
                timestamp,
            };

            distributions.push(distribution);
        }

        // 5. Batch insert reward distributions (one per job)
        let created_distributions =
            CheckpointRewardDistributionRepository::create_batch(&self.pool, &distributions)
                .await?;

        // 6. Log summary by worker
        let mut worker_rewards: HashMap<String, (i64, i64)> = HashMap::new();
        for dist in &created_distributions {
            let entry = worker_rewards
                .entry(dist.worker_public_key.clone())
                .or_insert((0, 0));
            entry.0 += 1; // job count
            entry.1 += dist.reward_amount; // total reward
        }

        for (worker_public_key, (job_count, total_reward)) in worker_rewards {
            info!(
                "Worker {} completed {} jobs, earning {} total reward ({} records created)",
                worker_public_key,
                job_count,
                total_reward,
                job_count
            );
        }

        info!(
            "Successfully created {} reward distributions (one per job) for checkpoint {}",
            created_distributions.len(),
            checkpoint_id
        );

        Ok(created_distributions)
    }

    /// Process a complete checkpoint (stats + events + reward calculation)
    /// This is a convenience method that handles the entire flow
    pub async fn process_checkpoint(
        &self,
        checkpoint_stats: CreateCheckpointStats,
        job_events: Vec<CreateWorkerJobEvent>,
    ) -> anyhow::Result<ProcessCheckpointResult> {
        let checkpoint_id = checkpoint_stats.checkpoint_id;

        info!("Processing complete checkpoint {}", checkpoint_id);

        // 1. Report checkpoint stats
        let stats = self.report_checkpoint_stats(checkpoint_stats).await?;

        // 2. Report worker job events
        let events = self.report_worker_job_events(job_events).await?;

        // 3. Calculate and distribute rewards (only if fees > 0)
        let distributions = if stats.fees_collected > 0 {
            self.calculate_and_distribute_rewards(checkpoint_id).await?
        } else {
            info!("No fees collected, skipping reward distribution");
            vec![]
        };

        Ok(ProcessCheckpointResult {
            checkpoint_stats: stats,
            job_events: events,
            reward_distributions: distributions,
        })
    }

    /// Get reward summary for a checkpoint
    pub async fn get_checkpoint_summary(
        &self,
        checkpoint_id: i64,
    ) -> anyhow::Result<Option<CheckpointRewardSummary>> {
        CheckpointRewardDistributionRepository::get_checkpoint_summary(&self.pool, checkpoint_id)
            .await
    }

    /// Get worker's reward statistics
    pub async fn get_worker_stats(
        &self,
        worker_public_key: &str,
    ) -> anyhow::Result<Option<WorkerCheckpointRewardStats>> {
        CheckpointRewardAggregationRepository::get_worker_stats(&self.pool, worker_public_key).await
    }

    /// Get worker's rewards in a time period from aggregated views
    pub async fn get_worker_rewards_aggregated(
        &self,
        worker_public_key: &str,
        time_period: TimePeriod,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> anyhow::Result<Vec<CheckpointRewardAggregation>> {
        let view_name = match time_period {
            TimePeriod::TwoMinutes => "checkpoint_rewards_2m",
            TimePeriod::OneHour => "checkpoint_rewards_1h",
            TimePeriod::OneDay => "checkpoint_rewards_1d",
            TimePeriod::OneWeek => "checkpoint_rewards_1w",
            TimePeriod::OneMonth => "checkpoint_rewards_1m",
        };

        CheckpointRewardAggregationRepository::get_aggregations(
            &self.pool,
            view_name,
            worker_public_key,
            start_time,
            end_time,
            limit,
        )
            .await
    }

    /// Manually refresh all continuous aggregates
    pub async fn refresh_all_aggregates(&self) -> anyhow::Result<()> {
        let views = vec![
            "checkpoint_rewards_2m",
            "checkpoint_rewards_1h",
            "checkpoint_rewards_1d",
            "checkpoint_rewards_1w",
            "checkpoint_rewards_1m",
        ];

        for view in views {
            CheckpointRewardAggregationRepository::refresh_aggregate(&self.pool, view).await?;
            info!("Refreshed aggregate view: {}", view);
        }

        Ok(())
    }


    /// Start background task to process pending checkpoint rewards
    /// This runs periodically and checks for checkpoints that need reward calculation
    pub async fn start_checkpoint_reward_task(pool: PgPool, interval_seconds: u64) {
        info!(
            "Starting checkpoint reward processing task (interval: {}s)",
            interval_seconds
        );

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;

        loop {
            interval.tick().await;

            match Self::process_pending_checkpoint_rewards(&pool).await {
                Ok(processed_count) => {
                    if processed_count > 0 {
                        info!(
                            "✅ Successfully processed rewards for {} checkpoints",
                            processed_count
                        );
                    }
                    consecutive_errors = 0; // Reset error counter on success
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::error!(
                        "❌ Failed to process checkpoint rewards (attempt {}/{}): {}",
                        consecutive_errors,
                        MAX_CONSECUTIVE_ERRORS,
                        e
                    );

                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        tracing::error!(
                            "🚨 Too many consecutive errors ({}), backing off for 5 minutes",
                            consecutive_errors
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                        consecutive_errors = 0; // Reset after backoff
                    }
                }
            }
        }
    }

    /// Process all pending checkpoints that need reward calculation
    /// Returns the number of checkpoints processed
    async fn process_pending_checkpoint_rewards(pool: &PgPool) -> anyhow::Result<usize> {
        // Find checkpoints that have stats and job events but no reward distributions
        let pending_checkpoints = Self::find_pending_checkpoints(pool).await?;

        if pending_checkpoints.is_empty() {
            debug!("No pending checkpoints to calculate rewards for");
            return Ok(0);
        }

        let (valid_checkpoints, invalid_checkpoints): (Vec<i64>, Vec<i64>) =
            pending_checkpoints.into_iter().partition(|&id| id >= 0);

        if !invalid_checkpoints.is_empty() {
            warn!(
                "⚠️ Found invalid (negative) checkpoint IDs in DB: {:?}",
                invalid_checkpoints
            );
        }

        let mut processed_count = 0;

        for checkpoint_id in valid_checkpoints {
            match Self::calculate_and_distribute_rewards_static(pool, checkpoint_id).await {
                Ok(distributions) => {
                    if !distributions.is_empty() {
                        info!(
                            "✅ Processed checkpoint {}: {} reward distributions created",
                            checkpoint_id,
                            distributions.len()
                        );
                        processed_count += 1;
                    } else {
                        info!(
                            "⏭️  Skipped checkpoint {} (no fees to distribute)",
                            checkpoint_id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Failed to process checkpoint {}: {}",
                        checkpoint_id,
                        e
                    );
                    // Continue processing other checkpoints even if one fails
                }
            }
        }

        Ok(processed_count)
    }

    /// Find checkpoints that have both stats and job events but no reward distributions
    async fn find_pending_checkpoints(pool: &PgPool) -> anyhow::Result<Vec<i64>> {
        let query = r#"
            SELECT DISTINCT cs.checkpoint_id
            FROM checkpoint_stats cs
            INNER JOIN worker_job_events wje
                ON cs.checkpoint_id = wje.checkpoint_id
            LEFT JOIN checkpoint_reward_distributions crd
                ON cs.checkpoint_id = crd.checkpoint_id
            WHERE crd.checkpoint_id IS NULL  -- No rewards calculated yet
              AND cs.fees_collected > 0      -- Only process checkpoints with fees
            ORDER BY cs.checkpoint_id ASC
            LIMIT 50  -- Process in batches of 50
        "#;

        let rows = sqlx::query_scalar::<_, i64>(query)
            .fetch_all(pool)
            .await?;

        Ok(rows)
    }

    /// Public version of find_pending_checkpoints for admin endpoint
    pub async fn find_pending_checkpoints_public(pool: &PgPool) -> anyhow::Result<Vec<i64>> {
        Self::find_pending_checkpoints(pool).await
    }


    /// Static version of calculate_and_distribute_rewards that takes &PgPool
    /// This is used by the background task
    pub async fn calculate_and_distribute_rewards_static(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> anyhow::Result<Vec<CheckpointRewardDistribution>> {
        info!(
            "Starting reward calculation for checkpoint {}",
            checkpoint_id
        );

        // 1. Get checkpoint stats
        let checkpoint_stats = CheckpointStatsRepository::get_by_checkpoint_id(
            pool,
            checkpoint_id,
        )
        .await?;

        let checkpoint_stats = match checkpoint_stats {
            Some(stats) => stats,
            None => {
                warn!("No checkpoint stats found for checkpoint {}", checkpoint_id);
                return Ok(vec![]);
            }
        };
        debug!("Checkpoint stats: {:?}", checkpoint_stats);

        // 2. Check if there are any fees to distribute
        if checkpoint_stats.fees_collected == 0 {
            info!(
                "No fees collected at checkpoint {}, skipping reward distribution",
                checkpoint_id
            );
            return Ok(vec![]);
        }

        // 3. Get all worker job events for this checkpoint
        let job_events = WorkerJobEventRepository::get_by_checkpoint(pool, checkpoint_id).await?;

        if job_events.is_empty() {
            warn!(
                "❗ No job events found for checkpoint {} despite fees being collected",
                checkpoint_id
            );
            return Ok(vec![]);
        }


        let total_jobs = job_events.len() as i64;
        let total_fees = checkpoint_stats.fees_collected;

        //to protect against division by zero
        if total_fees <= 0  {
            error!(
                "❌ Checkpoint {} has non-positive fees: {}, skipping",
                checkpoint_id, total_fees
            );
            return Ok(vec![]);
        }

        //todo! maybe need more checks here
        let reward_per_job = total_fees / total_jobs;  // Integer division
        let remainder = total_fees % total_jobs;       // Remainder to distribute

        info!(
            "✅ Checkpoint {}: {} total fees, {} total jobs, {} reward per job, {} remainder",
            checkpoint_id,
            total_fees,
            total_jobs,
            reward_per_job,
            remainder
        );

        // 4. Create one reward distribution per job
        let mut distributions = Vec::new();
        let timestamp = Utc::now();

        for (_index, event) in job_events.iter().enumerate() {
            //deflation model, discarding the remainder
            let reward_for_this_job = reward_per_job;

            // Distribute remainder to first N jobs
            // let reward_for_this_job = if (index as i64) < remainder {
            //     reward_per_job + 1
            // } else {
            //     reward_per_job
            // };

            let distribution = CreateCheckpointRewardDistribution {
                checkpoint_id,
                worker_public_key: event.worker_public_key.clone(),
                job_id: event.id,   //note: uuid here not QProvingJobID
                reward_amount: reward_for_this_job,
                total_fees_at_checkpoint: total_fees,
                total_jobs_at_checkpoint: total_jobs,
                metadata: None,
                timestamp,
            };

            distributions.push(distribution);
        }

        // 5. Batch insert reward distributions (one per job)
        let created_distributions =
            CheckpointRewardDistributionRepository::create_batch(pool, &distributions).await?;

        info!(
            "✅ Created {} reward distributions for checkpoint {}",
            created_distributions.len(),
            checkpoint_id
        );

        Ok(created_distributions)
    }
}


/// Time period for aggregated queries
#[derive(Debug, Clone, Copy)]
pub enum TimePeriod {
    TwoMinutes,
    OneHour,
    OneDay,
    OneWeek,
    OneMonth,
}

/// Result of processing a complete checkpoint
#[derive(Debug)]
pub struct ProcessCheckpointResult {
    pub checkpoint_stats: CheckpointStats,
    pub job_events: Vec<WorkerJobEvent>,
    pub reward_distributions: Vec<CheckpointRewardDistribution>,
}