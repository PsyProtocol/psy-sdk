use chrono::Utc;
use sqlx::PgPool;

use crate::models::{WorkerEventReward, WorkerRewards};
use crate::Result;

pub struct WorkerRewardsRepository;
pub struct WorkerEventRewardRepository;

/// Worker Rewards Repository
impl WorkerRewardsRepository {
    /// Get rewards for a specific worker by public key and checkpoint_id
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
        let rewards_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN checkpoint_id < $2 THEN 1 END)::BIGINT as claimed_proofs,
                COUNT(CASE WHEN checkpoint_id >= $2 THEN 1 END)::BIGINT as unclaimed_proofs,
                COUNT(*)::BIGINT as total_proofs,
                COALESCE(SUM(CASE WHEN checkpoint_id < $2 THEN reward_amount END), 0)::BIGINT as claimed_rewards,
                COALESCE(SUM(CASE WHEN checkpoint_id >= $2 THEN reward_amount END), 0)::BIGINT as unclaimed_rewards,
                COALESCE(SUM(reward_amount), 0)::BIGINT as total_rewards
            FROM worker_event_rewards
            WHERE public_key = $1
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

        // Query for time-based rewards (24h, 7d, 30d) from worker_event_rewards
        let time_rewards_row = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN timestamp >= $2 THEN 1 END)::BIGINT as proofs_24h,
                COUNT(CASE WHEN timestamp >= $3 THEN 1 END)::BIGINT as proofs_7d,
                COUNT(CASE WHEN timestamp >= $4 THEN 1 END)::BIGINT as proofs_30d,
                COALESCE(SUM(CASE WHEN timestamp >= $2 THEN reward_amount END), 0)::BIGINT as total_rewards_24h,
                COALESCE(SUM(CASE WHEN timestamp >= $3 THEN reward_amount END), 0)::BIGINT as total_rewards_7d,
                COALESCE(SUM(CASE WHEN timestamp >= $4 THEN reward_amount END), 0)::BIGINT as total_rewards_30d
            FROM worker_event_rewards
            WHERE public_key = $1
            "#,
            worker_public_key,
            twenty_four_hours_ago,
            seven_days_ago,
            thirty_days_ago
        )
        .fetch_one(pool)
        .await?;

        let total_rewards_24h = time_rewards_row.total_rewards_24h.unwrap_or(0);
        let total_rewards_7d = time_rewards_row.total_rewards_7d.unwrap_or(0);
        let total_rewards_30d = time_rewards_row.total_rewards_30d.unwrap_or(0);

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

/// Worker Event Reward Repository
impl WorkerEventRewardRepository {
    /// Insert worker event rewards
    pub async fn create_rewards(pool: &PgPool, rewards: &[WorkerEventReward]) -> Result<()> {
        for reward in rewards {
            sqlx::query!(
                r#"
                INSERT INTO worker_event_rewards
                    (id, public_key, checkpoint_id, reward_amount, timestamp)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (id) DO NOTHING
                "#,
                reward.id,
                reward.public_key,
                reward.checkpoint_id,
                reward.reward_amount,
                reward.timestamp
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Check if a worker event already has a reward
    pub async fn has_reward(pool: &PgPool, worker_event_id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM worker_event_rewards
                WHERE id = $1
            ) as exists
            "#,
            worker_event_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Get rewards for worker events by event IDs
    pub async fn get_rewards_by_event_ids(
        pool: &PgPool,
        worker_event_ids: &[uuid::Uuid],
    ) -> Result<Vec<WorkerEventReward>> {
        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount, timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE id = ANY($1)
            ORDER BY timestamp DESC
            "#,
            worker_event_ids
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }

    /// Get all rewards for worker events in a checkpoint range
    pub async fn get_rewards_by_checkpoint_range(
        pool: &PgPool,
        start_checkpoint: i64,
        end_checkpoint: i64,
    ) -> Result<Vec<WorkerEventReward>> {
        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount,
                   timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE checkpoint_id >= $1 AND checkpoint_id <= $2
            ORDER BY checkpoint_id DESC, timestamp DESC
            "#,
            start_checkpoint,
            end_checkpoint
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }

    /// Get rewards for a specific worker (by public_key) in a checkpoint range
    pub async fn get_worker_rewards(
        pool: &PgPool,
        public_key: &str,
        start_checkpoint: Option<i64>,
        end_checkpoint: Option<i64>,
    ) -> Result<Vec<WorkerEventReward>> {
        let start = start_checkpoint.unwrap_or(0);
        let end = end_checkpoint.unwrap_or(i64::MAX);

        let rewards = sqlx::query_as!(
            WorkerEventReward,
            r#"
            SELECT id, public_key, checkpoint_id, reward_amount,
                   timestamp, created_at, updated_at
            FROM worker_event_rewards
            WHERE public_key = $1
                AND checkpoint_id >= $2
                AND checkpoint_id <= $3
            ORDER BY checkpoint_id DESC, timestamp DESC
            "#,
            public_key,
            start,
            end
        )
        .fetch_all(pool)
        .await?;

        Ok(rewards)
    }
}
