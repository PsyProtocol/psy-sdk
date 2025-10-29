use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use plonky2::{field::types::PrimeField64, plonk::proof::ProofWithPublicInputs};
use psy_core::{
    data::qhashout::QHashOut,
    job::id::{QProvingJobDataID, VariableHeightRewardMerkleProof},
};
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_data::{
    guta::end_cap_input::SubmitUserEndCapNonProofInput,
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        user::PsyUserLeaf,
    },
};
use psy_prover::session::TxStatus;

use crate::realm::{C, D, F};

#[rpc(server, client, namespace = "psy")]
pub trait RealmEdgeRpc {
    /// Check if a user id belongs to this realm
    #[method(name = "check_user_id_in_realm")]
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool>;

    /// Submit user end cap proof
    #[method(name = "submit_user_end_cap")]
    async fn submit_user_end_cap(&self, user_ec_input: SubmitUserEndCapNonProofInput<F>, proof: ProofWithPublicInputs<F, C, D>) -> RpcResult<String>;

    #[method(name = "get_tx_status")]
    async fn get_tx_status(&self, user_id: u64, nonce: u64) -> RpcResult<TxStatus>;

    #[method(name = "get_checkpoint_leaf_data")]
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointLeaf<F>>;

    #[method(name = "get_checkpoint_leaf_data_f")]
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> RpcResult<PsyCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_latest_block_state")]
    async fn get_latest_block_state(&self) -> RpcResult<PsyBlockState>;

    #[method(name = "get_block_state")]
    async fn get_block_state(&self, checkpoint_id: u64) -> RpcResult<PsyBlockState>;

    #[method(name = "get_block_state_f")]
    async fn get_block_state_f(&self, checkpoint_id: F) -> RpcResult<PsyBlockState> {
        self.get_block_state(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_registration_tree_root")]
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_latest_checkpoint_tree_root")]
    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root")]
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root_f")]
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_checkpoint_tree_leaf_hash")]
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash_f")]
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_leaf_hash(checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_checkpoint_tree_merkle_proof")]
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_tree_merkle_proof_f")]
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_checkpoint_tree_merkle_proof(checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_checkpoint_global_state_roots")]
    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointGlobalStateRoots<F>>;

    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<PsyUserLeaf<F>>;

    #[method(name = "get_user_leaf_data_f")]
    async fn get_user_leaf_data_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<PsyUserLeaf<F>> {
        self.get_user_leaf_data(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_contract_state_tree_root")]
    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_root_f")]
    async fn get_user_contract_state_tree_root_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_contract_state_tree_root(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }

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
    ) -> RpcResult<QHashOut<F>> {
        self.get_user_contract_state_tree_leaf_hash(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32, height, leaf_id.to_canonical_u64()).await
    }

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
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_state_tree_merkle_proof(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32, height, leaf_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_contract_tree_root")]
    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_root_f")]
    async fn get_user_contract_tree_root_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_contract_tree_root(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_contract_tree_leaf_hash")]
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_leaf_hash_f")]
    async fn get_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_contract_tree_leaf_hash(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }

    #[method(name = "get_user_contract_tree_merkle_proof")]
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_contract_tree_merkle_proof_f")]
    async fn get_user_contract_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_tree_merkle_proof(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }

    #[method(name = "get_user_tree_root")]
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_root_f")]
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_tree_leaf_hash")]
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_leaf_hash_f")]
    async fn get_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_tree_leaf_hash(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_bottom_tree_merkle_proof")]
    async fn get_user_bottom_tree_merkle_proof(&self, root_level: u8, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_bottom_tree_merkle_proof_f")]
    async fn get_user_bottom_tree_merkle_proof_f(&self, root_level: u8, checkpoint_id: F, user_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_bottom_tree_merkle_proof(root_level, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

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
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_sub_tree_merkle_proof(checkpoint_id.to_canonical_u64(), root_level, leaf_level, leaf_index.to_canonical_u64()).await
    }

    #[method(name = "get_user_tree_merkle_proof")]
    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof_f")]
    async fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64())
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
