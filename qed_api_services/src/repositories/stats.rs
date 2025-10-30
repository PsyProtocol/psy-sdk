use chrono::Utc;
use sqlx::PgPool;

use crate::models::{GlobalRealmStats, RealmStats, WorkerStats};
use crate::repositories::WorkerRewardsAggregationRepository;
use crate::Result;

pub struct RealmStatsRepository;
pub struct WorkerStatsRepository;

/// Realm Statistics Repository
impl RealmStatsRepository {
    /// Get statistics for a specific realm
    pub async fn get_realm_stats(pool: &PgPool, realm_id: i64) -> Result<RealmStats> {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        // Get processing tasks count directly from worker_events (real-time)
        let processing_tasks_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as processing_tasks
            FROM worker_events
            WHERE realm_id = $1 AND status = 'PROCESSING'
            "#,
            realm_id
        )
        .fetch_one(pool)
        .await?;

        let processing_tasks = processing_tasks_row.processing_tasks.unwrap_or(0);

        // Get active workers and users for 1h (direct query)
        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_1h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_1h
            FROM worker_events
            WHERE realm_id = $1 AND timestamp >= $2
            "#,
            realm_id,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_1h = active_1h_row.active_workers_1h.unwrap_or(0);
        let active_users_1h = active_1h_row.active_users_1h.unwrap_or(0);

        // Get active workers and users for 24h (direct query)
        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_24h,
                COUNT(DISTINCT CASE WHEN source = 'REALM' THEN public_key END) as active_users_24h
            FROM worker_events
            WHERE realm_id = $1 AND timestamp >= $2
            "#,
            realm_id,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_24h = active_24h_row.active_workers_24h.unwrap_or(0);
        let active_users_24h = active_24h_row.active_users_24h.unwrap_or(0);

        Ok(RealmStats {
            realm_id,
            processing_tasks,
            active_workers_1h,
            active_workers_24h,
            active_users_1h,
            active_users_24h,
            last_updated: now,
        })
    }

    /// Get global statistics across all realms
    pub async fn get_global_realm_stats(pool: &PgPool) -> Result<GlobalRealmStats> {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        // Get total processing tasks across all realms (real-time)
        let total_processing_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as total_processing_tasks
            FROM worker_events
            WHERE status = 'PROCESSING'
            "#
        )
        .fetch_one(pool)
        .await?;

        let total_processing_tasks = total_processing_row.total_processing_tasks.unwrap_or(0);

        // Get 1h stats (direct query)
        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT realm_id) FILTER (WHERE realm_id IS NOT NULL) as active_realms_1h,
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_1h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;
        let active_workers_1h = active_1h_row.active_workers_1h.unwrap_or(0);
        let active_realms_1h = active_1h_row.active_realms_1h.unwrap_or(0);

        let active_1h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_users_1h
            FROM user_events
            WHERE timestamp >= $1
            "#,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;
        let active_users_1h = active_1h_row.active_users_1h.unwrap_or(0);

        // Get 24h stats (direct query)
        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT realm_id) FILTER (WHERE realm_id IS NOT NULL) as active_realms_24h,
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_workers_24h
            FROM worker_events
            WHERE timestamp >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let active_workers_24h = active_24h_row.active_workers_24h.unwrap_or(0);
        let active_realms_24h = active_24h_row.active_realms_24h.unwrap_or(0);

        let active_24h_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT public_key) FILTER (WHERE public_key IS NOT NULL) as active_users_24h
            FROM user_events
            WHERE timestamp >= $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;
        let active_users_24h = active_24h_row.active_users_24h.unwrap();

        Ok(GlobalRealmStats {
            total_processing_tasks,
            active_workers_1h,
            active_workers_24h,
            active_users_1h,
            active_users_24h,
            active_realms_1h,
            active_realms_24h,
            last_updated: now,
        })
    }
}

