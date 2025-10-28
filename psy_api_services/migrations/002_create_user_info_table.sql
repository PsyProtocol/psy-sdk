-- 002_create_user_info_table.sql
-- Create user_info table for storing user registration data

CREATE TABLE user_info (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    public_key VARCHAR(128) NOT NULL UNIQUE,
    twitter_handle VARCHAR(255),
    label VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast lookups by public_key (most common query)
CREATE INDEX idx_user_info_public_key ON user_info(public_key);

CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically update updated_at column
CREATE TRIGGER update_user_info_updated_at
    BEFORE UPDATE ON user_info
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();