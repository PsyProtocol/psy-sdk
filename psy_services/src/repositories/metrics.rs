use chrono::Utc;
use sqlx::PgPool;

use crate::{
    models::{TpsData, UserEvent, UserEventTxType, WorkerLeaderboardEntry},
    Result,
};

pub struct TpsRepository;
pub struct WorkerLeaderboardRepository;

/// TPS Repository
impl TpsRepository {
    /// Get the maximum checkpoint ID from worker_events table
    pub async fn get_max_checkpoint(pool: &PgPool) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            SELECT MAX(checkpoint_id) as max_checkpoint
            FROM worker_events
            "#
        )
        .fetch_one(pool)
        .await?;

        Ok(row.max_checkpoint.unwrap_or(0))
    }

    /// Calculate current TPS based on the last 12 seconds of user events
    pub async fn calculate_current_tps(pool: &PgPool) -> Result<TpsData> {
        let now = Utc::now();
        let twelve_seconds_ago = now - chrono::Duration::seconds(12);
        const TIME_WINDOW_SECONDS: i64 = 12;

        // Query all user_events from the last 12 seconds with their metadata
        let events = sqlx::query_as!(
            UserEvent,
            r#"
            SELECT
                user_id, public_key, tx_type as "tx_type: UserEventTxType",
                metadata, timestamp, created_at, updated_at
            FROM user_events
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp DESC
            "#,
            twelve_seconds_ago,
            now
        )
        .fetch_all(pool)
        .await?;

        // Calculate total transaction count by examining each event individually
        let mut total_transaction_count = 0i64;

        for event in &events {
            total_transaction_count += event.get_transaction_count();
        }

        // Calculate TPS
        let tps = total_transaction_count as f64 / TIME_WINDOW_SECONDS as f64;

        // Get the current block height (max checkpoint)
        let block_height = Self::get_max_checkpoint(pool).await?;

        Ok(TpsData {
            tps,
            transaction_count: total_transaction_count,
            time_window_seconds: TIME_WINDOW_SECONDS,
            block_height,
            timestamp: now,
        })
    }
}

/// Worker Leaderboard Repository
impl WorkerLeaderboardRepository {
    /// Get worker leaderboard for the last 24 hours
    /// Returns top workers ranked by total rewards earned, limited to specified
    /// count
    // In metrics.rs - WorkerLeaderboardRepository::get_leaderboard_24h()

    pub async fn get_leaderboard_24h(pool: &PgPool, limit: i64) -> Result<Vec<WorkerLeaderboardEntry>> {
        let twenty_four_hours_ago = Utc::now() - chrono::Duration::hours(24);

        // Query the worker_rewards_1d continuous aggregate for actual reward data
        // This pulls from checkpoint_reward_distributions which has the real calculated
        // rewards
        let rows = sqlx::query!(
            r#"
            WITH worker_rewards_24h AS (
                SELECT
                    worker_public_key,
                    SUM(completed_proofs)::BIGINT as proofs_24h,
                    SUM(total_rewards)::BIGINT as rewards_24h
                FROM worker_rewards_1d
                WHERE bucket >= $1
                GROUP BY worker_public_key
                HAVING SUM(total_rewards) > 0
            ),
            ranked_workers AS (
                SELECT
                    wr.worker_public_key,
                    ui.twitter_handle as twitter_username,
                    wr.proofs_24h,
                    wr.rewards_24h,
                    ROW_NUMBER() OVER (ORDER BY wr.rewards_24h DESC, wr.worker_public_key ASC) as rank
                FROM worker_rewards_24h wr
                LEFT JOIN user_info ui ON wr.worker_public_key = ui.public_key
            )
            SELECT
                worker_public_key,
                twitter_username,
                proofs_24h,
                rewards_24h,
                rank
            FROM ranked_workers
            WHERE rank <= $2
            ORDER BY rank ASC
            "#,
            twenty_four_hours_ago,
            limit
        )
        .fetch_all(pool)
        .await?;

        let leaderboard: Vec<WorkerLeaderboardEntry> = rows
            .into_iter()
            .filter_map(|row| {
                // All these fields should be non-null due to the query structure,
                // but sqlx infers them as Option. We filter out any unexpected nulls.
                Some(WorkerLeaderboardEntry {
                    worker_public_key: row.worker_public_key?,
                    twitter_username: row.twitter_username,
                    proofs_24h: row.proofs_24h?,
                    rewards_24h: row.rewards_24h?,
                    rank: row.rank?,
                })
            })
            .collect();

        Ok(leaderboard)
    }
}
