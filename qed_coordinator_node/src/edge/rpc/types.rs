use serde::Deserialize;
use plonky2::{plonk::proof::ProofWithPublicInputs, plonk::config::PoseidonGoldilocksConfig};
use qed_store::config::store_config::QEDFelt;
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;
use serde::{Serialize};
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}

/// push the latest checkpoint sync info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSyncInfo {
    pub latest_checkpoint_id: u64,     // checkpoint id
    pub description: Option<String>,
    pub source_coordinator_edge_id: Option<String>,
    pub sync_timestamp: u64, //
    pub compact: QEDCheckpointSyncInfoCompact<QEDFelt>,
}


#[derive(Debug, Clone, Serialize)]
pub struct LatestCheckpointResponse {
    pub(crate) checkpoint_id: u64,
}
