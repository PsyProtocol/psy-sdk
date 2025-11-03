-- 013_create_processing_status_table.sql
-- Tracks which checkpoints have been processed to avoid reprocessing

CREATE TABLE IF NOT EXISTS checkpoint_processing_status (
    checkpoint_id BIGINT PRIMARY KEY,
    status VARCHAR(50) NOT NULL CHECK (status IN ('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED', 'ROLLBACK_DETECTED')),
    events_processed INT DEFAULT 0,
    processed_at TIMESTAMPTZ,
    error_message TEXT,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
    );

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_checkpoint_status ON checkpoint_processing_status(status);
CREATE INDEX IF NOT EXISTS idx_checkpoint_processed ON checkpoint_processing_status(processed_at);
CREATE INDEX IF NOT EXISTS idx_checkpoint_updated ON checkpoint_processing_status(updated_at);

-- Add comment for documentation
COMMENT ON TABLE checkpoint_processing_status IS 'Tracks the processing status of checkpoints for worker event aggregation';
COMMENT ON COLUMN checkpoint_processing_status.checkpoint_id IS 'The checkpoint ID being processed';
COMMENT ON COLUMN checkpoint_processing_status.status IS 'Current processing status: PENDING, PROCESSING, COMPLETED, FAILED, or ROLLBACK_DETECTED';
COMMENT ON COLUMN checkpoint_processing_status.events_processed IS 'Number of events successfully processed for this checkpoint';
COMMENT ON COLUMN checkpoint_processing_status.processed_at IS 'Timestamp when processing was completed';
COMMENT ON COLUMN checkpoint_processing_status.error_message IS 'Error message if processing failed';

-- ============================================================================
-- Table: rollback_detections
-- Records when rollbacks are detected (multiple slot_ids at same checkpoint)
-- ============================================================================

CREATE TABLE IF NOT EXISTS rollback_detections (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    checkpoint_id BIGINT NOT NULL,
    conflicting_slots BIGINT[] NOT NULL,
    selected_slot BIGINT NOT NULL,
    discarded_slots BIGINT[] NOT NULL,
    affected_job_count INT NOT NULL,
    detection_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
    );

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_rollback_checkpoint ON rollback_detections(checkpoint_id);
CREATE INDEX IF NOT EXISTS idx_rollback_time ON rollback_detections(detection_time);
CREATE INDEX IF NOT EXISTS idx_rollback_selected_slot ON rollback_detections(selected_slot);

-- Add comment for documentation
COMMENT ON TABLE rollback_detections IS 'Audit log of detected blockchain rollbacks where multiple slot_ids exist for a checkpoint';
COMMENT ON COLUMN rollback_detections.checkpoint_id IS 'The checkpoint where rollback was detected';
COMMENT ON COLUMN rollback_detections.conflicting_slots IS 'Array of all slot IDs found at this checkpoint';
COMMENT ON COLUMN rollback_detections.selected_slot IS 'The slot ID that was selected (smallest ID)';
COMMENT ON COLUMN rollback_detections.discarded_slots IS 'Array of slot IDs that were discarded';
COMMENT ON COLUMN rollback_detections.affected_job_count IS 'Total number of jobs affected by this rollback';

-- ============================================================================
-- Table: archived_worker_events
-- Stores discarded worker events from rollbacks for audit trail
-- ============================================================================

CREATE TABLE IF NOT EXISTS archived_worker_events (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    original_id UUID NOT NULL UNIQUE,
    checkpoint_id BIGINT NOT NULL,
    slot_id BIGINT NOT NULL,
    worker_public_key VARCHAR(255) NOT NULL,
    job_id JSONB NOT NULL,
    status VARCHAR(50) NOT NULL,
    metadata JSONB,
    archived_at TIMESTAMPTZ NOT NULL,
    reason VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
    );

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_archived_checkpoint ON archived_worker_events(checkpoint_id);
CREATE INDEX IF NOT EXISTS idx_archived_slot ON archived_worker_events(slot_id);
CREATE INDEX IF NOT EXISTS idx_archived_original ON archived_worker_events(original_id);
CREATE INDEX IF NOT EXISTS idx_archived_at ON archived_worker_events(archived_at);
CREATE INDEX IF NOT EXISTS idx_archived_worker ON archived_worker_events(worker_public_key);

-- Add comment for documentation
COMMENT ON TABLE archived_worker_events IS 'Archive of worker events discarded due to rollbacks, kept for audit purposes';
COMMENT ON COLUMN archived_worker_events.original_id IS 'Original ID from worker_events table';
COMMENT ON COLUMN archived_worker_events.slot_id IS 'The slot ID of this discarded event';
COMMENT ON COLUMN archived_worker_events.reason IS 'Reason for archival (e.g., rollback_detected)';


