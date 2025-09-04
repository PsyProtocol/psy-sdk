-- 007_create_worker_rewards_aggregates.sql

CREATE OR REPLACE FUNCTION create_worker_rewards_aggregate(
    view_name TEXT,
    bucket_interval INTERVAL,
    refresh_start_offset INTERVAL,
    refresh_end_offset INTERVAL,
    refresh_schedule INTERVAL,
    retention_period INTERVAL DEFAULT NULL
) RETURNS VOID AS $$
DECLARE
    sql_query TEXT;
BEGIN
    -- Build the SQL query for creating the materialized view
    sql_query := format(
        'CREATE MATERIALIZED VIEW %I WITH (timescaledb.continuous) AS
         SELECT
             time_bucket(%L, timestamp) AS bucket,
             public_key,
             COUNT(*) as completed_proofs,
             SUM(reward_amount) as total_rewards,
             MAX(checkpoint_id) as max_checkpoint
         FROM worker_event_rewards
         GROUP BY bucket, public_key
         WITH NO DATA;',
        view_name,
        bucket_interval
    );

    -- Execute the query
    EXECUTE sql_query;
    RAISE NOTICE 'Created worker rewards aggregate view: %', view_name;

    sql_query := format(
        'ALTER MATERIALIZED VIEW %I SET (timescaledb.materialized_only = false);',
        view_name
    );
    EXECUTE sql_query;
    RAISE NOTICE 'Set materialized_only to false for view: %', view_name;

    -- Add refresh policy
    PERFORM add_continuous_aggregate_policy(
        view_name,
        start_offset => refresh_start_offset,
        end_offset => refresh_end_offset,
        schedule_interval => refresh_schedule
    );
    RAISE NOTICE 'Added refresh policy for view: %', view_name;

    -- Add retention policy if specified
    IF retention_period IS NOT NULL THEN
        PERFORM add_retention_policy(view_name, retention_period);
        RAISE NOTICE 'Added retention policy for view: % (retention: %)', view_name, retention_period;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Create continuous aggregates for worker rewards at different time intervals
SELECT create_worker_rewards_aggregate('worker_rewards_1d', '1 day', '7 days', '1 hour', '15 minutes', '1 year');
SELECT create_worker_rewards_aggregate('worker_rewards_1w', '1 week', '1 month', '1 day', '1 hour', '1 year');
SELECT create_worker_rewards_aggregate('worker_rewards_1m', '1 month', '1 year', '1 day', '1 hour', '1 year');