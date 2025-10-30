-- 011_create_checkpoint_reward_distributions_table.sql
-- Stores the calculated reward distribution for EACH JOB at each checkpoint
-- Each record represents a single job's reward, NOT aggregated by worker
-- Only populated when checkpoint has non-zero fees_collected

CREATE TABLE checkpoint_reward_distributions (
     id UUID DEFAULT uuid_generate_v4(),
     checkpoint_id BIGINT NOT NULL,
     worker_public_key VARCHAR(128) NOT NULL,  -- Worker receiving the reward for this job
     job_id UUID NOT NULL,                     -- Reference to worker_job_events.id (one record per job)
     reward_amount BIGINT NOT NULL,            -- Calculated reward for THIS SINGLE JOB: (total_fees / total_jobs)
     total_fees_at_checkpoint BIGINT NOT NULL, -- Total fees collected at this checkpoint (for reference)
     total_jobs_at_checkpoint BIGINT NOT NULL, -- Total jobs completed at this checkpoint (for reference)
     metadata JSONB DEFAULT '{}'::jsonb,       -- Flexible metadata
     timestamp TIMESTAMPTZ NOT NULL,           -- Distribution calculation timestamp
     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite primary key including the partition key
     PRIMARY KEY (id, timestamp),

    -- Foreign keys
     CONSTRAINT fk_checkpoint_reward_dist_checkpoint
         FOREIGN KEY (checkpoint_id) REFERENCES checkpoint_stats(checkpoint_id)
             ON DELETE CASCADE,

    -- Constraints
     CONSTRAINT chk_reward_amount_positive CHECK (reward_amount > 0),
     CONSTRAINT chk_total_fees_positive CHECK (total_fees_at_checkpoint > 0),
     CONSTRAINT chk_total_jobs_positive CHECK (total_jobs_at_checkpoint > 0),

    -- Ensure one distribution per job (job_id is unique per checkpoint)
     CONSTRAINT uq_checkpoint_job UNIQUE (job_id, timestamp)
);

-- Convert to hypertable for time-series data
SELECT create_hypertable('checkpoint_reward_distributions', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes for efficient querying
CREATE INDEX idx_checkpoint_reward_dist_worker ON checkpoint_reward_distributions(worker_public_key, timestamp DESC);
CREATE INDEX idx_checkpoint_reward_dist_checkpoint ON checkpoint_reward_distributions(checkpoint_id, timestamp DESC);
CREATE INDEX idx_checkpoint_reward_dist_worker_checkpoint ON checkpoint_reward_distributions(worker_public_key, checkpoint_id, timestamp DESC);
CREATE INDEX idx_checkpoint_reward_dist_job ON checkpoint_reward_distributions(job_id, timestamp DESC);

-- Create trigger for updated_at
CREATE TRIGGER update_checkpoint_reward_distributions_updated_at
    BEFORE UPDATE ON checkpoint_reward_distributions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Add comments for documentation
COMMENT ON TABLE checkpoint_reward_distributions IS 'Stores calculated reward for EACH JOB at each checkpoint. One record per job, NOT aggregated by worker. Only populated when checkpoint has non-zero fees.';
COMMENT ON COLUMN checkpoint_reward_distributions.checkpoint_id IS 'Checkpoint where this job was completed and reward distributed';
COMMENT ON COLUMN checkpoint_reward_distributions.worker_public_key IS 'Worker who completed this job and receives this reward';
COMMENT ON COLUMN checkpoint_reward_distributions.job_id IS 'Reference to the specific job in worker_job_events (one reward record per job)';
COMMENT ON COLUMN checkpoint_reward_distributions.reward_amount IS 'Calculated reward for THIS SINGLE JOB: total_fees_at_checkpoint / total_jobs_at_checkpoint';
COMMENT ON COLUMN checkpoint_reward_distributions.total_fees_at_checkpoint IS 'Total fees collected at this checkpoint (stored for reference and auditing)';
COMMENT ON COLUMN checkpoint_reward_distributions.total_jobs_at_checkpoint IS 'Total number of jobs at this checkpoint (stored for reference and auditing)';


-- Function to update worker_rewards aggregate table when checkpoint rewards are distributed
CREATE OR REPLACE FUNCTION update_worker_rewards_from_distribution()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert or update the worker_rewards aggregate
INSERT INTO worker_rewards (
    worker_public_key,
    total_jobs_completed,
    total_rewards,
    checkpoints_participated,
    first_reward_time,
    last_reward_time,
    max_checkpoint
)
SELECT
    NEW.worker_public_key,
    COUNT(DISTINCT job_id),
    SUM(reward_amount),
    COUNT(DISTINCT checkpoint_id),
    MIN(timestamp),
    MAX(timestamp),
    MAX(checkpoint_id)
FROM checkpoint_reward_distributions
WHERE worker_public_key = NEW.worker_public_key
    ON CONFLICT (worker_public_key) DO UPDATE SET
        total_jobs_completed = (
           SELECT COUNT(DISTINCT job_id)
           FROM checkpoint_reward_distributions
           WHERE worker_public_key = NEW.worker_public_key
       ),
       total_rewards = (
           SELECT SUM(reward_amount)
           FROM checkpoint_reward_distributions
           WHERE worker_public_key = NEW.worker_public_key
       ),
       checkpoints_participated = (
           SELECT COUNT(DISTINCT checkpoint_id)
           FROM checkpoint_reward_distributions
           WHERE worker_public_key = NEW.worker_public_key
       ),
       first_reward_time = LEAST(worker_rewards.first_reward_time, NEW.timestamp),
       last_reward_time = GREATEST(worker_rewards.last_reward_time, NEW.timestamp),
       max_checkpoint = GREATEST(worker_rewards.max_checkpoint, NEW.checkpoint_id),
       updated_at = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update worker_rewards when distributions are inserted
CREATE TRIGGER trigger_update_worker_rewards
    AFTER INSERT ON checkpoint_reward_distributions
    FOR EACH ROW
    EXECUTE FUNCTION update_worker_rewards_from_distribution();

COMMENT ON FUNCTION update_worker_rewards_from_distribution() IS
'Automatically updates worker_rewards aggregate table when new checkpoint reward distributions are inserted';