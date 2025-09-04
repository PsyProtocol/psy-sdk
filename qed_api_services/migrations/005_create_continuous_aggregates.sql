-- 005_create_continuous_aggregates.sql

CREATE OR REPLACE FUNCTION create_complete_worker_events_aggregate(
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
             realm_id,
             source,
             COUNT(*) as count,
             COUNT(CASE WHEN status = ''COMPLETED'' THEN 1 END) as completed_count,
             COUNT(CASE WHEN status = ''FAILED'' THEN 1 END) as failed_count,
             COUNT(CASE WHEN status = ''PROCESSING'' THEN 1 END) as processing_count,
             COUNT(CASE WHEN status = ''PENDING'' THEN 1 END) as pending_count,
             CAST(AVG(duration) as BIGINT) as avg_duration_ms,
             MIN(duration) as min_duration_ms,
             MAX(duration) as max_duration_ms
         FROM worker_events
         GROUP BY bucket, realm_id, source
         WITH NO DATA;',
        view_name,
        bucket_interval
    );

    -- Execute the query
    EXECUTE sql_query;
    RAISE NOTICE 'Created worker events aggregate view: %', view_name;

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

SELECT create_complete_worker_events_aggregate('worker_events_2m', '2 minutes', '10 minutes', '1 minute', '30 seconds', '2 month');
SELECT create_complete_worker_events_aggregate('worker_events_1h', '1 hour', '1 day', '15 minutes', '5 minutes', '1 year');
SELECT create_complete_worker_events_aggregate('worker_events_1d', '1 day', '7 days', '1 hour', '15 minutes', '1 year');
SELECT create_complete_worker_events_aggregate('worker_events_1w', '1 week', '1 month', '1 day', '1 hour', '1 year');
SELECT create_complete_worker_events_aggregate('worker_events_1m', '1 month', '1 year', '1 day', '1 hour', '1 year');
SELECT create_complete_worker_events_aggregate('worker_events_all_time', '20 years', '41 years', '1 hour', '1 hour');

CREATE OR REPLACE FUNCTION create_complete_user_events_aggregate(
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
             COUNT(*) as count,
             COUNT(CASE WHEN tx_type = ''REGISTER_USER'' THEN 1 END) as register_user_count,
             COUNT(CASE WHEN tx_type = ''DEPLOY_CONTRACT'' THEN 1 END) as deploy_contract_count,
             COUNT(CASE WHEN tx_type = ''GUTA'' THEN 1 END) as guta_count
         FROM user_events
         GROUP BY bucket
         WITH NO DATA;',
        view_name,
        bucket_interval
    );

    -- Execute the query
    EXECUTE sql_query;
    RAISE NOTICE 'Created user events aggregate view: %', view_name;

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

SELECT create_complete_user_events_aggregate('user_events_2m', '2 minutes', '10 minutes', '1 minute', '30 seconds', '1 week');
SELECT create_complete_user_events_aggregate('user_events_1h', '1 hour', '1 day', '15 minutes', '5 minutes', '1 year');
SELECT create_complete_user_events_aggregate('user_events_1d', '1 day', '7 days', '1 hour', '15 minutes', '1 year');
SELECT create_complete_user_events_aggregate('user_events_1w', '1 week', '1 month', '1 day', '1 hour', '1 year');
SELECT create_complete_user_events_aggregate('user_events_1m', '1 month', '1 year', '1 day', '1 hour', '1 year');
SELECT create_complete_user_events_aggregate('user_events_all_time', '20 years', '41 years', '1 hour', '1 hour');