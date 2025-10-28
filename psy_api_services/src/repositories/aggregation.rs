use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::models::{UserEventAggregation, WorkerEventAggregation, WorkerEventSource, WorkerRewardsAggregation};
use crate::Result;

pub struct WorkerEventAggregationRepository;
pub struct UserEventAggregationRepository;
pub struct WorkerRewardsAggregationRepository;

/// Worker Event Aggregation Queries
impl WorkerEventAggregationRepository {
    /// Get worker event aggregations from materialized views
    /// Note: Uses dynamic query for flexible view selection
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        realm_id: Option<i64>,
        source: Option<WorkerEventSource>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        offset: i64,
        limit: i64,
        order_asc: bool,
    ) -> Result<Vec<WorkerEventAggregation>> {
        // Note: view_name should be validated against a whitelist in production
        let order_direction = if order_asc { "ASC" } else { "DESC" };

        let query = format!(
            r#"
            SELECT
                bucket, realm_id, source,
                count, completed_count, failed_count, processing_count, pending_count,
                avg_duration_ms, min_duration_ms, max_duration_ms
            FROM {}
            WHERE ($1::BIGINT IS NULL OR realm_id = $1)
                AND ($2::VARCHAR IS NULL OR source = $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket >= $3)
                AND ($4::TIMESTAMPTZ IS NULL OR bucket <= $4)
            ORDER BY bucket {}
            LIMIT $5 OFFSET $6
            "#,
            view_name, order_direction
        );

        let aggregations = sqlx::query_as::<_, WorkerEventAggregation>(&query)
            .bind(realm_id)
            .bind(source)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}

/// User Event Aggregation Queries
impl UserEventAggregationRepository {
    /// Get user event aggregations from materialized views
    /// Note: Uses dynamic query for flexible view selection
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        offset: i64,
        limit: i64,
        order_asc: bool,
    ) -> Result<Vec<UserEventAggregation>> {
        let order_direction = if order_asc { "ASC" } else { "DESC" };

        let query = format!(
            r#"
            SELECT
                bucket, count, register_user_count, deploy_contract_count, guta_count
            FROM {}
            WHERE ($1::TIMESTAMPTZ IS NULL OR bucket >= $1)
                AND ($2::TIMESTAMPTZ IS NULL OR bucket <= $2)
            ORDER BY bucket {}
            LIMIT $3 OFFSET $4
            "#,
            view_name, order_direction
        );

        let aggregations = sqlx::query_as::<_, UserEventAggregation>(&query)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}

/// Worker Rewards Aggregation Repository
impl WorkerRewardsAggregationRepository {
    /// Get worker rewards aggregations from materialized views with fallback to raw data
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<WorkerRewardsAggregation>> {
        // First, try to get data from the continuous aggregate
        let query = format!(
            r#"
            SELECT
                bucket,
                public_key,
                completed_proofs::BIGINT as completed_proofs,
                total_rewards::BIGINT as total_rewards,
                max_checkpoint::BIGINT as max_checkpoint
            FROM {}
            WHERE public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR bucket >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket <= $3)
            ORDER BY bucket DESC
            LIMIT $4
            "#,
            view_name
        );

        tracing::debug!(
            "Querying continuous aggregate {} for worker {}",
            view_name,
            worker_public_key
        );

        let aggregations = sqlx::query_as::<_, WorkerRewardsAggregation>(&query)
            .bind(worker_public_key)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        // If we got data from the aggregate, return it
        if !aggregations.is_empty() {
            tracing::info!(
                "Found {} aggregation buckets from continuous aggregate {} for worker {}",
                aggregations.len(),
                view_name,
                worker_public_key
            );
            return Ok(aggregations);
        }

        // Fallback: Query raw data from worker_event_rewards table
        tracing::info!(
            "Continuous aggregate {} is empty for worker {}, falling back to raw data",
            view_name,
            worker_public_key
        );