CREATE OR REPLACE FUNCTION get_unprocessed_finalized_checkpoints(
    finalized_height BIGINT,
    max_limit INT DEFAULT 100
)
RETURNS TABLE (checkpoint_id BIGINT) AS $$
BEGIN
RETURN QUERY
SELECT DISTINCT we.checkpoint_id
FROM worker_events we
         LEFT JOIN worker_job_events wje ON we.checkpoint_id = wje.checkpoint_id
         LEFT JOIN checkpoint_processing_status cps ON we.checkpoint_id = cps.checkpoint_id
WHERE we.checkpoint_id <= finalized_height
  AND we.status = 'COMPLETED'
  AND wje.checkpoint_id IS NULL
  AND (cps.status IS NULL OR cps.status != 'COMPLETED')
ORDER BY we.checkpoint_id ASC
    LIMIT max_limit;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Function: Clean up old archived events
-- Removes archived events older than specified days
-- ============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_archived_events(retention_days INT DEFAULT 30)
RETURNS INT AS $$
DECLARE
deleted_count INT;
BEGIN
DELETE FROM archived_worker_events
WHERE archived_at < NOW() - INTERVAL '1 day' * retention_days;

GET DIAGNOSTICS deleted_count = ROW_COUNT;
RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- View: Processing statistics
-- Provides overview of processing status
-- ============================================================================

CREATE OR REPLACE VIEW checkpoint_processing_stats AS
SELECT
    COUNT(*) as total_checkpoints,
    COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) as completed,
    COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed,
    COUNT(CASE WHEN status = 'PROCESSING' THEN 1 END) as processing,
    COUNT(CASE WHEN status = 'PENDING' THEN 1 END) as pending,
    COUNT(CASE WHEN status = 'ROLLBACK_DETECTED' THEN 1 END) as rollbacks,
    SUM(events_processed) as total_events_processed,
    MAX(processed_at) as last_processed_at,
    MIN(CASE WHEN status != 'COMPLETED' THEN checkpoint_id END) as oldest_unprocessed
FROM checkpoint_processing_status;

-- ============================================================================
-- View: Recent rollback summary
-- Shows rollback activity for monitoring
-- ============================================================================

CREATE OR REPLACE VIEW rollback_summary AS
SELECT
    DATE_TRUNC('hour', detection_time) as hour,
    COUNT(*) as rollback_count,
    SUM(affected_job_count) as total_affected_jobs,
    ARRAY_AGG(DISTINCT checkpoint_id) as affected_checkpoints
FROM rollback_detections
WHERE detection_time > NOW() - INTERVAL '7 days'
GROUP BY DATE_TRUNC('hour', detection_time)
ORDER BY hour DESC;

-- ============================================================================
-- Trigger: Update timestamp on checkpoint_processing_status
-- ============================================================================

CREATE OR REPLACE FUNCTION update_checkpoint_processing_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS checkpoint_processing_updated_at ON checkpoint_processing_status;
CREATE TRIGGER checkpoint_processing_updated_at
    BEFORE UPDATE ON checkpoint_processing_status
    FOR EACH ROW
    EXECUTE FUNCTION update_checkpoint_processing_timestamp();


-- Migration: Add archived_worker_events table for rollback handling
-- This table stores worker events that were discarded due to blockchain rollbacks
-- when multiple slot_ids exist at the same checkpoint

-- ============================================================================
-- Table: archived_worker_events
-- ============================================================================

CREATE TABLE IF NOT EXISTS archived_worker_events (
    -- Primary key
                                                      id UUID DEFAULT gen_random_uuid() PRIMARY KEY,

    -- Reference to original event
    original_id UUID NOT NULL UNIQUE,  -- The ID from worker_events table

-- Event identification
    checkpoint_id BIGINT NOT NULL,
    slot_id BIGINT NOT NULL,  -- The slot_id that was discarded

-- Worker information
    worker_public_key VARCHAR(255) NOT NULL,

    -- Job details (stored as JSONB since job_id is complex)
    job_id JSONB NOT NULL,

    -- Event status at time of archival
    status VARCHAR(50) NOT NULL,

    -- Original metadata from the event
    metadata JSONB,

    -- Archival information
    archived_at TIMESTAMPTZ NOT NULL,
    reason VARCHAR(100) NOT NULL,  -- e.g., 'rollback_detected'

-- Automatic timestamp
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
    );

-- ============================================================================
-- Indexes for Performance
-- ============================================================================

-- Index on checkpoint_id for filtering by checkpoint
CREATE INDEX IF NOT EXISTS idx_archived_checkpoint
    ON archived_worker_events(checkpoint_id);

-- Index on slot_id for analyzing discarded slots
CREATE INDEX IF NOT EXISTS idx_archived_slot
    ON archived_worker_events(slot_id);

-- Index on original_id for lookups
CREATE INDEX IF NOT EXISTS idx_archived_original
    ON archived_worker_events(original_id);

-- Index on archived_at for cleanup operations
CREATE INDEX IF NOT EXISTS idx_archived_at
    ON archived_worker_events(archived_at);

