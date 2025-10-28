use chrono::Utc;
use sqlx::PgPool;

use crate::{
    models::{GlobalRealmStats, RealmStats, WorkerStats},
    Result,
};

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

        let username = username_row.and_then(|row| row.twitter_handle).filter(|s| !s.is_empty());

        // Get processing tasks count grouped by realm_id
        let processing_tasks_rows = sqlx::query!(
            r#"
            SELECT realm_id, COUNT(*) as task_count
            FROM worker_events
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
            let realm_key = match row.realm_id {
                Some(id) => format!("realm{}", id),
                None => "realm_unknown".to_string(),
            };
            let count = row.task_count.unwrap_or(0);
            processing_tasks.insert(realm_key, count);
            total_processing_tasks += count;
        }

        // Get completed and failed tasks in the last 24h
        let completion_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as completed_24h,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed_24h,
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as total_proofs
            FROM worker_events
            WHERE public_key = $1 AND timestamp >= $2
            "#,
            worker_public_key,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let completed_24h = completion_stats_row.completed_24h.unwrap_or(0);
        let failed_24h = completion_stats_row.failed_24h.unwrap_or(0);
        let total_proofs = completion_stats_row.total_proofs.unwrap_or(0);

        // Get completed and failed tasks in the last 1h
        let completion_1h_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as completed_1h,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed_1h
            FROM worker_events
            WHERE public_key = $1 AND timestamp >= $2
            "#,
            worker_public_key,
            one_hour_ago
        )
        .fetch_one(pool)
        .await?;

        let completed_1h = completion_1h_stats_row.completed_1h.unwrap_or(0);
        let failed_1h = completion_1h_stats_row.failed_1h.unwrap_or(0);

        // Get total completed and failed tasks of all time
        let total_completion_stats_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as total_completed,
                COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as total_failed
            FROM worker_events
            WHERE public_key = $1
            "#,
            worker_public_key
        )
        .fetch_one(pool)
        .await?;

        let total_completed = total_completion_stats_row.total_completed.unwrap_or(0);
        let total_failed = total_completion_stats_row.total_failed.unwrap_or(0);

        // Calculate total rewards in the last 24 hours
        // Only count rewards for GenerateStandardProof jobs (topic = 0) with COMPLETED
        // status
        const REWARD_PER_PROOF: i64 = 5_000_000_000; // 5*10^9 psy
        const TOPIC_GENERATE_STANDARD_PROOF: i16 = 0;

        let rewards_24h_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as reward_proofs_24h
            FROM worker_events
            WHERE public_key = $1
                AND topic = $2
                AND status = 'COMPLETED'
                AND timestamp >= $3
            "#,
            worker_public_key,
            TOPIC_GENERATE_STANDARD_PROOF,
            twenty_four_hours_ago
        )
        .fetch_one(pool)
        .await?;

        let reward_proofs_24h = rewards_24h_row.reward_proofs_24h.unwrap_or(0);
        let total_rewards_24h = reward_proofs_24h * REWARD_PER_PROOF;

        // Calculate total rewards based on GUTA completed events
        // Assuming the job_id contains information about the event type
        // or you have a specific way to identify GUTA events
        let total_rewards_row = sqlx::query!(
            r#"
            SELECT COUNT(*) as reward_count
            FROM worker_events
            WHERE public_key = $1
                AND topic = $2
                AND status = 'COMPLETED'
            "#,
            worker_public_key,
            TOPIC_GENERATE_STANDARD_PROOF,
        )
        .fetch_one(pool)
        .await?;

        let total_rewards_proofs = total_rewards_row.reward_count.unwrap_or(0);
        let total_rewards = total_rewards_proofs * REWARD_PER_PROOF;

        // Calculate average proof time from completed tasks
        let avg_proof_time_row = sqlx::query!(
            r#"
            SELECT COALESCE(AVG(duration), 0)::BIGINT as avg_duration
            FROM worker_events
            WHERE public_key = $1
                AND status = 'COMPLETED'
                AND duration IS NOT NULL
            "#,
            worker_public_key
        )
        .fetch_one(pool)
        .await?;

        let avg_proof_time = avg_proof_time_row.avg_duration.unwrap_or(0);

        // Get the block height of the last completed worker event
        let last_completed_block_height_row = sqlx::query!(
            r#"
            SELECT checkpoint_id as block_height
            FROM worker_events
            WHERE public_key = $1 AND status = 'COMPLETED'
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            worker_public_key
        )
        .fetch_optional(pool)
        .await?;

        let last_completed_block_height = last_completed_block_height_row.map(|row| row.block_height);

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
