-- 004_create_user_events_table.sql
CREATE TABLE user_event_tx_types (
    id SERIAL PRIMARY KEY,
    tx_type VARCHAR(100) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO user_event_tx_types (tx_type) VALUES 
    ('REGISTER_USER'), 
    ('DEPLOY_CONTRACT'), 
    ('GUTA');

CREATE TABLE user_events (
    user_id VARCHAR(255) PRIMARY KEY,
    public_key VARCHAR(128) NOT NULL,
    tx_type VARCHAR(100) NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb,  -- Add metadata for flexibility
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Foreign key constraint to enum table
    CONSTRAINT fk_user_events_tx_type
        FOREIGN KEY (tx_type) REFERENCES user_event_tx_types(tx_type)
);

SELECT create_hypertable('user_events', 'timestamp', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX idx_user_events_tx_type ON user_events(tx_type, timestamp DESC);

CREATE TRIGGER update_user_events_updated_at
    BEFORE UPDATE ON user_events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
