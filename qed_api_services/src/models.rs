use chrono::{DateTime, Utc};
use qed_core::job::id::QProvingJobDataID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use serde_json::Value as JsonValue;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerEventStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerEventSource {
    Coordinator,
    Realm,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    sqlx::Type,
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum UserEventTxType {
    RegisterUser,
    DeployContract,
    Guta,
    UserEndcap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub id: Option<Uuid>,
    pub realm_id: Option<i64>,
    pub public_key: Option<String>,
    pub status: WorkerEventStatus,
    pub source: WorkerEventSource,
    pub job_id: QProvingJobDataID,
    pub checkpoint_id: i64,
    pub duration: Option<i64>, // milliseconds
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserEvent {
    pub user_id: String,
    pub public_key: String,
    pub tx_type: UserEventTxType,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserInfo {
    pub id: Option<Uuid>,
    pub public_key: String,
    pub twitter_handle: Option<String>,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerEventAggregation {
    pub bucket: DateTime<Utc>,
    pub realm_id: Option<i64>,
    pub source: WorkerEventSource,
    pub count: i64,
    pub completed_count: i64,
    pub failed_count: i64,
    pub processing_count: i64,
    pub pending_count: i64,
    pub avg_duration_ms: Option<i64>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserEventAggregation {
    pub bucket: DateTime<Utc>,
    pub count: i64,
    pub register_user_count: i64,
    pub deploy_contract_count: i64,
    pub guta_count: i64,
}

// JSON conversion helper functions for QProvingJobDataID
pub fn job_id_from_json(value: serde_json::Value) -> Result<QProvingJobDataID, serde_json::Error> {
    serde_json::from_value(value)
}

pub fn job_id_to_json(job_id: &QProvingJobDataID) -> serde_json::Value {
    serde_json::to_value(job_id).unwrap_or(serde_json::Value::Null)
}

// Realm statistics models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmStats {
    pub realm_id: i64,
    pub processing_tasks: i64,
    pub active_workers_1h: i64,
    pub active_workers_24h: i64,
    pub active_users_1h: i64,
    pub active_users_24h: i64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRealmStats {
    pub total_processing_tasks: i64,
    pub active_workers_1h: i64,
    pub active_workers_24h: i64,
    pub active_users_1h: i64,
    pub active_users_24h: i64,
    pub active_realms_1h: i64,
    pub active_realms_24h: i64,
    pub last_updated: DateTime<Utc>,
}

// Worker statistics models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub public_key: String,       // Worker's public key
    pub username: Option<String>, // Twitter handle from user_info table, or None if not found
    pub processing_tasks: std::collections::HashMap<String, i64>, // realm_id -> task count
    pub total_processing_tasks: i64,
    pub total_rewards: i64, // Reserved field, currently 0
    pub total_proofs: i64,  // Number of proofs completed in the last 24 hours
    pub completed_24h: i64,
    pub failed_24h: i64,
    pub total_rewards_24h: i64, // Total rewards earned in last 24 hours
    pub total_completed: i64,   // Total completed tasks of all time
    pub total_failed: i64,      // Total failed tasks of all time
    pub completed_1h: i64,      // Completed tasks in last 1 hour
    pub failed_1h: i64,         // Failed tasks in last 1 hour
    pub avg_proof_time: i64,    // Average proof generation time in milliseconds
    pub last_completed_block_height: Option<i64>, // Block height of last completed worker event
    pub last_updated: DateTime<Utc>,
}

// Worker rewards models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRewards {
    pub worker_public_key: String,
    pub checkpoint_id: i64,
    pub claimed_rewards: i64,   // in psy (5*10^9 per proof)
    pub unclaimed_rewards: i64, // in psy (5*10^9 per proof)
    pub total_rewards: i64,     // claimed + unclaimed
    pub claimed_proofs: i64,    // number of claimed proofs
    pub unclaimed_proofs: i64,  // number of unclaimed proofs
    pub total_proofs: i64,      // total proofs count
    pub total_rewards_24h: i64, // total rewards in last 24 hours (claimed + unclaimed)
    pub total_rewards_7d: i64,  // total rewards in last 7 days (claimed + unclaimed)
    pub total_rewards_30d: i64, // total rewards in last 30 days (claimed + unclaimed)
    pub last_updated: DateTime<Utc>,
}

// Worker event reward models
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerEventReward {
    pub id: uuid::Uuid,     // Same as worker_events.id (no Option, always present)
    pub public_key: String, // Which worker processed this
    pub checkpoint_id: i64, // Which checkpoint this reward belongs to
    pub reward_amount: i64, // Reward for this specific worker event
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// TPS models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpsData {
    pub tps: f64,
    pub transaction_count: i64,
    pub time_window_seconds: i64,
    pub block_height: i64,
    pub timestamp: DateTime<Utc>,
}

// Worker leaderboard models
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerLeaderboardEntry {
    pub worker_public_key: String,        // Worker's public key
    pub twitter_username: Option<String>, // Worker's Twitter username from user_info table
    pub proofs_24h: i64,                  // Number of proofs generated in last 24 hours
    pub rewards_24h: i64,                 // Total rewards earned in last 24 hours (in psy)
    pub rank: i64,                        // Ranking position (1-based)
}

// Worker rewards aggregation models
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerRewardsAggregation {
    pub bucket: DateTime<Utc>,     // Time bucket for aggregation
    pub public_key: String,        // Worker's public key
    pub completed_proofs: i64,     // Number of completed proofs in this bucket
    pub total_rewards: i64,        // Total rewards earned in this bucket (in psy)
    pub max_checkpoint: i64,       // Maximum checkpoint ID in this bucket
}

impl UserEvent {
    pub fn get_transaction_count(&self) -> i64 {
        match self.tx_type {
            UserEventTxType::RegisterUser | UserEventTxType::DeployContract => 1,
            UserEventTxType::UserEndcap => {
                if let Some(_metadata) = &self.metadata {
                    // For future use, we can add more fields to the metadata
                }
                1
            }
            UserEventTxType::Guta => {
                if let Some(_metadata) = &self.metadata {
                    // For future use, we can add more fields to the metadata
                }
                0
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobFilterCategory {
    All,            // All jobs
    RewardOnly,     // Only reward-eligible jobs
}

impl Default for JobFilterCategory {
    fn default() -> Self {
        JobFilterCategory::All
    }
}

impl JobFilterCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "reward_only" => JobFilterCategory::RewardOnly,
            "all" | _ => JobFilterCategory::All,
        }
    }
}

// Job status summary model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobStatusSummary {
    pub status: String,
    pub job_count: i64,
    pub percentage: Option<f64>,  // Using Option because it could be NULL
    pub last_update: Option<DateTime<Utc>>,
}

// Job status detailed model (from materialized view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestJobStatus {
    pub job_id: serde_json::Value,  // JSONB field
    pub realm_id: Option<i64>,
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub public_key: Option<String>,
    pub checkpoint_id: i64,
}

// Realm-specific job status summary
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RealmJobStatusSummary {
    pub realm_id: Option<i64>,
    pub status: String,
    pub job_count: i64,
    pub percentage: Option<f64>,
    pub last_update: Option<DateTime<Utc>>,
}

/// Checkpoint statistics from blockchain
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CheckpointStats {
    pub checkpoint_id: i64,
    pub fees_collected: i64,      // Total transaction fees collected (in minimal units)
    pub user_ops_processed: i64,  // Number of user operations processed
    pub total_transactions: i64,  // Total number of transactions
    pub slots_modified: i64,      // Number of slots modified
    pub metadata: JsonValue,      // Flexible metadata for future extensions
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating checkpoint stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLeafStat {
    pub checkpoint_id: i64,
    pub fees_collected: i64,
    pub user_ops_processed: i64,
    pub total_transactions: i64,
    pub slots_modified: i64,
    pub metadata: Option<JsonValue>,
    pub timestamp: DateTime<Utc>,
}

/// Worker job event (3+ blocks confirmed)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkerJobEvent {
    pub id: Uuid,
    pub worker_public_key: String,
    pub checkpoint_id: i64,
    pub job_id: JsonValue,           // QProvingJobDataID serialized as JSONB
    pub topic: Option<i16>,
    pub circuit_type: Option<i16>,
    pub duration: Option<i64>,       // milliseconds
    pub status: String,              // typically "COMPLETED"
    pub metadata: JsonValue,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating worker job events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkerJobEvent {
    pub worker_public_key: String,
    pub checkpoint_id: i64,
    pub job_id: JsonValue,
    pub topic: Option<i16>,
    pub circuit_type: Option<i16>,
    pub duration: Option<i64>,
    pub status: Option<String>,
    pub metadata: Option<JsonValue>,
    pub timestamp: DateTime<Utc>,
}

/// Checkpoint reward distribution
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CheckpointRewardDistribution {
    pub id: Uuid,
    pub checkpoint_id: i64,
    pub worker_public_key: String,
    pub job_id: Uuid,
    pub reward_amount: i64,
    pub total_fees_at_checkpoint: i64,
    pub total_jobs_at_checkpoint: i64,
    pub metadata: JsonValue,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating checkpoint reward distributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckpointRewardDistribution {
    pub checkpoint_id: i64,
    pub worker_public_key: String,
    pub job_id: Uuid,
    pub reward_amount: i64,
    pub total_fees_at_checkpoint: i64,
    pub total_jobs_at_checkpoint: i64,
    pub metadata: Option<JsonValue>,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated checkpoint rewards (from continuous aggregates)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CheckpointRewardAggregation {
    pub bucket: DateTime<Utc>,
    pub worker_public_key: String,
    pub checkpoints_participated: i64,
    pub jobs_completed: i64,
    pub total_rewards: i64,
    pub avg_reward_per_job: Option<f64>,
    pub max_checkpoint: i64,
    pub min_checkpoint: i64,
}

/// Summary statistics for a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRewardSummary {
    pub checkpoint_id: i64,
    pub fees_collected: i64,
    pub total_jobs: i64,
    pub total_workers: i64,
    pub reward_per_job: i64,
    pub timestamp: DateTime<Utc>,
}

/// Worker's reward statistics across time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCheckpointRewardStats {
    pub worker_public_key: String,
    pub total_rewards: i64,
    pub total_jobs_completed: i64,
    pub checkpoints_participated: i64,
    pub avg_reward_per_job: f64,
    pub last_checkpoint_id: i64,
    pub last_reward_timestamp: DateTime<Utc>,
}


#[derive(Debug, Deserialize)]
pub struct CheckpointStatsRequest {
    pub checkpoint_id: i64,
    pub fees_collected: i64,
    pub user_ops_processed: i64,
    pub total_transactions: i64,
    pub slots_modified: i64,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CheckpointStatsResponse {
    pub success: bool,
    pub checkpoint_id: i64,
    pub message: String,
}


#[derive(Debug, Deserialize)]
pub struct WorkerJobEventRequest {
    pub worker_public_key: String,
    pub checkpoint_id: i64,
    pub job_id: serde_json::Value,
    pub topic: Option<i16>,
    pub circuit_type: Option<i16>,
    pub duration: Option<i64>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}


#[derive(Debug, Serialize)]
pub struct WorkerJobEventsResponse {
    pub success: bool,
    pub events_reported: usize,
    pub checkpoint_id: i64,
    pub message: String,
}



#[derive(Debug, Deserialize)]
pub struct ProcessCheckpointRequest {
    pub checkpoint_stats: CheckpointStatsRequest,
    pub job_events: Vec<WorkerJobEventRequest>,
}

#[derive(Debug, Serialize)]
pub struct ProcessCheckpointResponse {
    pub checkpoint_id: i64,
    pub fees_collected: i64,
    pub total_jobs: i64,
    pub total_workers: i64,
    pub reward_per_job: i64,
    pub total_distributions_created: usize,  // Total reward records (one per job)
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckpointQuery {
    pub start_checkpoint: Option<i64>,
    pub end_checkpoint: Option<i64>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WorkerRewardQuery {
    pub time_period: Option<String>, // "2m", "1h", "1d", "1w", "1m"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WorkerRewardResponse {
    pub worker_public_key: String,
    pub time_period: String,
    pub aggregations: Vec<CheckpointRewardAggregation>,
    pub total_rewards: i64,
    pub total_jobs: i64,
    pub total_checkpoints: i64,
}

#[derive(Debug, Serialize)]
pub struct CheckpointProcessingStatus {
    pub pending_checkpoints: Vec<i64>,
    pub pending_count: usize,
    pub last_processed_checkpoint: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLeavesRequest {
    pub leaves: Vec<CheckpointLeafStat>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLeavesResponse {
    pub success: bool,
    pub processed_count: usize,
    pub message: String,
}


// ============================================================================
// Core contract data structures
// ============================================================================

// Function metadata that will be stored within the JSONB metadata field
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QFunctionMetadata {
    pub method_id: u32,
    pub name: String,
    pub num_inputs: u32,
    pub num_outputs: u32,
}

// The UserContractMetadata that gets stored in the metadata JSONB field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContractMetadata {
    pub contract_uuid: Uuid,
    pub state_tree_height: u16,
    pub function_count: usize,
    pub functions: Vec<QFunctionMetadata>,
    pub function_whitelist_root: String,
    pub contract_id: u64,
    // Any additional fields can be added here without schema changes
}

// ============================================================================
// Watcher report structure (what the API receives)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadataReport {
    pub contract_uuid: Uuid,
    pub checkpoint_id: u64,
    pub contract_id: u64,
    pub deployer: String,
    pub function_whitelist_root: String,
    pub metadata: JsonValue,  // JSONB field storing complete UserContractMetadata
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Database models
// ============================================================================

// Database model for the contracts table
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Contract {
    pub contract_id: i64,  // BIGINT in PostgreSQL
    pub contract_uuid: Uuid,
    pub checkpoint_id: i64,  // BIGINT in PostgreSQL
    pub deployer: String,
    pub function_whitelist_root: String,
    pub metadata: JsonValue,  // Complete UserContractMetadata as JSONB
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// API response models
// ============================================================================

// Response model for the frontend API - includes extracted function names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResponse {
    pub contract_id: i64,
    pub contract_uuid: Uuid,
    pub checkpoint_id: i64,
    pub deployer: String,
    pub function_whitelist_root: String,
    pub state_tree_height: Option<u16>,  // Extracted from metadata if available
    pub function_count: Option<usize>,   // Extracted from metadata if available
    pub functions: Vec<QFunctionMetadata>,  // Extracted from metadata for convenience
    pub metadata: JsonValue,  // Full metadata for extensibility
    pub timestamp: DateTime<Utc>,
}

// Simplified response for list operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSummary {
    pub contract_id: i64,
    pub contract_uuid: Uuid,
    pub deployer: String,
    pub checkpoint_id: i64,
    pub function_count: Option<usize>,
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Request/Response models for telemetry endpoint
// ============================================================================

// Request payload for the telemetry endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractTelemetryPayload {
    pub report: ContractMetadataReport,
}

// Response from the telemetry endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractTelemetryResponse {
    pub success: bool,
    pub contract_id: i64,
    pub message: String,
}

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListContractsParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub deployer: Option<String>,
    pub checkpoint_id: Option<i64>,
    pub function_name: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
pub struct ListContractsResponse {
    pub contracts: Vec<ContractSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}