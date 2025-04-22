use crate::edge::rpc::{C, D};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::base_types::hash160::Hash160;
use qed_core::data::base_types::hash256::Hash256;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QTokenTransferRPCRequest {
    pub user_id: u64,
    pub to: u64,
    pub value: u64,
    pub nonce: u64,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QClaimDepositRPCRequest {
    pub user_id: u64,
    pub deposit_id: u64,
    pub value: u64,

    pub txid: Hash256,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub public_key: [u8; 33],

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAddWithdrawalRPCRequest {
    pub user_id: u64,
    pub value: u64,
    pub nonce: u64,

    pub destination_type: u8,
    pub destination: Hash160,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QSubmitEndCapRPCRequest<F: RichField> {
    pub user_ec_input: SubmitUserEndCapNonProofInput<F>,
    pub proof: ProofWithPublicInputs<GoldilocksField, C, D>,
}
