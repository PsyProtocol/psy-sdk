use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use qed_core::job::id::{QProvingJobDataID, JobProof};
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::config::store_config::QEDFelt;
use qed_core::data::qhashout::QHashOut;
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::qdata::checkpoint::{QEDCheckpointLeaf, QEDL2BlockState, QEDCheckpointGlobalStateRoots};
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qdata::user::QEDUserLeaf;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_data::config::store_config::QCheckpointSyncInfoCompact;

// Import the request types from qed_prover
use qed_prover::local::request::{QRegisterUserRPCRequest, QDeployContractRPCRequest};

use super::types::LatestCheckpointResponse;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[rpc(server, client, namespace = "qed")]
pub trait CoordinatorEdgeRpc {
    // Basic methods
    #[method(name = "register_user")]
    async fn register_user(&self, public_key: ZKPublicKeyInfo<F>) -> RpcResult<String>;

    #[method(name = "get_user_id")]
    async fn get_user_id(&self, public_key: QHashOut<F>) -> RpcResult<u64>;

    #[method(name = "deploy_contract")]
    async fn deploy_contract(&self, deploy_contract: QBCDeployContract<F>) -> RpcResult<String>;

    #[method(name = "build_block")]
    async fn build_block(&self) -> RpcResult<String>;

    #[method(name = "submit_guta")]
    async fn submit_guta(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<String>;

    #[method(name = "get_latest_checkpoint")]
    async fn get_latest_checkpoint(&self) -> RpcResult<LatestCheckpointResponse>;

    #[method(name = "latest_checkpoint")]
    async fn latest_checkpoint(&self) -> RpcResult<u64>;

    #[method(name = "get_latest_checkpoint_id")]
    async fn get_latest_checkpoint_id(&self) -> RpcResult<u64>;

    // Checkpoint sync info
    #[method(name = "get_checkpoint_sync_info")]
    async fn get_checkpoint_sync_info(&self, checkpoint_id: u64) -> RpcResult<CheckpointSyncInfo<F>>;

    #[method(name = "get_checkpoint_sync_info_compact")]
    async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> RpcResult<QCheckpointSyncInfoCompact>;

    // Contract methods
    #[method(name = "get_contract_leaf_data")]
    async fn get_contract_leaf_data(&self, contract_id: u64) -> RpcResult<QEDContractLeaf<F>>;

    #[method(name = "get_contract_leaf_data_f")]
    async fn get_contract_leaf_data_f(&self, contract_id: F) -> RpcResult<QEDContractLeaf<F>>;

    #[method(name = "get_contract_code_definition")]
    async fn get_contract_code_definition(&self, contract_id: u64) -> RpcResult<ContractCodeDefinition>;

    #[method(name = "get_contract_code_definition_f")]
    async fn get_contract_code_definition_f(&self, contract_id: F) -> RpcResult<ContractCodeDefinition>;

    // Checkpoint methods
    #[method(name = "get_checkpoint_leaf_data")]
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointLeaf<F>>;

    #[method(name = "get_checkpoint_leaf_data_f")]
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> RpcResult<QEDCheckpointLeaf<F>>;

    #[method(name = "get_checkpoint_global_state_roots")]
    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointGlobalStateRoots<F>>;

    // L2 block state
    #[method(name = "get_latest_l2_block_state")]
    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_l2_block_state")]
    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_l2_block_state_f")]
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> RpcResult<QEDL2BlockState>;

    // User registration tree
    #[method(name = "get_user_registration_tree_root")]
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_root_f")]
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_leaf_hash")]
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_leaf_hash_f")]
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_merkle_proof")]
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_registration_tree_merkle_proof_f")]
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // User tree
    #[method(name = "get_user_tree_root")]
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_root_f")]
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_sub_tree_merkle_proof")]
    async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_top_tree_merkle_proof")]
    async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_top_tree_cap_root")]
    async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_latest_top_tree_cap_root")]
    async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QEDUserLeaf<F>>;

    #[method(name = "get_user_tree_merkle_proof")]
    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof_f")]
    async fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // Contract function tree
    #[method(name = "get_contract_function_tree_root")]
    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_root_f")]
    async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_leaf_hash")]
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_leaf_hash_f")]
    async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_merkle_proof")]
    async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_contract_function_tree_merkle_proof_f")]
    async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // Contract tree
    #[method(name = "get_contract_tree_root")]
    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_root_f")]
    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_leaf_hash")]
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_leaf_hash_f")]
    async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_merkle_proof")]
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_contract_tree_merkle_proof_f")]
    async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // Deposit tree
    #[method(name = "get_deposit_tree_root")]
    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_root_f")]
    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_leaf_hash")]
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_leaf_hash_f")]
    async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_merkle_proof")]
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_deposit_tree_merkle_proof_f")]
    async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // Withdrawal tree
    #[method(name = "get_withdrawal_tree_root")]
    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_root_f")]
    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_leaf_hash")]
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_leaf_hash_f")]
    async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_merkle_proof")]
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_withdrawal_tree_merkle_proof_f")]
    async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // Checkpoint tree
    #[method(name = "get_latest_checkpoint_tree_root")]
    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root")]
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root_f")]
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash")]
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash_f")]
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_merkle_proof")]
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_tree_merkle_proof_f")]
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "generate_batch_proofs")]
    async fn generate_batch_proofs(&self, checkpoint_id: u64, job_ids: Vec<QProvingJobDataID>) -> RpcResult<Vec<JobProof>>;
}
