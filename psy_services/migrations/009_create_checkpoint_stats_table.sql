-- 009_create_checkpoint_stats_table.sql
-- Stores blockchain statistics per checkpoint with rollback safety
-- Only data that has been confirmed for 3+ blocks is inserted, should be guaranteed by the reporter

CREATE TABLE checkpoint_stats (
    checkpoint_id BIGINT PRIMARY KEY,
    fees_collected BIGINT NOT NULL,           -- Total transaction fees collected at this checkpoint (in minimal units)
    user_ops_processed BIGINT NOT NULL,       -- Number of user operations processed
    total_transactions BIGINT NOT NULL,       -- Total number of transactions
    slots_modified BIGINT NOT NULL,           -- Number of slots modified
    metadata JSONB DEFAULT '{}'::jsonb,       -- Flexible metadata for future extensions
    timestamp TIMESTAMPTZ NOT NULL,           -- Block timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT chk_fees_non_negative CHECK (fees_collected >= 0),
    CONSTRAINT chk_user_ops_non_negative CHECK (user_ops_processed >= 0),
    CONSTRAINT chk_total_transactions_non_negative CHECK (total_transactions >= 0),
    CONSTRAINT chk_slots_non_negative CHECK (slots_modified >= 0)
);

-- Create indexes for efficient querying
CREATE INDEX idx_checkpoint_stats_timestamp ON checkpoint_stats(timestamp DESC);
CREATE INDEX idx_checkpoint_stats_fees ON checkpoint_stats(fees_collected) WHERE fees_collected > 0;
CREATE INDEX idx_checkpoint_stats_checkpoint_timestamp ON checkpoint_stats(checkpoint_id, timestamp);

-- Create trigger for updated_at
CREATE TRIGGER update_checkpoint_stats_updated_at
    BEFORE UPDATE ON checkpoint_stats
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Add comments for documentation
COMMENT ON TABLE checkpoint_stats IS 'Stores blockchain statistics per checkpoint. Data is only inserted after 3+ block confirmations to avoid rollback issues.';
COMMENT ON COLUMN checkpoint_stats.checkpoint_id IS 'Unique checkpoint identifier from the blockchain';
COMMENT ON COLUMN checkpoint_stats.fees_collected IS 'Total transaction fees collected at this checkpoint in minimal units. If 0, no rewards are distributed.';
COMMENT ON COLUMN checkpoint_stats.user_ops_processed IS 'Number of user operations processed at this checkpoint';
COMMENT ON COLUMN checkpoint_stats.metadata IS 'Flexible JSON field for future extensions and additional blockchain data';