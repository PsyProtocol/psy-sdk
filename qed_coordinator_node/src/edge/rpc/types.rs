use serde::Deserialize;
use plonky2::{plonk::proof::ProofWithPublicInputs, plonk::config::PoseidonGoldilocksConfig};
use serde::de::DeserializeOwned;
use qed_store::config::store_config::QEDFelt;
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}

#[derive(Debug, Deserialize)]
pub struct GetUserIdRequest {
    pub public_key_param: String, // hex string
}



#[derive(Debug, Deserialize)]
pub struct GetByIdRequest {
    pub id: u64,
}
#[derive(Debug, Deserialize)]
pub struct GetByFRequest
{
    pub id: QEDFelt,
}

#[derive(Debug, Deserialize)]
pub struct GetUserLeafRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct GetUserRegistrationLeafRequest {
    pub checkpoint_id: u64,
    pub leaf_index: u64,
}
#[derive(Debug, Deserialize)]
pub struct GetUserRegistrationFLeafRequest
{
    pub checkpoint_id: QEDFelt,
    pub leaf_index: QEDFelt,
}