use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use plonky2::{
    field::types::PrimeField64,
    plonk::{
        config::{GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::{
    data::qhashout::QHashOut,
    job::id::{QProvingJobDataID, VariableHeightRewardMerkleProof},
};
use psy_crypto::{hash::merkle::core::MerkleProofCore, signature::zk::data::ZKPublicKeyInfo};
use psy_data::{
    config::store_config::{PsyFelt, QCheckpointSyncInfoCompact},
    guta::api::SubmitGUTARealmResultAPINoProofInput,
    qblock::cmds::deploy_contract::QBCDeployContract,
    qdata::{
        checkpoint::{CheckpointSyncInfo, PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        contract::{ContractCodeDefinition, PsyContractLeaf},
        contract_metadata::ContractMetaData,
        user::PsyUserLeaf,
    },
};
use psy_provider::request::{QDeployContractRPCRequest, QRegisterUserRPCRequest};

use super::types::LatestCheckpointResponse;

type F = PsyFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[rpc(server, client, namespace = "psy")]
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
        realm_id: u64,
    ) -> RpcResult<String>;

    #[method(name = "submit_guta_v1")]
    async fn submit_guta_v1(&self, input: SubmitGUTARealmResultAPINoProofInput<F>, proof: Vec<u8>, realm_id: u64) -> RpcResult<()>;

    #[method(name = "has_pending_guta")]
    async fn has_pending_guta(&self, realm_id: u32) -> RpcResult<bool>;


    #[method(name = "get_latest_checkpoint")]
    async fn get_latest_checkpoint(&self) -> RpcResult<LatestCheckpointResponse>;

    #[method(name = "latest_checkpoint")]
    async fn latest_checkpoint(&self) -> RpcResult<u64>;

    #[method(name = "get_latest_checkpoint_id")]
    async fn get_latest_checkpoint_id(&self) -> RpcResult<u64>;

    // Checkpoint sync info
    #[method(name = "get_checkpoint_sync_info")]
    async fn get_checkpoint_sync_info(&self, realm_id: u32, checkpoint_id: u64) -> RpcResult<CheckpointSyncInfo<F>>;

    #[method(name = "get_checkpoint_sync_info_compact")]
    async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> RpcResult<QCheckpointSyncInfoCompact>;

    #[method(name = "get_latest_checkpoint_sync_info")]
    async fn get_latest_checkpoint_sync_info(&self, realm_id: u32) -> RpcResult<CheckpointSyncInfo<F>>;

    // Contract methods
    #[method(name = "get_contract_leaf_data")]
    async fn get_contract_leaf_data(&self, contract_id: u64) -> RpcResult<PsyContractLeaf<F>>;

    #[method(name = "get_contract_leaf_data_f")]
    async fn get_contract_leaf_data_f(&self, contract_id: F) -> RpcResult<PsyContractLeaf<F>> {
        self.get_contract_leaf_data(contract_id.to_canonical_u64()).await
    }

    #[method(name = "get_contract_code_definition")]
    async fn get_contract_code_definition(&self, contract_id: u64) -> RpcResult<ContractCodeDefinition>;

    #[method(name = "get_contract_code_definition_f")]
    async fn get_contract_code_definition_f(&self, contract_id: F) -> RpcResult<ContractCodeDefinition> {
        self.get_contract_code_definition(contract_id.to_canonical_u64()).await
    }

    // Checkpoint methods
    #[method(name = "get_checkpoint_leaf_data")]
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointLeaf<F>>;

    #[method(name = "get_checkpoint_leaf_data_f")]
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> RpcResult<PsyCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_checkpoint_global_state_roots")]
    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointGlobalStateRoots<F>>;

    // block state
    #[method(name = "get_latest_block_state")]
    async fn get_latest_block_state(&self) -> RpcResult<PsyBlockState>;

    #[method(name = "get_block_state")]
    async fn get_block_state(&self, checkpoint_id: u64) -> RpcResult<PsyBlockState>;

    #[method(name = "get_block_state_f")]
    async fn get_block_state_f(&self, checkpoint_id: F) -> RpcResult<PsyBlockState> {
        self.get_block_state(checkpoint_id.to_canonical_u64()).await
    }

    // User registration tree
    #[method(name = "get_user_registration_tree_root")]
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_root_f")]
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_registration_tree_leaf_hash")]
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_registration_tree_leaf_hash_f")]
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_leaf_hash(checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64())
            .await
    }

    #[method(name = "get_user_registration_tree_merkle_proof")]
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_registration_tree_merkle_proof_f")]
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_registration_tree_merkle_proof(checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64())
            .await
    }

    #[method(name = "get_user_registration_proof")]
    async fn get_user_registration_proof(&self, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    // User tree
    #[method(name = "get_user_tree_root")]
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_root_f")]
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_user_sub_tree_merkle_proof")]
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_top_tree_merkle_proof")]
    async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_top_tree_cap_root")]
    async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_latest_top_tree_cap_root")]
    async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<PsyUserLeaf<F>>;

    #[method(name = "get_user_tree_merkle_proof")]
    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof_f")]
    async fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64())
            .await
    }

    // Contract function tree
    #[method(name = "get_contract_function_tree_root")]
    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_root_f")]
    async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_root(checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32)
            .await
    }

    #[method(name = "get_contract_function_tree_leaf_hash")]
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_function_tree_leaf_hash_f")]
    async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        )
        .await
    }

    #[method(name = "get_contract_function_tree_merkle_proof")]
    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof(checkpoint_id, contract_id, function_id)
            .await
    }

    #[method(name = "get_contract_function_tree_merkle_proof_f")]
    async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        )
        .await
    }

    // Contract tree
    #[method(name = "get_contract_tree_root")]
    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_root_f")]
    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_contract_tree_leaf_hash")]
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_contract_tree_leaf_hash_f")]
    async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_leaf_hash(checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32)
            .await
    }

    #[method(name = "get_contract_tree_merkle_proof")]
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_contract_tree_merkle_proof_f")]
    async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_tree_merkle_proof(checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32)
            .await
    }

    // Deposit tree
    #[method(name = "get_deposit_tree_root")]
    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_root_f")]
    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_deposit_tree_leaf_hash")]
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_deposit_tree_leaf_hash_f")]
    async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_leaf_hash(checkpoint_id.to_canonical_u64(), deposit_id.to_canonical_u64() as u32)
            .await
    }

    #[method(name = "get_deposit_tree_merkle_proof")]
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_deposit_tree_merkle_proof_f")]
    async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_deposit_tree_merkle_proof(checkpoint_id.to_canonical_u64(), deposit_id.to_canonical_u64() as u32)
            .await
    }

    // Withdrawal tree
    #[method(name = "get_withdrawal_tree_root")]
    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_root_f")]
    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    #[method(name = "get_withdrawal_tree_leaf_hash")]
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_withdrawal_tree_leaf_hash_f")]
    async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_leaf_hash(checkpoint_id.to_canonical_u64(), withdrawal_id.to_canonical_u64() as u32)
            .await
    }

    #[method(name = "get_withdrawal_tree_merkle_proof")]
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_withdrawal_tree_merkle_proof_f")]
    async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_withdrawal_tree_merkle_proof(checkpoint_id.to_canonical_u64(), withdrawal_id.to_canonical_u64() as u32)
            .await
    }

    // Checkpoint tree
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
        self.get_checkpoint_tree_leaf_hash(checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64())
            .await
    }

    #[method(name = "get_checkpoint_tree_merkle_proof")]
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_tree_merkle_proof_f")]
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_checkpoint_tree_merkle_proof(checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64())
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


    #[method(name = "get_contract_metadata")]
    async fn get_contract_metadata(&self, contract_uuid: &str) -> RpcResult<ContractMetaData<F>>;

    #[method(name = "get_current_checkpoint_id")]
    async fn get_current_checkpoint_id(&self) -> RpcResult<u64>;
}