-- Index on worker_public_key for worker-specific queries
CREATE INDEX IF NOT EXISTS idx_archived_worker
    ON archived_worker_events(worker_public_key);

-- Compound index for rollback analysis
CREATE INDEX IF NOT EXISTS idx_archived_checkpoint_slot
    ON archived_worker_events(checkpoint_id, slot_id);

-- Index on reason for filtering by archival reason
CREATE INDEX IF NOT EXISTS idx_archived_reason
    ON archived_worker_events(reason);

-- ============================================================================
-- Table Documentation
-- ============================================================================

COMMENT ON TABLE archived_worker_events IS
'Archive of worker events discarded due to blockchain rollbacks. When multiple slot_ids exist at the same checkpoint_id, the system selects the slot with the smallest ID and archives events from other slots here for audit purposes.';

COMMENT ON COLUMN archived_worker_events.id IS
'Unique identifier for the archived record';

COMMENT ON COLUMN archived_worker_events.original_id IS
'Original UUID from the worker_events table before archival';

COMMENT ON COLUMN archived_worker_events.checkpoint_id IS
'The checkpoint ID where the rollback was detected';

COMMENT ON COLUMN archived_worker_events.slot_id IS
'The slot ID of this discarded event (not selected during rollback resolution)';

COMMENT ON COLUMN archived_worker_events.worker_public_key IS
'Public key of the worker who processed this job';

COMMENT ON COLUMN archived_worker_events.job_id IS
'The job identifier (stored as JSONB for complex structure)';

COMMENT ON COLUMN archived_worker_events.status IS
'Status of the event at the time of archival (e.g., COMPLETED, FAILED)';

COMMENT ON COLUMN archived_worker_events.metadata IS
'Original metadata from the worker event';

COMMENT ON COLUMN archived_worker_events.archived_at IS
'Timestamp when this event was archived';

COMMENT ON COLUMN archived_worker_events.reason IS
'Reason for archival (e.g., rollback_detected, manual_cleanup)';

-- ============================================================================
-- Helper Functions
-- ============================================================================

-- Function to get archived events for a specific checkpoint and slot
CREATE OR REPLACE FUNCTION get_archived_events_by_checkpoint_slot(
    p_checkpoint_id BIGINT,
    p_slot_id BIGINT DEFAULT NULL
)
RETURNS TABLE (
    id UUID,
    original_id UUID,
    worker_public_key VARCHAR,
    job_id JSONB,
    status VARCHAR,
    archived_at TIMESTAMPTZ
) AS $$
BEGIN
RETURN QUERY
SELECT
    ae.id,
    ae.original_id,
    ae.worker_public_key,
    ae.job_id,
    ae.status,
    ae.archived_at
FROM archived_worker_events ae
WHERE ae.checkpoint_id = p_checkpoint_id
  AND (p_slot_id IS NULL OR ae.slot_id = p_slot_id)
ORDER BY ae.archived_at DESC;
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old archived events
CREATE OR REPLACE FUNCTION cleanup_archived_events(
    retention_days INT DEFAULT 30
)
RETURNS TABLE (
    deleted_count BIGINT,
    oldest_kept_date TIMESTAMPTZ
) AS $$
DECLARE
v_deleted_count BIGINT;
    v_cutoff_date TIMESTAMPTZ;
BEGIN
    v_cutoff_date := NOW() - (retention_days || ' days')::INTERVAL;

DELETE FROM archived_worker_events
WHERE archived_at < v_cutoff_date;

GET DIAGNOSTICS v_deleted_count = ROW_COUNT;

RETURN QUERY
SELECT
    v_deleted_count,
    MIN(ae.archived_at) as oldest_kept_date
FROM archived_worker_events ae;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Views for Monitoring
-- ============================================================================

-- View showing rollback statistics
CREATE OR REPLACE VIEW archived_events_stats AS
SELECT
    checkpoint_id,
    slot_id,
    COUNT(*) as event_count,
    COUNT(DISTINCT worker_public_key) as affected_workers,
    MIN(archived_at) as first_archived,
    MAX(archived_at) as last_archived,
    reason
FROM archived_worker_events
GROUP BY checkpoint_id, slot_id, reason
ORDER BY checkpoint_id DESC, slot_id;

-- View showing recent archival activity
CREATE OR REPLACE VIEW recent_archival_activity AS
SELECT
    DATE_TRUNC('hour', archived_at) as hour,
    reason,
    COUNT(*) as events_archived,
    COUNT(DISTINCT checkpoint_id) as checkpoints_affected,
    COUNT(DISTINCT slot_id) as slots_affected,
    COUNT(DISTINCT worker_public_key) as workers_affected
FROM archived_worker_events
WHERE archived_at > NOW() - INTERVAL '24 hours'
GROUP BY DATE_TRUNC('hour', archived_at), reason
ORDER BY hour DESC;