        // First, check if there's any data at all for this worker
        let check_query = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM worker_event_rewards
            WHERE public_key = $1
            "#,
            worker_public_key
        )
        .fetch_one(pool)
        .await?;

        let total_records = check_query.count.unwrap_or(0);

        if total_records == 0 {
            tracing::warn!(
                "No rewards data found in worker_event_rewards table for worker {}",
                worker_public_key
            );
            return Ok(Vec::new());
        }

        tracing::info!(
            "Found {} total reward records for worker {}, aggregating by time buckets",
            total_records,
            worker_public_key
        );

        // Determine the bucket interval based on view_name
        let interval = match view_name {
            "worker_rewards_1d" => "1 day",
            "worker_rewards_1w" => "1 week",
            "worker_rewards_1m" => "1 month",
            _ => {
                tracing::error!("Unknown view name: {}", view_name);
                return Ok(Vec::new());
            }
        };

        // Build and execute the fallback query using dynamic SQL
        // We use dynamic query here because sqlx has issues with time_bucket and intervals
        let fallback_query = format!(
            r#"
            SELECT
                time_bucket('{}', timestamp) AS bucket,
                public_key,
                COUNT(*)::BIGINT as completed_proofs,
                COALESCE(SUM(reward_amount), 0)::BIGINT as total_rewards,
                COALESCE(MAX(checkpoint_id), 0)::BIGINT as max_checkpoint
            FROM worker_event_rewards
            WHERE public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR timestamp >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR timestamp <= $3)
            GROUP BY time_bucket('{}', timestamp), public_key
            ORDER BY bucket DESC
            LIMIT $4
            "#,
            interval,
            interval
        );

        let rows = sqlx::query(&fallback_query)
            .bind(worker_public_key)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        let mut fallback_aggregations = Vec::new();
        for row in rows {
            let bucket: DateTime<Utc> = row.try_get("bucket")?;
            let public_key: String = row.try_get("public_key")?;
            let completed_proofs: i64 = row.try_get("completed_proofs")?;
            let total_rewards: i64 = row.try_get("total_rewards")?;
            let max_checkpoint: i64 = row.try_get("max_checkpoint")?;

            fallback_aggregations.push(WorkerRewardsAggregation {
                bucket,
                public_key,
                completed_proofs,
                total_rewards,
                max_checkpoint,
            });
        }

        if fallback_aggregations.is_empty() {
            tracing::warn!(
                "Could not create aggregations from raw data for worker {} with interval {}",
                worker_public_key,
                interval
            );

            // As a last resort, try to get a single aggregation of all data
            tracing::info!("Attempting to create a single aggregation bucket for all data");

            let single_bucket_query = r#"
                SELECT
                    date_trunc('day', MIN(timestamp)) AS bucket,
                    public_key,
                    COUNT(*)::BIGINT as completed_proofs,
                    COALESCE(SUM(reward_amount), 0)::BIGINT as total_rewards,
                    COALESCE(MAX(checkpoint_id), 0)::BIGINT as max_checkpoint
                FROM worker_event_rewards
                WHERE public_key = $1
                GROUP BY public_key
                HAVING COUNT(*) > 0
            "#;

            let single_row = sqlx::query(single_bucket_query)
                .bind(worker_public_key)
                .fetch_optional(pool)
                .await?;

            if let Some(row) = single_row {
                let bucket: DateTime<Utc> = row.try_get("bucket")?;
                let public_key: String = row.try_get("public_key")?;
                let completed_proofs: i64 = row.try_get("completed_proofs")?;
                let total_rewards: i64 = row.try_get("total_rewards")?;
                let max_checkpoint: i64 = row.try_get("max_checkpoint")?;

                let aggregation = WorkerRewardsAggregation {
                    bucket,
                    public_key,
                    completed_proofs,
                    total_rewards,
                    max_checkpoint,
                };

                tracing::info!(
                    "Created single aggregation bucket with {} proofs and {} total rewards",
                    aggregation.completed_proofs,
                    aggregation.total_rewards
                );
                return Ok(vec![aggregation]);
            }
        } else {
            tracing::info!(
                "Successfully created {} aggregation buckets from raw data for worker {}",
                fallback_aggregations.len(),
                worker_public_key
            );
        }

        Ok(fallback_aggregations)
    }

    /// Force refresh a continuous aggregate (useful for testing and manual intervention)
    pub async fn refresh_aggregate(
        pool: &PgPool,
        view_name: &str,
    ) -> Result<()> {
        // Validate view name to prevent SQL injection
        let valid_views = vec!["worker_rewards_1d", "worker_rewards_1w", "worker_rewards_1m"];
        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        tracing::info!("Manually refreshing continuous aggregate: {}", view_name);

        // Use CALL to invoke the refresh procedure
        let query = format!(
            "CALL refresh_continuous_aggregate('{}', NULL, NULL)",
            view_name
        );

        sqlx::query(&query)
            .execute(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to refresh aggregate {}: {}", view_name, e);
                anyhow::anyhow!("Failed to refresh aggregate {}: {}", view_name, e)
            })?;

        tracing::info!("Successfully refreshed continuous aggregate: {}", view_name);
        Ok(())
    }

    /// Check if a continuous aggregate has any data
    pub async fn check_aggregate_has_data(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
    ) -> Result<bool> {
        let valid_views = vec!["worker_rewards_1d", "worker_rewards_1w", "worker_rewards_1m"];
        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE public_key = $1) as has_data",
            view_name
        );

        let result = sqlx::query_scalar::<_, bool>(&query)
            .bind(worker_public_key)
            .fetch_one(pool)
            .await?;

        Ok(result)
    }
}