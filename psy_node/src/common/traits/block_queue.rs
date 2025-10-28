use plonky2::{hash::hash_types::RichField, plonk::proof::ProofWithPublicInputs};
use psy_core::{data::{base_types::hash256::Hash256, qhashout::QHashOut}, job::id::{QJobTopic, QProvingJobDataID}};
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_data::guta::header::GlobalUserTreeAggregatorHeader;

use crate::common::api_request_id::QEDAPIWriteRequestId;





pub trait CoordinatorBlockAPIInputQueueImm<F: RichField> {
    fn add_user_registration_request_imm(&self, request_id: QEDAPIWriteRequestId, fingerprint: QHashOut<F>, public_key_param: QHashOut<F>) -> anyhow::Result<()>;
    fn add_contract_deploy_request_imm(&self, request_id: QEDAPIWriteRequestId, public_key: QHashOut<F>) -> anyhow::Result<()>;
}


pub trait CoordinatorBlockAPINodeImmRead<F: RichField> {
    fn get_guta_sub_tree_merkle_proof(&self, guta_realm_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
}
pub trait CoordinatorBlockAPINodeImm<F: RichField> {
    fn report_guta_update(&self, request_id: QEDAPIWriteRequestId, proof_id: QProvingJobDataID, proof_to_sub_root: MerkleProofCore<QHashOut<F>>, header: GlobalUserTreeAggregatorHeader<F>) -> anyhow::Result<()>;

}