/// Worker Statistics Repository
impl WorkerStatsRepository {
    /// Get statistics for a specific worker by public key
    pub async fn get_worker_stats(pool: &PgPool, worker_public_key: &str) -> Result<WorkerStats> {
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        let one_hour_ago = now - chrono::Duration::hours(1);

        // Get username (twitter_handle) from user_info table by matching public_key
        let username_row = sqlx::query!(
            r#"
            SELECT twitter_handle
            FROM user_info
            WHERE public_key = $1
            "#,
            worker_public_key
        )
        .fetch_optional(pool)
        .await?;

        let username = username_row
            .and_then(|row| row.twitter_handle)
            .filter(|s| !s.is_empty());

        // Get PROCESSING and PENDING tasks count from latest_job_status materialized view
        // This gives accurate current state without double-counting from worker_events audit trail
        let processing_tasks_rows = sqlx::query!(
                r#"
                SELECT realm_id, COUNT(*) as task_count
                FROM latest_job_status
                WHERE public_key = $1 AND status = 'PROCESSING'
                GROUP BY realm_id
                "#,
                worker_public_key
            )
            .fetch_all(pool)
            .await?;

        let mut processing_tasks = std::collections::HashMap::new();
        let mut total_processing_tasks = 0i64;

        for row in processing_tasks_rows {
            if let Some(realm_id) = row.realm_id {
                let count = row.task_count.unwrap_or(0);
                processing_tasks.insert(realm_id.to_string(), count);
                total_processing_tasks += count;
            }
        }

        // Get 24-hour rewards from checkpoint_rewards_1d continuous aggregate
        // This contains actual calculated rewards from checkpoint_reward_distributions
        let rewards_24h_row = sqlx::query!(
                r#"
                SELECT
                    COALESCE(SUM(completed_proofs), 0)::BIGINT as total_proofs,
                    COALESCE(SUM(total_rewards), 0)::BIGINT as total_rewards_24h
                FROM worker_rewards_1d
                WHERE public_key = $1
                    AND bucket >= $2
                "#,
                worker_public_key,
                twenty_four_hours_ago
            )
            .fetch_one(pool)
            .await?;

        let total_proofs = rewards_24h_row.total_proofs.unwrap_or(0);
        let total_rewards_24h = rewards_24h_row.total_rewards_24h.unwrap_or(0);

        // Get all-time total rewards from worker_rewards_all_time aggregate
        // This is refreshed hourly and much more efficient than querying raw table
        let total_rewards =
            WorkerRewardsAggregationRepository::get_worker_rewards_all(pool, worker_public_key)
                .await
                .unwrap_or(0);

        // Get completed and failed counts for different time windows from latest_job_status
        // Use worker_events for historical time-based queries since latest_job_status only has current state
        let time_window_stats = sqlx::query!(
                r#"
                SELECT
                    COUNT(CASE WHEN status = 'COMPLETED' AND timestamp >= $2 THEN 1 END)::BIGINT as completed_24h,
                    COUNT(CASE WHEN status = 'FAILED' AND timestamp >= $2 THEN 1 END)::BIGINT as failed_24h,
                    COUNT(CASE WHEN status = 'COMPLETED' AND timestamp >= $3 THEN 1 END)::BIGINT as completed_1h,
                    COUNT(CASE WHEN status = 'FAILED' AND timestamp >= $3 THEN 1 END)::BIGINT as failed_1h,
                    COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END)::BIGINT as total_completed,
                    COUNT(CASE WHEN status = 'FAILED' THEN 1 END)::BIGINT as total_failed
                FROM worker_events
                WHERE public_key = $1
                "#,
                worker_public_key,
                twenty_four_hours_ago,
                one_hour_ago
            )
            .fetch_one(pool)
            .await?;

        let completed_24h = time_window_stats.completed_24h.unwrap_or(0);
        let failed_24h = time_window_stats.failed_24h.unwrap_or(0);
        let completed_1h = time_window_stats.completed_1h.unwrap_or(0);
        let failed_1h = time_window_stats.failed_1h.unwrap_or(0);
        let total_completed = time_window_stats.total_completed.unwrap_or(0);
        let total_failed = time_window_stats.total_failed.unwrap_or(0);

        // Calculate average proof time from completed tasks
        let avg_proof_time_row = sqlx::query!(
                r#"
                SELECT CAST(AVG(duration) as BIGINT) as avg_duration
                FROM worker_events
                WHERE public_key = $1
                    AND status = 'COMPLETED'
                    AND duration IS NOT NULL
                    AND duration > 0
                "#,
                worker_public_key
            )
            .fetch_one(pool)
            .await?;

        let avg_proof_time = avg_proof_time_row.avg_duration.unwrap_or(0);

        // Get last completed block height (checkpoint_id)
        let last_block_row = sqlx::query!(
                r#"
                SELECT checkpoint_id
                FROM worker_events
                WHERE public_key = $1 AND status = 'COMPLETED'
                ORDER BY checkpoint_id DESC, timestamp DESC
                LIMIT 1
                "#,
                worker_public_key
            )
            .fetch_optional(pool)
            .await?;

        let last_completed_block_height = last_block_row
            .and_then(|row| Some(row.checkpoint_id));

        Ok(WorkerStats {
            public_key: worker_public_key.to_string(),
            username,
            processing_tasks,
            total_processing_tasks,
            total_rewards,
            total_proofs,
            completed_24h,
            failed_24h,
            total_rewards_24h,
            total_completed,
            total_failed,
            completed_1h,
            failed_1h,
            avg_proof_time,
            last_completed_block_height,
            last_updated: now,
        })
    }
}