use crate::realm::{C, D, F};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use plonky2::field::types::PrimeField64;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::id::{QProvingJobDataID, VariableHeightRewardMerkleProof};
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use psy_data::qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf};
use psy_data::qdata::{checkpoint::QEDL2BlockState, user::QEDUserLeaf};
use qed_prover::session::TxStatus;

#[rpc(server, client, namespace = "qed")]
pub trait RealmEdgeRpc {
    /// Check if a user id belongs to this realm
    #[method(name = "check_user_id_in_realm")]
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool>;

    /// Submit user end cap proof
    #[method(name = "submit_user_end_cap")]
    async fn submit_user_end_cap(
        &self,
        user_ec_input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<String>;

    #[method(name = "get_tx_status")]
    async fn get_tx_status(
        &self,
        user_id: u64,
        nonce: u64,
    ) -> RpcResult<TxStatus>;

    #[method(name = "get_checkpoint_leaf_data")]
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64)
        -> RpcResult<QEDCheckpointLeaf<F>>;

    #[method(name = "get_checkpoint_leaf_data_f")]
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F)
        -> RpcResult<QEDCheckpointLeaf<F>>;

    #[method(name = "get_latest_l2_block_state")]
    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_l2_block_state")]
    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_l2_block_state_f")]
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_user_registration_tree_root")]
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_latest_checkpoint_tree_root")]
    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root")]
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root_f")]
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash")]
    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash_f")]
    async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_merkle_proof")]
    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_tree_merkle_proof_f")]
    async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_global_state_roots")]
    async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QEDCheckpointGlobalStateRoots<F>>;

    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QEDUserLeaf<F>>;

    #[method(name = "get_user_leaf_data_f")]
    async fn get_user_leaf_data_f(&self, checkpoint_id: F, user_id: F)
        -> RpcResult<QEDUserLeaf<F>>;

    #[method(name = "get_user_contract_state_tree_root")]
    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_root_f")]
    async fn get_user_contract_state_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_leaf_hash")]
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_leaf_hash_f")]
    async fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_merkle_proof")]
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_contract_state_tree_merkle_proof_f")]
    async fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_contract_tree_root")]
    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_root_f")]
    async fn get_user_contract_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_leaf_hash")]
    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_leaf_hash_f")]
    async fn get_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_merkle_proof")]
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_contract_tree_merkle_proof_f")]
    async fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_root")]
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_root_f")]
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_leaf_hash")]
    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_leaf_hash_f")]
    async fn get_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_bottom_tree_merkle_proof")]
    async fn get_user_bottom_tree_merkle_proof(
        &self,
        root_level: u8,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_bottom_tree_merkle_proof_f")]
    async fn get_user_bottom_tree_merkle_proof_f(
        &self,
        root_level: u8,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_sub_tree_merkle_proof")]
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_sub_tree_merkle_proof_f")]
    async fn get_user_sub_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        root_level: u8,
        leaf_level: u8,
        leaf_index: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof")]
    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof_f")]
    async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        )
        .await
    }

    #[method(name = "generate_batch_variable_height_reward_proofs")]
    async fn generate_batch_variable_height_reward_proofs(
        &self,
        checkpoint_id: u64,
        job_ids: Vec<QProvingJobDataID>,
    ) -> RpcResult<Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)>>;

    #[method(name = "get_graphviz")]
    async fn get_graphviz(&self, checkpoint_id: u64) -> RpcResult<String>;
}
