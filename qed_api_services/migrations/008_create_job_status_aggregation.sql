-- 008_create_job_status_aggregation.sql

-- Create materialized view for latest job status
-- This view maintains the latest status for each unique job_id
CREATE MATERIALIZED VIEW latest_job_status AS
SELECT DISTINCT ON (job_id)
    id,
    job_id,
    realm_id,
    public_key,
    status,
    source,
    topic,
    circuit_type,
    checkpoint_id,
    duration,
    metadata,
    timestamp,
    created_at,
    updated_at
FROM worker_events
ORDER BY job_id, timestamp DESC, id DESC;

-- Create unique index on job_id to support CONCURRENTLY refresh
-- This is required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_latest_job_status_job_id ON latest_job_status (job_id);

-- Create indexes for efficient querying
CREATE INDEX idx_latest_job_status_status ON latest_job_status (status);
CREATE INDEX idx_latest_job_status_realm_id ON latest_job_status (realm_id);
CREATE INDEX idx_latest_job_status_timestamp ON latest_job_status (timestamp DESC);
CREATE INDEX idx_latest_job_status_status_realm ON latest_job_status (status, realm_id);

-- Create a function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_latest_job_status()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY latest_job_status;
END;
$$ LANGUAGE plpgsql;

-- Optional: Create a view for job status summary statistics
CREATE OR REPLACE VIEW job_status_summary AS
SELECT
    status,
    COUNT(*) AS job_count,
    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2) AS percentage,
    MAX(timestamp) AS last_update
FROM latest_job_status
GROUP BY status
ORDER BY
    CASE status
        WHEN 'PENDING' THEN 1
        WHEN 'PROCESSING' THEN 2
        WHEN 'COMPLETED' THEN 3
        WHEN 'FAILED' THEN 4
        ELSE 5
        END;

-- Optional: Create a view for realm-specific job status summary
CREATE OR REPLACE VIEW job_status_summary_by_realm AS
SELECT
    realm_id,
    status,
    COUNT(*) AS job_count,
    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (PARTITION BY realm_id), 0), 2) AS percentage,
    MAX(timestamp) AS last_update
FROM latest_job_status
GROUP BY realm_id, status
ORDER BY realm_id,
         CASE status
             WHEN 'PENDING' THEN 1
             WHEN 'PROCESSING' THEN 2
             WHEN 'COMPLETED' THEN 3
             WHEN 'FAILED' THEN 4
             ELSE 5
             END;

-- Optional: Create a function to get job status summary with time window
CREATE OR REPLACE FUNCTION get_job_status_summary(
    time_window INTERVAL DEFAULT NULL
)
RETURNS TABLE (
    status VARCHAR(100),
    job_count BIGINT,
    percentage NUMERIC,
    last_update TIMESTAMPTZ
) AS $$
BEGIN
    IF time_window IS NULL THEN
        -- Return all jobs
        RETURN QUERY
SELECT
    ljs.status::VARCHAR(100),
    COUNT(*)::BIGINT AS job_count,
    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::NUMERIC AS percentage,
    MAX(ljs.timestamp) AS last_update
FROM latest_job_status ljs
GROUP BY ljs.status
ORDER BY
    CASE ljs.status
        WHEN 'PENDING' THEN 1
        WHEN 'PROCESSING' THEN 2
        WHEN 'COMPLETED' THEN 3
        WHEN 'FAILED' THEN 4
        ELSE 5
        END;
ELSE
        -- Return jobs within time window
        RETURN QUERY
SELECT
    ljs.status::VARCHAR(100),
    COUNT(*)::BIGINT AS job_count,
    ROUND(100.0 * COUNT(*) / NULLIF(SUM(COUNT(*)) OVER (), 0), 2)::NUMERIC AS percentage,
    MAX(ljs.timestamp) AS last_update
FROM latest_job_status ljs
WHERE ljs.timestamp >= (NOW() - time_window)
GROUP BY ljs.status
ORDER BY
    CASE ljs.status
        WHEN 'PENDING' THEN 1
        WHEN 'PROCESSING' THEN 2
        WHEN 'COMPLETED' THEN 3
        WHEN 'FAILED' THEN 4
        ELSE 5
        END;
END IF;
END;
$$ LANGUAGE plpgsql;