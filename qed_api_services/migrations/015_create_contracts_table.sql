-- 015_create_contracts_table.sql
-- Stores contract metadata reports from watcher telemetry

-- Create extension if not exists (may already be created by earlier migrations)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create contracts table
CREATE TABLE contracts (
    -- Primary key using contract_id as BIGINT to match the u64 from Rust
    contract_id BIGINT PRIMARY KEY,

    -- Contract UUID
    contract_uuid UUID NOT NULL,

    -- Checkpoint where this contract was reported
    checkpoint_id BIGINT NOT NULL,

    -- Contract deployer address
    deployer VARCHAR(255) NOT NULL,

    -- Function whitelist root hash
    function_whitelist_root VARCHAR(255) NOT NULL,

    -- JSONB field storing the complete UserContractMetadata and any future fields
    -- This provides maximum flexibility for protocol evolution
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Timestamp from the report
    timestamp TIMESTAMPTZ NOT NULL,

    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT uq_contract_uuid UNIQUE (contract_uuid)
);

-- Create indexes for efficient querying
CREATE INDEX idx_contracts_uuid ON contracts(contract_uuid);
CREATE INDEX idx_contracts_checkpoint ON contracts(checkpoint_id);
CREATE INDEX idx_contracts_deployer ON contracts(deployer);
CREATE INDEX idx_contracts_timestamp ON contracts(timestamp DESC);
CREATE INDEX idx_contracts_created_at ON contracts(created_at DESC);

-- Index on metadata for efficient JSONB queries (especially for function names)
CREATE INDEX idx_contracts_metadata_gin ON contracts USING GIN (metadata);

-- Specific index for querying functions array within metadata
CREATE INDEX idx_contracts_functions ON contracts USING GIN ((metadata -> 'functions'));

-- Create trigger for updated_at
CREATE TRIGGER update_contracts_updated_at
    BEFORE UPDATE ON contracts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Add comments for documentation
COMMENT ON TABLE contracts IS 'Stores contract metadata reports received from watcher telemetry';
COMMENT ON COLUMN contracts.contract_id IS 'Primary key - unique contract ID (u64 from Rust)';
COMMENT ON COLUMN contracts.contract_uuid IS 'Contract UUID';
COMMENT ON COLUMN contracts.checkpoint_id IS 'Checkpoint ID where this contract was reported';
COMMENT ON COLUMN contracts.deployer IS 'Address of the contract deployer';
COMMENT ON COLUMN contracts.function_whitelist_root IS 'Root hash of the function whitelist';
COMMENT ON COLUMN contracts.metadata IS 'JSONB containing complete UserContractMetadata for maximum flexibility';
COMMENT ON COLUMN contracts.timestamp IS 'Timestamp from the watcher report';