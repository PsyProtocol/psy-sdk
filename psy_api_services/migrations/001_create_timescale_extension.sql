-- 001_create_timescale_extension.sql
-- Enable TimescaleDB extension

-- Enable timescaledb extension for time-series data
CREATE EXTENSION IF NOT EXISTS timescaledb;
-- Enable uuid extension for UUID support
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";