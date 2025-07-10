use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use serde::{Deserialize, Serialize};

use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;

use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;

use qed_store::config::store_config::QEDFelt;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}

#[derive(Deserialize)]
pub struct RegisterUserParams {
    pub public_key: ZKPublicKeyInfo<QEDFelt>,
    pub secp256k1_public_key_hash: QHashOut<QEDFelt>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestCheckpointResponse {
    pub(crate) checkpoint_id: u64,
}
