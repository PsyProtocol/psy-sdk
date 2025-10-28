// Alternative implementation using runtime-checked queries
// repositories/job_status.rs

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use crate::models::{JobStatusSummary, RealmJobStatusSummary};
use crate::Result;

pub struct JobStatusRepository;

impl JobStatusRepository {
    /// Get job status summary from the materialized view
    pub async fn get_job_status_summary(pool: &PgPool) -> Result<Vec<JobStatusSummary>> {
        let query = r#"
            SELECT
                status,
                COUNT(*)::BIGINT as job_count,
                ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::FLOAT8 as percentage,
                MAX(timestamp) as last_update
            FROM latest_job_status
            GROUP BY status
            ORDER BY
                CASE status
                    WHEN 'PENDING' THEN 1
                    WHEN 'PROCESSING' THEN 2
                    WHEN 'COMPLETED' THEN 3
                    WHEN 'FAILED' THEN 4
                    ELSE 5
                END
        "#;

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await?;

        let summaries = rows
            .into_iter()
            .map(|row| {
                JobStatusSummary {
                    status: row.get("status"),
                    job_count: row.get("job_count"),
                    percentage: row.try_get("percentage").ok(),
                    last_update: row.try_get("last_update").ok(),
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Get job status summary within a time window
    pub async fn get_job_status_summary_with_time_window(
        pool: &PgPool,
        since: DateTime<Utc>,
    ) -> Result<Vec<JobStatusSummary>> {
        let query = r#"
            SELECT
                status,
                COUNT(*)::BIGINT as job_count,
                ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::FLOAT8 as percentage,
                MAX(timestamp) as last_update
            FROM latest_job_status
            WHERE timestamp >= $1
            GROUP BY status
            ORDER BY
                CASE status
                    WHEN 'PENDING' THEN 1
                    WHEN 'PROCESSING' THEN 2
                    WHEN 'COMPLETED' THEN 3
                    WHEN 'FAILED' THEN 4
                    ELSE 5
                END
        "#;

        let rows = sqlx::query(query)
            .bind(since)
            .fetch_all(pool)
            .await?;

        let summaries = rows
            .into_iter()
            .map(|row| {
                JobStatusSummary {
                    status: row.get("status"),
                    job_count: row.get("job_count"),
                    percentage: row.try_get("percentage").ok(),
                    last_update: row.try_get("last_update").ok(),
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Get job status summary by realm
    pub async fn get_job_status_summary_by_realm(
        pool: &PgPool,
        realm_id: Option<i64>,
    ) -> Result<Vec<JobStatusSummary>> {
        let query = if realm_id.is_some() {
            r#"
                SELECT
                    status,
                    COUNT(*)::BIGINT as job_count,
                    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::FLOAT8 as percentage,
                    MAX(timestamp) as last_update
                FROM latest_job_status
                WHERE realm_id = $1
                GROUP BY status
                ORDER BY
                    CASE status
                        WHEN 'PENDING' THEN 1
                        WHEN 'PROCESSING' THEN 2
                        WHEN 'COMPLETED' THEN 3
                        WHEN 'FAILED' THEN 4
                        ELSE 5
                    END
            "#
        } else {
            r#"
                SELECT
                    status,
                    COUNT(*)::BIGINT as job_count,
                    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::FLOAT8 as percentage,
                    MAX(timestamp) as last_update
                FROM latest_job_status
                WHERE realm_id IS NULL
                GROUP BY status
                ORDER BY
                    CASE status
                        WHEN 'PENDING' THEN 1
                        WHEN 'PROCESSING' THEN 2
                        WHEN 'COMPLETED' THEN 3
                        WHEN 'FAILED' THEN 4
                        ELSE 5
                    END
            "#
        };

        let rows = if let Some(realm_id) = realm_id {
            sqlx::query(query)
                .bind(realm_id)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(query)
                .fetch_all(pool)
                .await?
        };

        let summaries = rows
            .into_iter()
            .map(|row| {
                JobStatusSummary {
                    status: row.get("status"),
                    job_count: row.get("job_count"),
                    percentage: row.try_get("percentage").ok(),
                    last_update: row.try_get("last_update").ok(),
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Get all realm job status summaries
    pub async fn get_all_realm_job_status_summary(
        pool: &PgPool,
    ) -> Result<Vec<RealmJobStatusSummary>> {
        let query = r#"
            SELECT
                realm_id,
                status,
                COUNT(*)::BIGINT as job_count,
                ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (PARTITION BY realm_id), 0), 2)::FLOAT8 as percentage,
                MAX(timestamp) as last_update
            FROM latest_job_status
            GROUP BY realm_id, status
            ORDER BY realm_id,
                CASE status
                    WHEN 'PENDING' THEN 1
                    WHEN 'PROCESSING' THEN 2
                    WHEN 'COMPLETED' THEN 3
                    WHEN 'FAILED' THEN 4
                    ELSE 5
                END
        "#;

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await?;

        let summaries = rows
            .into_iter()
            .map(|row| {
                RealmJobStatusSummary {
                    realm_id: row.try_get("realm_id").ok(),
                    status: row.get("status"),
                    job_count: row.get("job_count"),
                    percentage: row.try_get("percentage").ok(),
                    last_update: row.try_get("last_update").ok(),
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Get count of jobs by status (simplified query)
    pub async fn get_job_counts_by_status(pool: &PgPool) -> Result<std::collections::HashMap<String, i64>> {
        use std::collections::HashMap;

        let query = r#"
            SELECT
                status,
                COUNT(*)::BIGINT as count
            FROM latest_job_status
            GROUP BY status
        "#;

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await?;

        let mut counts = HashMap::new();
        for row in rows {
            let status: String = row.get("status");
            let count: i64 = row.get("count");
            counts.insert(status, count);
        }

        // Ensure all statuses are present
        for status in &["PENDING", "PROCESSING", "COMPLETED", "FAILED"] {
            counts.entry(status.to_string()).or_insert(0);
        }

        Ok(counts)
    }

    /// Check if the materialized view exists and has data
    pub async fn check_materialized_view_health(pool: &PgPool) -> Result<bool> {
        let query = r#"
            SELECT EXISTS (
                SELECT 1
                FROM pg_matviews
                WHERE matviewname = 'latest_job_status'
            ) as exists,
            (
                SELECT COUNT(*) > 0
                FROM latest_job_status
                LIMIT 1
            ) as has_data
        "#;

        let row = sqlx::query(query)
            .fetch_one(pool)
            .await?;

        let exists: bool = row.get("exists");
        let has_data: Option<bool> = row.try_get("has_data").ok();

        Ok(exists && has_data.unwrap_or(false))
    }
}