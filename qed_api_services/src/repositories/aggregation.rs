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
    /// Get worker rewards aggregations from continuous aggregates
    /// These aggregates now query checkpoint_reward_distributions for actual rewards
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<WorkerRewardsAggregation>> {
        // Validate view name to prevent SQL injection
        let valid_views = vec![
            "worker_rewards_1d",
            "worker_rewards_1w",
            "worker_rewards_1m",
            "worker_rewards"
        ];
        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        // All-time view has different structure (no bucket column)
        let aggregations = if view_name == "worker_rewards" {
            let query = r#"
                SELECT
                    last_reward_time as bucket,
                    worker_public_key as public_key,
                    jobs_completed::BIGINT as completed_proofs,
                    total_rewards::BIGINT as total_rewards,
                    max_checkpoint::BIGINT as max_checkpoint
                FROM worker_rewards
                WHERE worker_public_key = $1
                LIMIT 1
            "#;

            tracing::debug!(
                "Querying worker_rewards table for worker {}",
                worker_public_key
            );

            sqlx::query_as::<_, WorkerRewardsAggregation>(query)
                .bind(worker_public_key)
                .fetch_all(pool)
                .await?
        } else {
            let query = format!(
                r#"
                SELECT
                    bucket,
                    worker_public_key as public_key,
                    jobs_completed::BIGINT as completed_proofs,
                    total_rewards::BIGINT as total_rewards,
                    max_checkpoint::BIGINT as max_checkpoint
                FROM {}
                WHERE worker_public_key = $1
                    AND ($2::TIMESTAMPTZ IS NULL OR bucket >= $2)
                    AND ($3::TIMESTAMPTZ IS NULL OR bucket <= $3)
                ORDER BY bucket DESC
                LIMIT $4
                "#,
                view_name
            );

            tracing::debug!(
                "Querying continuous aggregate {} for worker {} (start: {:?}, end: {:?}, limit: {})",
                view_name,
                worker_public_key,
                start_time,
                end_time,
                limit
            );

            sqlx::query_as::<_, WorkerRewardsAggregation>(&query)
                .bind(worker_public_key)
                .bind(start_time)
                .bind(end_time)
                .bind(limit)
                .fetch_all(pool)
                .await?
        };

        if aggregations.is_empty() {
            tracing::info!(
                "No aggregation data found in {} for worker {}. This is normal if the worker has no reward-eligible jobs in checkpoint_reward_distributions.",
                view_name,
                worker_public_key
            );
        } else {
            tracing::info!(
                "Found {} aggregation buckets from {} for worker {}",
                aggregations.len(),
                view_name,
                worker_public_key
            );
        }
        Ok(aggregations)
    }

    /// Get total rewards for a worker across time range from aggregates
    pub async fn get_worker_rewards_with_period(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64)> {
        let valid_views = vec![
            "worker_rewards_1d",
            "worker_rewards_1w",
            "worker_rewards_1m",
        ];
        if !valid_views.contains(&view_name) {
            return Err(anyhow::anyhow!("Invalid view name: {}", view_name));
        }

        let query = format!(
            r#"
            SELECT
                COALESCE(SUM(jobs_completed), 0)::BIGINT as total_proofs,
                COALESCE(SUM(total_rewards), 0)::BIGINT as total_rewards
            FROM {}
            WHERE worker_public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR bucket >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket <= $3)
            "#,
            view_name
        );

        let row = sqlx::query(&query)
            .bind(worker_public_key)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(pool)
            .await?;

        let total_proofs: i64 = row.try_get("total_proofs")?;
        let total_rewards: i64 = row.try_get("total_rewards")?;

        Ok((total_proofs, total_rewards))
    }

    /// Get total rewards for a worker across all time buckets
    pub async fn get_worker_rewards_all(
        pool: &PgPool,
        worker_public_key: &str,
    ) -> Result<i64> {

        let view_name = "worker_rewards_all_time";
        let query = format!(
            r#"
            SELECT
                COALESCE(SUM(completed_proofs), 0)::BIGINT as total_proofs,
                COALESCE(SUM(total_rewards), 0)::BIGINT as total_rewards
            FROM {}
            WHERE public_key = $1
            "#,
            view_name
        );

        let row = sqlx::query(&query)
            .bind(worker_public_key)
            .fetch_one(pool)
            .await?;

        let total_rewards: i64 = row.try_get("total_rewards")?;

        Ok(total_rewards)
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