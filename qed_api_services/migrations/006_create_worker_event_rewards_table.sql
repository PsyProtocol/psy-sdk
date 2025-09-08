-- 006_create_worker_event_rewards_table.sql

CREATE TABLE worker_event_rewards (
    id UUID NOT NULL,                   -- Same as worker_events.id (no auto-generation)
    public_key VARCHAR(128) NOT NULL,   -- Which worker processed this
    checkpoint_id BIGINT NOT NULL,      -- Which checkpoint this reward belongs to
    reward_amount BIGINT NOT NULL,      -- Reward for this specific worker event
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Primary key including timestamp for TimescaleDB partitioning
    PRIMARY KEY (id, timestamp)
);

SELECT create_hypertable('worker_event_rewards', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes for efficient querying
CREATE INDEX idx_worker_event_rewards_public_key ON worker_event_rewards(public_key, timestamp DESC);
CREATE INDEX idx_worker_event_rewards_checkpoint ON worker_event_rewards(checkpoint_id, timestamp DESC);
CREATE INDEX idx_worker_event_rewards_public_key_checkpoint ON worker_event_rewards(public_key, checkpoint_id, timestamp DESC);

CREATE TRIGGER update_worker_event_rewards_updated_at
    BEFORE UPDATE ON worker_event_rewards
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();