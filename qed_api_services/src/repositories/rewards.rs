use chrono::Utc;
use sqlx::PgPool;

use crate::models::{WorkerEventReward, WorkerRewards};
use crate::Result;

pub struct WorkerRewardsRepository;
// In rewards.rs - Update WorkerRewardsRepository to use new system

impl WorkerRewardsRepository {
    /// Get rewards for a specific worker by public key and checkpoint_id
    /// Updated to use checkpoint_reward_distributions instead of worker_event_rewards
    pub async fn get_worker_rewards(
        pool: &PgPool,
        worker_public_key: &str,
        checkpoint_id: i64,
    ) -> Result<WorkerRewards> {
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        let seven_days_ago = now - chrono::Duration::days(7);
        let thirty_days_ago = now - chrono::Duration::days(30);

        // Get rewards and proofs counts by checkpoint (claimed vs unclaimed)
        // "Claimed" = rewards from checkpoints before the specified checkpoint_id
        // "Unclaimed" = rewards from checkpoints >= the specified checkpoint_id
        let rewards_row = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT CASE WHEN checkpoint_id < $2 THEN job_id END)::BIGINT as claimed_proofs,
                COUNT(DISTINCT CASE WHEN checkpoint_id >= $2 THEN job_id END)::BIGINT as unclaimed_proofs,
                COUNT(DISTINCT job_id)::BIGINT as total_proofs,
                COALESCE(SUM(CASE WHEN checkpoint_id < $2 THEN reward_amount END), 0)::BIGINT as claimed_rewards,
                COALESCE(SUM(CASE WHEN checkpoint_id >= $2 THEN reward_amount END), 0)::BIGINT as unclaimed_rewards,
                COALESCE(SUM(reward_amount), 0)::BIGINT as total_rewards
            FROM checkpoint_reward_distributions
            WHERE worker_public_key = $1
            "#,
            worker_public_key,
            checkpoint_id
        )
            .fetch_one(pool)
            .await?;

        let claimed_proofs = rewards_row.claimed_proofs.unwrap_or(0);
        let unclaimed_proofs = rewards_row.unclaimed_proofs.unwrap_or(0);
        let total_proofs = rewards_row.total_proofs.unwrap_or(0);
        let claimed_rewards = rewards_row.claimed_rewards.unwrap_or(0);
        let unclaimed_rewards = rewards_row.unclaimed_rewards.unwrap_or(0);
        let total_rewards = rewards_row.total_rewards.unwrap_or(0);

        // Query for time-based rewards (24h, 7d, 30d) from worker_rewards views
        let total_rewards_24h = sqlx::query_scalar!(
                r#"
                SELECT COALESCE(SUM(total_rewards), 0)::BIGINT
                FROM worker_rewards_1d
                WHERE public_key = $1 AND bucket >= $2
                "#,
                worker_public_key,
                twenty_four_hours_ago
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

        let total_rewards_7d = sqlx::query_scalar!(
                r#"
                SELECT COALESCE(SUM(total_rewards), 0)::BIGINT
                FROM worker_rewards_1d
                WHERE public_key = $1 AND bucket >= $2
                "#,
                worker_public_key,
                seven_days_ago
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

        let total_rewards_30d = sqlx::query_scalar!(
                r#"
                SELECT COALESCE(SUM(total_rewards), 0)::BIGINT
                FROM worker_rewards_1d
                WHERE public_key = $1 AND bucket >= $2
                "#,
                worker_public_key,
                thirty_days_ago
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

        Ok(WorkerRewards {
            worker_public_key: worker_public_key.to_string(),
            checkpoint_id,
            claimed_rewards,
            unclaimed_rewards,
            total_rewards,
            claimed_proofs,
            unclaimed_proofs,
            total_proofs,
            total_rewards_24h,
            total_rewards_7d,
            total_rewards_30d,
            last_updated: now,
        })
    }
}