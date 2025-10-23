-- 010_create_worker_job_events_table.sql
-- Stores worker job completion events that have been confirmed for 3+ blocks
-- This is separate from worker_events to track only stable, reward-eligible jobs

CREATE TABLE worker_job_events (
    id UUID DEFAULT uuid_generate_v4(),
    worker_public_key VARCHAR(128) NOT NULL,  -- Worker who completed this job
    checkpoint_id BIGINT NOT NULL,            -- Checkpoint where the job was completed
    job_id JSONB NOT NULL,                    -- QProvingJobDataID serialized as JSONB
    topic SMALLINT,                           -- Topic from job_id.topic
    circuit_type SMALLINT,                    -- Circuit type from job_id.circuit_type
    duration BIGINT,                          -- Job duration in milliseconds
    status VARCHAR(100) NOT NULL DEFAULT 'COMPLETED', -- Job completion status
    metadata JSONB DEFAULT '{}'::jsonb,       -- Flexible metadata for future extensions
    timestamp TIMESTAMPTZ NOT NULL,           -- Job completion timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite primary key including the partition key
    PRIMARY KEY (id, timestamp),

    -- Foreign key to checkpoint_stats to ensure checkpoint exists
    CONSTRAINT fk_worker_job_events_checkpoint
       FOREIGN KEY (checkpoint_id) REFERENCES checkpoint_stats(checkpoint_id)
           ON DELETE CASCADE
);

-- Convert to hypertable for time-series data
SELECT create_hypertable('worker_job_events', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes for efficient querying
CREATE INDEX idx_worker_job_events_worker_checkpoint ON worker_job_events(worker_public_key, checkpoint_id, timestamp DESC);
CREATE INDEX idx_worker_job_events_checkpoint ON worker_job_events(checkpoint_id, timestamp DESC);
CREATE INDEX idx_worker_job_events_worker_timestamp ON worker_job_events(worker_public_key, timestamp DESC);
CREATE INDEX idx_worker_job_events_topic ON worker_job_events(topic, timestamp DESC);
CREATE INDEX idx_worker_job_events_circuit_type ON worker_job_events(circuit_type, timestamp DESC);

-- Create trigger for updated_at
CREATE TRIGGER update_worker_job_events_updated_at
    BEFORE UPDATE ON worker_job_events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Add comments for documentation
COMMENT ON TABLE worker_job_events IS 'Stores worker job completion events confirmed for 3+ blocks. Used for reward distribution calculation.';
COMMENT ON COLUMN worker_job_events.worker_public_key IS 'Public key of the worker who completed this job';
COMMENT ON COLUMN worker_job_events.checkpoint_id IS 'Checkpoint ID where this job was completed';
COMMENT ON COLUMN worker_job_events.job_id IS 'Detailed job information including topic and circuit type';
COMMENT ON COLUMN worker_job_events.status IS 'Job status - typically COMPLETED for reward-eligible jobs';
COMMENT ON COLUMN worker_job_events.metadata IS 'Flexible JSON field for future extensions and additional job data';