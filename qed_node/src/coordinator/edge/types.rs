use serde::{Deserialize, Serialize};

use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;

use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;

use qed_data::config::store_config::QEDFelt;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestCheckpointResponse {
    pub checkpoint_id: u64,
}