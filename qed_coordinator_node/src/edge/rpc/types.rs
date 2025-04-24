use serde::Deserialize;
use plonky2::{plonk::proof::ProofWithPublicInputs, plonk::config::PoseidonGoldilocksConfig};
use qed_store::config::store_config::QEDFelt;
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}