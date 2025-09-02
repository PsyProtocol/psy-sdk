-- 003_create_worker_events_table.sql

CREATE TABLE worker_event_statuses (
    id SERIAL PRIMARY KEY,
    status VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO worker_event_statuses (status) VALUES
    ('PENDING'),
    ('PROCESSING'),
    ('COMPLETED'),
    ('FAILED');

CREATE TABLE worker_event_sources (
    id SERIAL PRIMARY KEY,
    source VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO worker_event_sources (source) VALUES
    ('COORDINATOR'),
    ('REALM');

CREATE TABLE worker_events (
    id UUID DEFAULT uuid_generate_v4(),
    realm_id BIGINT,
    public_key VARCHAR(128),
    status VARCHAR(100) NOT NULL,
    source VARCHAR(255) NOT NULL,
    job_id JSONB NOT NULL,               -- QProvingJobDataID serialized as JSONB
    topic SMALLINT,                  -- Topic from job_id.topic
    circuit_type SMALLINT,           -- Circuit type from job_id.circuit_type
    checkpoint_id BIGINT NOT NULL,
    duration BIGINT,                     -- milliseconds, nullable for pending/processing/failed events
    metadata JSONB DEFAULT '{}'::jsonb,  -- Add metadata for flexibility
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Composite primary key including the partition key
    PRIMARY KEY (id, timestamp),

    -- Foreign key constraints to enum tables
    CONSTRAINT fk_worker_events_status
        FOREIGN KEY (status) REFERENCES worker_event_statuses(status),
    CONSTRAINT fk_worker_events_source
        FOREIGN KEY (source) REFERENCES worker_event_sources(source)
);

SELECT create_hypertable('worker_events', 'timestamp', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_worker_events_realm_id ON worker_events(realm_id, timestamp DESC);
CREATE INDEX idx_worker_events_status ON worker_events(status, timestamp DESC);
CREATE INDEX idx_worker_events_realm_status_time ON worker_events(realm_id, status, timestamp DESC);
CREATE INDEX idx_worker_events_source ON worker_events(source, timestamp DESC);
CREATE INDEX idx_worker_events_topic ON worker_events(topic, timestamp DESC);
CREATE INDEX idx_worker_events_circuit_type ON worker_events(circuit_type, timestamp DESC);
CREATE INDEX idx_worker_events_topic_circuit_type ON worker_events(topic, circuit_type, timestamp DESC);
-- Optimized index for worker rewards queries: WHERE public_key = ? AND topic = ? AND status = 'COMPLETED'
CREATE INDEX idx_worker_events_rewards_query ON worker_events(public_key, topic, status, timestamp DESC);

CREATE TRIGGER update_worker_events_updated_at
    BEFORE UPDATE ON worker_events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();