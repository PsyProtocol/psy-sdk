use async_trait::async_trait;
use plonky2::hash::hash_types::RichField;
use psy_common::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::{
    core::{DeltaMerkleProofCore, MerkleProofCore},
    utils::{common::QMerkleNode, sub_tree_nca::UpdateNCAProofsWithDependencies},
};
use psy_data::{
    dpn::event::PsyUserEventRecord,
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        contract::{ContractCodeDefinition, PsyContractLeaf},
        user::PsyUserLeaf,
    },
    qstore::uct_merkle_nodes::CSTUserUpdate,
    qsync::coordinator::PsyCheckpointSyncInfo,
};

pub mod reader_async;
pub mod writer_imm;

#[async_trait]
pub trait PsyRealmStoreReaderAsync<F: RichField> {
    async fn get_first_user_id(&self, public_key: QHashOut<F>) -> anyhow::Result<u64>;
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointLeaf<F>>;
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<PsyCheckpointLeaf<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_checkpoint_leaf_data(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_latest_block_state(&self) -> anyhow::Result<PsyBlockState>;

    async fn get_block_state(&self, checkpoint_id: u64) -> anyhow::Result<PsyBlockState>;
    async fn get_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<PsyBlockState> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_block_state(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_checkpoint_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_checkpoint_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
        .await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_checkpoint_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
        .await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointGlobalStateRoots<F>>;

    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<PsyUserLeaf<F>>;
    async fn get_user_leaf_data_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<PsyUserLeaf<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_leaf_data(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_root_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_state_tree_root(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_state_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        )
        .await
    }
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_state_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        )
        .await
    }

    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_root_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_tree_root(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_contract_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_bottom_tree_merkle_proof(
        &self,
        root_level: u8,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_bottom_tree_merkle_proof_f(
        &self,
        root_level: u8,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_bottom_tree_merkle_proof(
            self,
            root_level,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        )
        .await
    }
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;

    async fn get_user_event_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        event_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_event_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, event_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_event_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            event_index.to_canonical_u64(),
        )
        .await
    }

    async fn get_user_event_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_event_tree_root_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_event_tree_root(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_event_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, event_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_event_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, event_index: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_event_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            event_index.to_canonical_u64(),
        )
        .await
    }

    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;

    async fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    async fn get_user_event_data(&self, checkpoint_id: u64, user_id: u64, event_index: u64) -> anyhow::Result<PsyUserEventRecord<F>>;
    async fn get_user_event_data_f(&self, checkpoint_id: F, user_id: F, event_index: F) -> anyhow::Result<PsyUserEventRecord<F>> {
        <Self as PsyRealmStoreReaderAsync<F>>::get_user_event_data(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            event_index.to_canonical_u64(),
        )
        .await
    }
}

#[async_trait]
pub trait PsyRealmStoreWriterAsyncImm<F: RichField> {
    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>>;
    async fn injest_user_leaves_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaves: &[PsyUserLeaf<F>],
    ) -> anyhow::Result<Vec<DeltaMerkleProofCore<QHashOut<F>>>>;
    async fn injest_user_leaves_batch_imm(&self, checkpoint_id: u64, leaves: &[PsyUserLeaf<F>]) -> anyhow::Result<()>;
    async fn injest_checked_cst_nodes_imm(&self, user_updates: &[CSTUserUpdate<QHashOut<F>>]) -> anyhow::Result<()>;
    async fn injest_checkpoint_sync_data_imm(&self, sync_info: PsyCheckpointSyncInfo<F>) -> anyhow::Result<()>;

    async fn injest_user_contract_events_batch_imm(
        &self,
        checkpoint_id: u64,
        start_event_index: u64,
        events: &[PsyUserEventRecord<F>],
    ) -> anyhow::Result<()>;

    async fn set_contract_leaf_data_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &PsyContractLeaf<F>) -> anyhow::Result<()>;
    async fn set_contract_leaf_data_f_imm(&self, checkpoint_id: F, contract_id: F, leaf_data: &PsyContractLeaf<F>) -> anyhow::Result<()> {
        <Self as PsyRealmStoreWriterAsyncImm<F>>::set_contract_leaf_data_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_data,
        )
        .await
    }

    async fn set_contract_code_definition_imm(&self, checkpoint_id: u64, contract_id: u64, definition: &ContractCodeDefinition)
        -> anyhow::Result<()>;
    async fn set_contract_code_definition_f_imm(&self, checkpoint_id: F, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()> {
        <Self as PsyRealmStoreWriterAsyncImm<F>>::set_contract_code_definition_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            definition,
        )
        .await
    }
}
