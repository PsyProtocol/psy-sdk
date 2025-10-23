use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use crate::models::{CheckpointRewardAggregation, CheckpointRewardDistribution, CheckpointRewardSummary, CheckpointStats, CreateCheckpointRewardDistribution, CreateCheckpointStats, CreateWorkerJobEvent, WorkerCheckpointRewardStats, WorkerJobEvent};

/// Repository for checkpoint statistics
pub struct CheckpointStatsRepository;

impl CheckpointStatsRepository {
    /// Create a new checkpoint stats entry
    pub async fn create(pool: &PgPool, stats: &CreateCheckpointStats) -> Result<CheckpointStats> {
        let metadata = stats.metadata.clone().unwrap_or(serde_json::json!({}));

        let record = sqlx::query_as!(
            CheckpointStats,
            r#"
            INSERT INTO checkpoint_stats
                (checkpoint_id, fees_collected, user_ops_processed, total_transactions,
                 slots_modified, metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (checkpoint_id) DO UPDATE
            SET fees_collected = EXCLUDED.fees_collected,
                user_ops_processed = EXCLUDED.user_ops_processed,
                total_transactions = EXCLUDED.total_transactions,
                slots_modified = EXCLUDED.slots_modified,
                metadata = EXCLUDED.metadata,
                timestamp = EXCLUDED.timestamp,
                updated_at = NOW()
            RETURNING checkpoint_id, fees_collected, user_ops_processed, total_transactions,
                      slots_modified, metadata, timestamp, created_at, updated_at
            "#,
            stats.checkpoint_id,
            stats.fees_collected,
            stats.user_ops_processed,
            stats.total_transactions,
            stats.slots_modified,
            metadata,
            stats.timestamp
        )
        .fetch_one(pool)
        .await?;

        Ok(record)
    }

    /// Get checkpoint stats by checkpoint_id
    pub async fn get_by_checkpoint_id(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<Option<CheckpointStats>> {
        let record = sqlx::query_as!(
            CheckpointStats,
            r#"
            SELECT checkpoint_id, fees_collected, user_ops_processed, total_transactions,
                   slots_modified, metadata, timestamp, created_at, updated_at
            FROM checkpoint_stats
            WHERE checkpoint_id = $1
            "#,
            checkpoint_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    /// Get checkpoint stats in a range
    pub async fn get_by_checkpoint_range(
        pool: &PgPool,
        start_checkpoint: i64,
        end_checkpoint: i64,
    ) -> Result<Vec<CheckpointStats>> {
        let records = sqlx::query_as!(
            CheckpointStats,
            r#"
            SELECT checkpoint_id, fees_collected, user_ops_processed, total_transactions,
                   slots_modified, metadata, timestamp, created_at, updated_at
            FROM checkpoint_stats
            WHERE checkpoint_id >= $1 AND checkpoint_id <= $2
            ORDER BY checkpoint_id ASC
            "#,
            start_checkpoint,
            end_checkpoint
        )
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// Get checkpoint stats with non-zero fees (reward-eligible checkpoints)
    pub async fn get_reward_eligible_checkpoints(
        pool: &PgPool,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<CheckpointStats>> {
        let records = sqlx::query_as!(
            CheckpointStats,
            r#"
            SELECT checkpoint_id, fees_collected, user_ops_processed, total_transactions,
                   slots_modified, metadata, timestamp, created_at, updated_at
            FROM checkpoint_stats
            WHERE fees_collected > 0
                AND ($1::TIMESTAMPTZ IS NULL OR timestamp >= $1)
                AND ($2::TIMESTAMPTZ IS NULL OR timestamp <= $2)
            ORDER BY checkpoint_id DESC
            LIMIT $3
            "#,
            start_time,
            end_time,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(records)
    }
}

/// Repository for worker job events
pub struct WorkerJobEventRepository;

impl WorkerJobEventRepository {
    /// Create a new worker job event
    pub async fn create(pool: &PgPool, event: &CreateWorkerJobEvent) -> Result<WorkerJobEvent> {
        let metadata = event.metadata.clone().unwrap_or(serde_json::json!({}));
        let status = event.status.clone().unwrap_or_else(|| "COMPLETED".to_string());

        let record = sqlx::query_as!(
            WorkerJobEvent,
            r#"
            INSERT INTO worker_job_events
                (worker_public_key, checkpoint_id, job_id, topic, circuit_type,
                 duration, status, metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, worker_public_key, checkpoint_id, job_id, topic, circuit_type,
                      duration, status, metadata, timestamp, created_at, updated_at
            "#,
            event.worker_public_key,
            event.checkpoint_id,
            event.job_id,
            event.topic,
            event.circuit_type,
            event.duration,
            status,
            metadata,
            event.timestamp
        )
        .fetch_one(pool)
        .await?;

        Ok(record)
    }

    /// Batch create worker job events
    pub async fn create_batch(
        pool: &PgPool,
        events: &[CreateWorkerJobEvent],
    ) -> Result<Vec<WorkerJobEvent>> {
        let mut created_events = Vec::new();

        for event in events {
            let created = Self::create(pool, event).await?;
            created_events.push(created);
        }

        Ok(created_events)
    }

    /// Get worker job events by checkpoint_id
    pub async fn get_by_checkpoint(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<Vec<WorkerJobEvent>> {
        let records = sqlx::query_as!(
            WorkerJobEvent,
            r#"
            SELECT id, worker_public_key, checkpoint_id, job_id, topic, circuit_type,
                   duration, status, metadata, timestamp, created_at, updated_at
            FROM worker_job_events
            WHERE checkpoint_id = $1
            ORDER BY timestamp ASC
            "#,
            checkpoint_id
        )
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// Get worker job events by worker public key
    pub async fn get_by_worker(
        pool: &PgPool,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<WorkerJobEvent>> {
        let records = sqlx::query_as!(
            WorkerJobEvent,
            r#"
            SELECT id, worker_public_key, checkpoint_id, job_id, topic, circuit_type,
                   duration, status, metadata, timestamp, created_at, updated_at
            FROM worker_job_events
            WHERE worker_public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR timestamp >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR timestamp <= $3)
            ORDER BY timestamp DESC
            LIMIT $4
            "#,
            worker_public_key,
            start_time,
            end_time,
            limit
        )
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// Count jobs by checkpoint (for reward calculation)
    pub async fn count_jobs_by_checkpoint(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM worker_job_events
            WHERE checkpoint_id = $1
            "#,
            checkpoint_id
        )
            .fetch_one(pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }
}

/// Repository for checkpoint reward distributions
pub struct CheckpointRewardDistributionRepository;

impl CheckpointRewardDistributionRepository {
    /// Create a new checkpoint reward distribution
    pub async fn create(
        pool: &PgPool,
        distribution: &CreateCheckpointRewardDistribution,
    ) -> Result<CheckpointRewardDistribution> {
        let metadata = distribution.metadata.clone().unwrap_or(serde_json::json!({}));

        let record = sqlx::query_as!(
            CheckpointRewardDistribution,
            r#"
            INSERT INTO checkpoint_reward_distributions
                (checkpoint_id, worker_public_key, job_id, reward_amount,
                 total_fees_at_checkpoint, total_jobs_at_checkpoint,
                 metadata, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, checkpoint_id, worker_public_key, job_id, reward_amount,
                      total_fees_at_checkpoint, total_jobs_at_checkpoint,
                      metadata, timestamp, created_at, updated_at
            "#,
            distribution.checkpoint_id,
            distribution.worker_public_key,
            distribution.job_id,
            distribution.reward_amount,
            distribution.total_fees_at_checkpoint,
            distribution.total_jobs_at_checkpoint,
            metadata,
            distribution.timestamp
        )
            .fetch_one(pool)
            .await?;

        Ok(record)
    }

    /// Batch create checkpoint reward distributions
    pub async fn create_batch(
        pool: &PgPool,
        distributions: &[CreateCheckpointRewardDistribution],
    ) -> Result<Vec<CheckpointRewardDistribution>> {
        let mut created_distributions = Vec::new();

        for distribution in distributions {
            let created = Self::create(pool, distribution).await?;
            created_distributions.push(created);
        }

        Ok(created_distributions)
    }

    /// Get reward distributions by checkpoint
    pub async fn get_by_checkpoint(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<Vec<CheckpointRewardDistribution>> {
        let records = sqlx::query_as!(
            CheckpointRewardDistribution,
            r#"
            SELECT id, checkpoint_id, worker_public_key, job_id, reward_amount,
                   total_fees_at_checkpoint, total_jobs_at_checkpoint,
                   metadata, timestamp, created_at, updated_at
            FROM checkpoint_reward_distributions
            WHERE checkpoint_id = $1
            ORDER BY reward_amount DESC
            "#,
            checkpoint_id
        )
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// Get reward distributions by worker
    pub async fn get_by_worker(
        pool: &PgPool,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<CheckpointRewardDistribution>> {
        let records = sqlx::query_as!(
            CheckpointRewardDistribution,
            r#"
            SELECT id, checkpoint_id, worker_public_key, job_id, reward_amount,
                   total_fees_at_checkpoint, total_jobs_at_checkpoint,
                   metadata, timestamp, created_at, updated_at
            FROM checkpoint_reward_distributions
            WHERE worker_public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR timestamp >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR timestamp <= $3)
            ORDER BY timestamp DESC
            LIMIT $4
            "#,
            worker_public_key,
            start_time,
            end_time,
            limit
        )
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// Get checkpoint reward summary
    pub async fn get_checkpoint_summary(
        pool: &PgPool,
        checkpoint_id: i64,
    ) -> Result<Option<CheckpointRewardSummary>> {
        let record = sqlx::query!(
            r#"
            SELECT
                cs.checkpoint_id,
                cs.fees_collected,
                cs.timestamp,
                COUNT(DISTINCT crd.worker_public_key) as total_workers,
                COUNT(DISTINCT crd.job_id) as total_jobs,
                CASE
                    WHEN COUNT(DISTINCT crd.job_id) > 0
                    THEN cs.fees_collected / COUNT(DISTINCT crd.job_id)
                    ELSE 0
                END as reward_per_job
            FROM checkpoint_stats cs
            LEFT JOIN checkpoint_reward_distributions crd ON cs.checkpoint_id = crd.checkpoint_id
            WHERE cs.checkpoint_id = $1
            GROUP BY cs.checkpoint_id, cs.fees_collected, cs.timestamp
            "#,
            checkpoint_id
        )
            .fetch_optional(pool)
            .await?;

        Ok(record.map(|r| CheckpointRewardSummary {
            checkpoint_id: r.checkpoint_id,
            fees_collected: r.fees_collected,
            total_jobs: r.total_jobs.unwrap_or(0),
            total_workers: r.total_workers.unwrap_or(0),
            reward_per_job: r.reward_per_job.unwrap_or(0),
            timestamp: r.timestamp,
        }))
    }
}

/// Repository for checkpoint reward aggregations
pub struct CheckpointRewardAggregationRepository;

impl CheckpointRewardAggregationRepository {
    /// Get aggregated checkpoint rewards from continuous aggregates
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<CheckpointRewardAggregation>> {
        // Validate view name to prevent SQL injection
        let valid_views = vec![
            "checkpoint_rewards_2m",
            "checkpoint_rewards_1h",
            "checkpoint_rewards_1d",
            "checkpoint_rewards_1w",
            "checkpoint_rewards_1m",
        ];

        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        let query = format!(
            r#"
            SELECT
                bucket,
                worker_public_key,
                checkpoints_participated::BIGINT,
                jobs_completed::BIGINT,
                total_rewards::BIGINT,
                avg_reward_per_job::DOUBLE PRECISION,
                max_checkpoint::BIGINT,
                min_checkpoint::BIGINT
            FROM {}
            WHERE worker_public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR bucket >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket <= $3)
            ORDER BY bucket DESC
            LIMIT $4
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, CheckpointRewardAggregation>(&query)
            .bind(worker_public_key)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }

    /// Force refresh a continuous aggregate
    pub async fn refresh_aggregate(pool: &PgPool, view_name: &str) -> Result<()> {
        let valid_views = vec![
            "checkpoint_rewards_2m",
            "checkpoint_rewards_1h",
            "checkpoint_rewards_1d",
            "checkpoint_rewards_1w",
            "checkpoint_rewards_1m",
        ];

        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        let query = format!(
            "CALL refresh_continuous_aggregate('{}', NULL, NULL)",
            view_name
        );

        sqlx::query(&query).execute(pool).await?;

        tracing::info!("Successfully refreshed continuous aggregate: {}", view_name);
        Ok(())
    }

    /// Get worker reward statistics across all time
    pub async fn get_worker_stats(
        pool: &PgPool,
        worker_public_key: &str,
    ) -> Result<Option<WorkerCheckpointRewardStats>> {
        let record = sqlx::query!(
            r#"
            SELECT
                worker_public_key,
                SUM(reward_amount)::BIGINT as total_rewards,
                COUNT(DISTINCT job_id)::BIGINT as total_jobs_completed,
                COUNT(DISTINCT checkpoint_id)::BIGINT as checkpoints_participated,
                AVG(reward_amount)::DOUBLE PRECISION as avg_reward_per_job,
                MAX(checkpoint_id)::BIGINT as last_checkpoint_id,
                MAX(timestamp) as last_reward_timestamp
            FROM checkpoint_reward_distributions
            WHERE worker_public_key = $1
            GROUP BY worker_public_key
            "#,
            worker_public_key
        )
            .fetch_optional(pool)
            .await?;

        Ok(record.map(|r| WorkerCheckpointRewardStats {
            worker_public_key: r.worker_public_key,
            total_rewards: r.total_rewards.unwrap_or(0),
            total_jobs_completed: r.total_jobs_completed.unwrap_or(0),
            checkpoints_participated: r.checkpoints_participated.unwrap_or(0),
            avg_reward_per_job: r.avg_reward_per_job.unwrap_or(0.0),
            last_checkpoint_id: r.last_checkpoint_id.unwrap_or(0),
            last_reward_timestamp: r.last_reward_timestamp.unwrap_or_else(Utc::now),
        }))
    }
}
