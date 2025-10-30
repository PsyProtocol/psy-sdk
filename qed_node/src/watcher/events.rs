use chrono::{DateTime, Utc};
use qed_data::qdata::contract_uuid::ContractUUID;
use qed_data::qdata::user::QEDUserLeaf;
use serde::{Deserialize, Serialize};
use qed_api_services::models::UserEventTxType;
use qed_core::job::id::{LayerId, ProvingJobCircuitType, QProvingJobDataID};
use qed_data::config::store_config::QEDFelt;
use qed_data::qblock::cmds::deploy_contract::QFunctionMetadata;
use qed_data::qdata::ups_end_cap_result::UPSEndCapResultCompact;
use crate::watcher::timeout_watcher::WatcherSourceNodeType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatcherMessage {
    // User operations - immediate reporting
    UserRegistration(UserRegistrationEvent),
    DeployContract(UserDeployContractEvent),
    GutaSubmission(UserGutaSubmissionEvent),
    EndcapSubmission(UserEndcapSubmissionEvent),

    // Job status - immediate reporting
    JobPending(JobPendingEvent),
    JobStarted(JobStartedEvent),
    JobCompleted(JobCompletedEvent),
    JobTimeout(JobTimeoutEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationEvent {
    pub timestamp: DateTime<Utc>,
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
    pub metadata: UserRegistrationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationMetadata {
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeployContractEvent {
    pub deployer: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: UserContractMetadata,  // Contains contract details without code
    pub node_id: String,
    pub node_type: WatcherSourceNodeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContractMetadata {
    pub contract_uuid: ContractUUID,
    pub state_tree_height: u16,
    pub function_count: usize,
    pub functions: Vec<QFunctionMetadata>,
    pub function_whitelist_root: String,
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
pub struct JobPendingEvent {
    pub job_id: QProvingJobDataID,
    pub start_time: u64,
    pub layer_id: LayerId,
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
pub struct UserEndcapSubmissionEvent {
    pub realm_id: u64,
    pub user_id: u64,
    pub metadata: UserEndcapSubmissionMetadata,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEndcapSubmissionMetadata {
    pub checkpoint_id: u64,
    pub state_transition: UPSEndCapResultCompact<QEDFelt>,
    pub new_user_leaf: QEDUserLeaf<QEDFelt>,
    pub endcap_proof_public_inputs: Vec<QEDFelt>,
    pub node_id: String,
    pub node_type: String,
}
