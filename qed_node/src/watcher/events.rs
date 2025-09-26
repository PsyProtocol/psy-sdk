use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use qed_api_services::models::UserEventTxType;
use qed_core::job::id::{LayerId, ProvingJobCircuitType, QProvingJobDataID};
use qed_data::config::store_config::QEDFelt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatcherMessage {
    // User operations - immediate reporting
    UserRegistration(UserRegistrationEvent),
    DeployContract(UserDeployContractEvent),
    GutaSubmission(UserGutaSubmissionEvent),

    // Job status - immediate reporting
    JobStarted(JobStartedEvent),
    JobCompleted(JobCompletedEvent),
    JobTimeout(JobTimeoutEvent),

    // Backup operations - report with retry, then schedule deletion
    BackupProof(BackupProofEvent),
    BackupWitness(BackupWitnessEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationEvent {
    pub public_key: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationMetadata {
    pub registration_time: DateTime<Utc>,
    pub node_id: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeployContractEvent {
    pub deployer: String,
    pub metadata: UserDeployContractMetadata,  // Contains contract details without code
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeployContractMetadata {
    pub state_tree_height: u16,
    pub function_count: usize,
    pub function_whitelist_root: String,
    pub node_id: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGutaSubmissionEvent {
    pub realm_id: u64,
    pub metadata: UserGutaSubmissionMetadata,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGutaSubmissionMetadata {
    pub checkpoint_id: u64,
    pub circuit_type: ProvingJobCircuitType,
    pub top_line_proof: TopLineProofData,
    pub realm_proof_public_inputs: Vec<QEDFelt>,
    pub node_id: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLineProofData {
    pub old_root: String,
    pub new_root: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStartedEvent {
    pub job_id: QProvingJobDataID,
    pub worker_id: String,
    pub start_time: u64,
    pub layer_id: LayerId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCompletedEvent {
    pub job_id: QProvingJobDataID,
    pub worker_id: Option<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTimeoutEvent {
    pub job_id: QProvingJobDataID,
    pub worker_id: Option<String>,
    pub start_time: u64,
    pub timeout_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProofEvent {
    pub job_id: QProvingJobDataID,
    pub proof_data: Vec<u8>,
    pub timestamp: u64,
    pub delete_after_blocks: u64,  // Delete after N blocks (e.g., 256)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupWitnessEvent {
    pub job_id: QProvingJobDataID,
    pub witness_data: Vec<u8>,
    pub timestamp: u64,
    pub delete_after_blocks: u64,
}
