-- 012_create_checkpoint_reward_aggregates.sql
-- Create continuous aggregates for checkpoint reward distributions at different time intervals
-- These views enable efficient queries for frontend dashboards

CREATE OR REPLACE FUNCTION create_checkpoint_reward_aggregate(
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
    -- Note: Removed "WITH NO DATA" and removed the separate ALTER statement
    sql_query := format(
        'CREATE MATERIALIZED VIEW %I WITH (timescaledb.continuous) AS
         SELECT
             time_bucket(%L, timestamp) AS bucket,
             worker_public_key,
             COUNT(DISTINCT checkpoint_id) as checkpoints_participated,
             COUNT(DISTINCT job_id) as jobs_completed,
             SUM(reward_amount) as total_rewards,
             AVG(reward_amount) as avg_reward_per_job,
             MAX(checkpoint_id) as max_checkpoint,
             MIN(checkpoint_id) as min_checkpoint
         FROM checkpoint_reward_distributions
         GROUP BY bucket, worker_public_key
         WITH NO DATA;',
        view_name,
        bucket_interval
    );

    -- Execute the query
    EXECUTE sql_query;
    RAISE NOTICE 'Created checkpoint reward aggregate view: %', view_name;

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

-- Create continuous aggregates for different time intervals

-- 2-minute buckets (for real-time monitoring) - keep data for 7 days
SELECT create_checkpoint_reward_aggregate(
       'checkpoint_rewards_2m',
       '2 minutes',
       '1 day',
       '1 minute',
       '2 minutes',
       '7 days'
);

-- 1-hour buckets (for hourly analysis) - keep data for 30 days
SELECT create_checkpoint_reward_aggregate(
       'checkpoint_rewards_1h',
       '1 hour',
       '7 days',
       '1 hour',
       '30 minutes',
       '30 days'
);

-- 24-hour (1 day) buckets (for daily analysis) - keep data for 1 year
SELECT create_checkpoint_reward_aggregate(
       'checkpoint_rewards_1d',
       '1 day',
       '30 days',
       '1 day',
       '1 hour',
       '1 year'
);

-- 7-day (1 week) buckets (for weekly trends) - keep data for 1 year
SELECT create_checkpoint_reward_aggregate(
       'checkpoint_rewards_1w',
       '1 week',
       '3 months',
       '1 week',
       '1 day',
       '1 year'
);

-- 30-day (1 month) buckets (for monthly trends) - keep data for 2 years
SELECT create_checkpoint_reward_aggregate(
       'checkpoint_rewards_1m',
       '1 month',
       '1 year',
       '1 month',
       '1 day',
       '2 years'
);