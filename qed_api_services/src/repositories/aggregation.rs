use chrono::{DateTime, Utc};
use sqlx::PgPool;

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
        limit: i64,
    ) -> Result<Vec<WorkerEventAggregation>> {
        // Note: view_name should be validated against a whitelist in production
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
            ORDER BY bucket DESC
            LIMIT $5
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, WorkerEventAggregation>(&query)
            .bind(realm_id)
            .bind(source)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
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
        limit: i64,
    ) -> Result<Vec<UserEventAggregation>> {
        let query = format!(
            r#"
            SELECT
                bucket, count, register_user_count, deploy_contract_count, guta_count
            FROM {}
            WHERE ($1::TIMESTAMPTZ IS NULL OR bucket >= $1)
                AND ($2::TIMESTAMPTZ IS NULL OR bucket <= $2)
            ORDER BY bucket DESC
            LIMIT $3
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, UserEventAggregation>(&query)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}

/// Worker Rewards Aggregation Repository
impl WorkerRewardsAggregationRepository {
    /// Get worker rewards aggregations from materialized views
    /// Note: Uses dynamic query for flexible view selection
    pub async fn get_aggregations(
        pool: &PgPool,
        view_name: &str,
        worker_public_key: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<WorkerRewardsAggregation>> {
        // Note: view_name should be validated against a whitelist in production
        let query = format!(
            r#"
            SELECT
                bucket, public_key, completed_proofs, total_rewards, max_checkpoint
            FROM {}
            WHERE public_key = $1
                AND ($2::TIMESTAMPTZ IS NULL OR bucket >= $2)
                AND ($3::TIMESTAMPTZ IS NULL OR bucket <= $3)
            ORDER BY bucket DESC
            LIMIT $4
            "#,
            view_name
        );

        let aggregations = sqlx::query_as::<_, WorkerRewardsAggregation>(&query)
            .bind(worker_public_key)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .fetch_all(pool)
            .await?;

        Ok(aggregations)
    }
}