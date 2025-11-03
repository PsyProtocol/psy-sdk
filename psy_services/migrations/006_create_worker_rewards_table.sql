-- 006_create_worker_rewards_table.sql
-- Aggregate table for worker rewards
-- This table maintains running totals per worker, updated when checkpoint rewards are distributed

CREATE TABLE worker_rewards (
    worker_public_key VARCHAR(128) NOT NULL PRIMARY KEY,
    total_jobs_completed BIGINT NOT NULL DEFAULT 0,
    total_rewards BIGINT NOT NULL DEFAULT 0,
    checkpoints_participated BIGINT NOT NULL DEFAULT 0,
    first_reward_time TIMESTAMPTZ,
    last_reward_time TIMESTAMPTZ,
    max_checkpoint BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_worker_rewards_total_rewards ON worker_rewards(total_rewards DESC);
CREATE INDEX idx_worker_rewards_last_reward_time ON worker_rewards(last_reward_time DESC);
CREATE INDEX idx_worker_rewards_max_checkpoint ON worker_rewards(max_checkpoint DESC);

-- Create trigger for updated_at
CREATE TRIGGER update_worker_rewards_updated_at
    BEFORE UPDATE ON worker_rewards
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Add comments for documentation
COMMENT ON TABLE worker_rewards IS 'Aggregate table tracking total rewards per worker. Updated automatically when checkpoint rewards are distributed.';
COMMENT ON COLUMN worker_rewards.worker_public_key IS 'Worker public key (primary key)';
COMMENT ON COLUMN worker_rewards.total_jobs_completed IS 'Total number of jobs completed by this worker across all time';
COMMENT ON COLUMN worker_rewards.total_rewards IS 'Total rewards earned by this worker across all time';
COMMENT ON COLUMN worker_rewards.checkpoints_participated IS 'Number of unique checkpoints this worker participated in';
COMMENT ON COLUMN worker_rewards.first_reward_time IS 'Timestamp of first reward received';
COMMENT ON COLUMN worker_rewards.last_reward_time IS 'Timestamp of most recent reward received';
COMMENT ON COLUMN worker_rewards.max_checkpoint IS 'Latest checkpoint ID where worker received rewards';