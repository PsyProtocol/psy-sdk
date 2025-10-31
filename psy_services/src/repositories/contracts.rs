// Create new file: repositories/contracts.rs

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::models::{Contract, ContractMetadataReport, ContractResponse, ContractSummary, QFunctionMetadata, UserContractMetadata};

pub struct ContractRepository;

impl ContractRepository {
    /// Create or update a contract from a watcher report
    pub async fn upsert_from_report(pool: &PgPool, report: &ContractMetadataReport) -> anyhow::Result<Contract> {
        let contract = sqlx::query_as::<_, Contract>(
            r#"
            INSERT INTO contracts (
                contract_id,
                contract_uuid,
                checkpoint_id,
                deployer,
                function_whitelist_root,
                metadata,
                timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (contract_id) DO UPDATE SET
                contract_uuid = EXCLUDED.contract_uuid,
                checkpoint_id = EXCLUDED.checkpoint_id,
                deployer = EXCLUDED.deployer,
                function_whitelist_root = EXCLUDED.function_whitelist_root,
                metadata = EXCLUDED.metadata,
                timestamp = EXCLUDED.timestamp,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(report.contract_id as i64)
        .bind(report.contract_uuid)
        .bind(report.checkpoint_id as i64)
        .bind(&report.deployer)
        .bind(&report.function_whitelist_root)
        .bind(&report.metadata)
        .bind(report.timestamp)
        .fetch_one(pool)
        .await?;

        info!(
            "Contract upserted successfully: contract_id={}, uuid={}, deployer={}, checkpoint={}",
            contract.contract_id, contract.contract_uuid, contract.deployer, contract.checkpoint_id
        );

        Ok(contract)
    }

    /// Get a contract by its ID with full details
    pub async fn get_by_id(pool: &PgPool, contract_id: i64) -> anyhow::Result<Option<ContractResponse>> {
        let contract = sqlx::query_as::<_, Contract>(
            r#"
            SELECT * FROM contracts
            WHERE contract_id = $1
            "#,
        )
        .bind(contract_id)
        .fetch_optional(pool)
        .await?;

        match contract {
            Some(c) => Ok(Some(Self::contract_to_response(c)?)),
            None => Ok(None),
        }
    }

    /// Get a contract by its UUID with full details
    pub async fn get_by_uuid(pool: &PgPool, contract_uuid: Uuid) -> anyhow::Result<Option<ContractResponse>> {
        let contract = sqlx::query_as::<_, Contract>(
            r#"
            SELECT * FROM contracts
            WHERE contract_uuid = $1
            "#,
        )
        .bind(contract_uuid)
        .fetch_optional(pool)
        .await?;

        match contract {
            Some(c) => Ok(Some(Self::contract_to_response(c)?)),
            None => Ok(None),
        }
    }

    /// List contracts with filtering and pagination
    pub async fn list(
        pool: &PgPool,
        limit: i64,
        offset: i64,
        deployer: Option<&str>,
        checkpoint_id: Option<i64>,
    ) -> anyhow::Result<Vec<ContractSummary>> {
        let mut query = String::from(
            r#"
            SELECT * FROM contracts
            WHERE 1=1
            "#,
        );

        let mut bind_idx = 1;
        let mut bindings: Vec<String> = Vec::new();

        if deployer.is_some() {
            query.push_str(&format!(" AND deployer = ${}", bind_idx));
            bind_idx += 1;
        }

        if checkpoint_id.is_some() {
            query.push_str(&format!(" AND checkpoint_id = ${}", bind_idx));
            bind_idx += 1;
        }

        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${} OFFSET ${}", bind_idx, bind_idx + 1));

        // Build the query based on filters
        let mut q = sqlx::query_as::<_, Contract>(&query);

        if let Some(d) = deployer {
            q = q.bind(d);
        }
        if let Some(cp) = checkpoint_id {
            q = q.bind(cp);
        }
        q = q.bind(limit).bind(offset);

        let contracts = q.fetch_all(pool).await?;

        // Convert to summary format
        let summaries: Result<Vec<ContractSummary>, anyhow::Error> = contracts.into_iter().map(|c| Self::contract_to_summary(c)).collect();

        summaries
    }

    /// Search contracts by function name
    pub async fn search_by_function_name(pool: &PgPool, function_name: &str, limit: i64, offset: i64) -> anyhow::Result<Vec<ContractResponse>> {
        // Query using JSONB operators to search within the functions array
        let contracts = sqlx::query_as::<_, Contract>(
            r#"
            SELECT * FROM contracts
            WHERE metadata -> 'functions' @> $1::jsonb
               OR metadata #>> '{functions}' ILIKE $2
            ORDER BY timestamp DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(json!([{"name": function_name}]))
        .bind(format!("%{}%", function_name))
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let mut responses = Vec::new();
        for contract in contracts {
            responses.push(Self::contract_to_response(contract)?);
        }

        Ok(responses)
    }

    /// Get contracts by deployer
    pub async fn get_by_deployer(pool: &PgPool, deployer: &str, limit: i64, offset: i64) -> anyhow::Result<Vec<ContractSummary>> {
        let contracts = sqlx::query_as::<_, Contract>(
            r#"
            SELECT * FROM contracts
            WHERE deployer = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(deployer)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let summaries: Result<Vec<ContractSummary>, anyhow::Error> = contracts.into_iter().map(|c| Self::contract_to_summary(c)).collect();

        summaries
    }

    /// Get contracts at a specific checkpoint
    pub async fn get_by_checkpoint(pool: &PgPool, checkpoint_id: i64) -> anyhow::Result<Vec<ContractSummary>> {
        let contracts = sqlx::query_as::<_, Contract>(
            r#"
            SELECT * FROM contracts
            WHERE checkpoint_id = $1
            ORDER BY contract_id
            "#,
        )
        .bind(checkpoint_id)
        .fetch_all(pool)
        .await?;

        let summaries: Result<Vec<ContractSummary>, anyhow::Error> = contracts.into_iter().map(|c| Self::contract_to_summary(c)).collect();

        summaries
    }

    /// Get total count of contracts with optional filters
    pub async fn count(pool: &PgPool, deployer: Option<&str>, checkpoint_id: Option<i64>) -> anyhow::Result<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM contracts WHERE 1=1");

        if deployer.is_some() {
            query.push_str(" AND deployer = $1");
        }
        if checkpoint_id.is_some() {
            let idx = if deployer.is_some() { 2 } else { 1 };
            query.push_str(&format!(" AND checkpoint_id = ${}", idx));
        }

        let mut q = sqlx::query_scalar::<_, i64>(&query);

        if let Some(d) = deployer {
            q = q.bind(d);
        }
        if let Some(cp) = checkpoint_id {
            q = q.bind(cp);
        }

        let count = q.fetch_one(pool).await?;
        Ok(count)
    }

    // ============================================================================
    // Helper functions for data transformation
    // ============================================================================

    /// Convert a Contract to ContractResponse with extracted metadata
    fn contract_to_response(contract: Contract) -> anyhow::Result<ContractResponse> {
        // Try to extract UserContractMetadata from the metadata field
        let user_metadata: Option<UserContractMetadata> = contract
            .metadata
            .as_object()
            .and_then(|_| serde_json::from_value(contract.metadata.clone()).ok());

        let (state_tree_height, function_count, functions) = if let Some(meta) = user_metadata {
            (Some(meta.state_tree_height), Some(meta.function_count), meta.functions)
        } else {
            // Fallback: try to extract just the functions array
            let functions = Self::extract_functions_from_metadata(&contract.metadata).unwrap_or_default();
            (None, None, functions)
        };

        Ok(ContractResponse {
            contract_id: contract.contract_id,
            contract_uuid: contract.contract_uuid,
            checkpoint_id: contract.checkpoint_id,
            deployer: contract.deployer,
            function_whitelist_root: contract.function_whitelist_root,
            state_tree_height,
            function_count,
            functions,
            metadata: contract.metadata,
            timestamp: contract.timestamp,
        })
    }

    /// Convert a Contract to ContractSummary for list operations
    fn contract_to_summary(contract: Contract) -> anyhow::Result<ContractSummary> {
        // Try to extract function count from metadata
        let function_count = contract
            .metadata
            .get("function_count")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .or_else(|| contract.metadata.get("functions").and_then(|f| f.as_array()).map(|arr| arr.len()));

        Ok(ContractSummary {
            contract_id: contract.contract_id,
            contract_uuid: contract.contract_uuid,
            deployer: contract.deployer,
            checkpoint_id: contract.checkpoint_id,
            function_count,
            timestamp: contract.timestamp,
        })
    }

    /// Extract functions array from metadata JSON
    fn extract_functions_from_metadata(metadata: &serde_json::Value) -> anyhow::Result<Vec<QFunctionMetadata>> {
        let functions = metadata
            .get("functions")
            .and_then(|f| f.as_array())
            .ok_or_else(|| anyhow::anyhow!("Functions array not found in metadata"))?;

        let mut result = Vec::new();
        for func in functions {
            if let Ok(function) = serde_json::from_value::<QFunctionMetadata>(func.clone()) {
                result.push(function);
            }
        }

        Ok(result)
    }

    /// Get all unique function names from all contracts
    pub async fn get_all_function_names(pool: &PgPool) -> anyhow::Result<Vec<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT jsonb_array_elements(metadata -> 'functions') ->> 'name' as name
            FROM contracts
            WHERE metadata -> 'functions' IS NOT NULL
            ORDER BY name
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.name).collect())
    }
}
