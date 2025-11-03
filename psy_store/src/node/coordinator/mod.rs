use async_trait::async_trait;
use plonky2::hash::hash_types::{HashOut, RichField};
use psy_common::data::qhashout::QHashOut;
use psy_data::qdata::{contract_metadata::ContractMetaData, contract_uuid::ContractUUID, realm_status::BasicRealmStatus};

#[derive(Debug, Clone)]
pub struct InitializeParams<F: RichField> {
    pub gutas_root: QHashOut<F>,
    pub deploy_contracts_root: QHashOut<F>,
    pub register_users_root: QHashOut<F>,
    pub next_contract_id: u32,
    pub next_user_id: u64,
}

impl<F: RichField> Default for InitializeParams<F> {
    fn default() -> Self {
        Self {
            gutas_root: QHashOut::ZERO,
            deploy_contracts_root: QHashOut::ZERO,
            register_users_root: QHashOut::ZERO,
            next_contract_id: 0,
            next_user_id: 0,
        }
    }
}
use psy_crypto::hash::merkle::{
    core::{DeltaMerkleProofCore, MerkleProofCore},
    spiderman::SpidermanUpdateProof,
    utils::{
        common::QMerkleNode,
        sub_tree_nca::{NCAProofsWithTopLine, UpdateNCAProofsWithDependencies},
    },
};
use psy_data::{
    config::store_config::UserPublicKeyTableStore,
    models::checkpoint::user_public_keys::PsyUserPublicKeyHelperModelCore,
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        contract::{ContractCodeDefinition, PsyContractLeaf},
        user_public_key::PsyUserPublicKeyRecord,
    },
    qsync::coordinator::PsyCheckpointSyncInfoCompact,
};

pub mod reader_async;
pub mod writer_imm;

#[async_trait]
pub trait PsyCoordinatorStoreReaderAsync<F: RichField>: Send + Sync {
    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<PsyContractLeaf<F>>;
    async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<PsyContractLeaf<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_leaf_data(self, contract_id.to_canonical_u64()).await
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointLeaf<F>>;
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<PsyCheckpointLeaf<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_checkpoint_leaf_data(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_code_definition(self, contract_id.to_canonical_u64()).await
    }
    async fn get_latest_block_state(&self) -> anyhow::Result<PsyBlockState>;

    async fn get_block_state(&self, checkpoint_id: u64) -> anyhow::Result<PsyBlockState>;
    async fn get_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<PsyBlockState> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_block_state(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_index.to_canonical_u64(),
        )
        .await
    }
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_index.to_canonical_u64(),
        )
        .await
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_user_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_top_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;

    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_root(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        )
        .await
    }

    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_contract_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        )
        .await
    }

    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_deposit_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_deposit_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_deposit_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64() as u32,
        )
        .await
    }

    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64() as u32,
        )
        .await
    }
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64() as u32,
        )
        .await
    }

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
        .await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
        .await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointGlobalStateRoots<F>>;
    async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointSyncInfoCompact<F>>;

    async fn get_first_user_id(&self, public_key: QHashOut<F>) -> anyhow::Result<u64>;

    async fn get_realm_status(&self, realm_id: u64) -> anyhow::Result<BasicRealmStatus<F>> {
        let realm_statuses = self.get_realm_statuses(&[realm_id]).await?;
        if realm_statuses.len() != 1 {
            return Err(anyhow::anyhow!(
                "get_realm_status should return only 1, but return {} realm status",
                realm_statuses.len()
            ));
        }
        Ok(realm_statuses[0])
    }
    async fn get_realm_statuses(&self, realm_ids: &[u64]) -> anyhow::Result<Vec<BasicRealmStatus<F>>>;

    async fn get_contract_metadata(&self, contract_uuid: ContractUUID) -> anyhow::Result<ContractMetaData<F>> {
        let contract_metadatas = self.get_contract_metadatas(&[contract_uuid]).await?;
        if contract_metadatas.len() != 1 {
            return Err(anyhow::anyhow!(
                "get_contract_metadata should return only 1, but return {} contract metadata",
                contract_metadatas.len()
            ));
        }
        Ok(contract_metadatas[0])
    }
    async fn get_contract_metadatas(&self, contract_uuids: &[ContractUUID]) -> anyhow::Result<Vec<ContractMetaData<F>>>;
}
#[async_trait]
pub trait PsyCoordinatorStoreWriterAsyncImm<F: RichField> {
    async fn batch_append_user_registration_tree_imm(
        &self,
        checkpoint_id: u64,
        start_leaf_index: u64,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)>;
    async fn batch_append_user_registration_tree_f_imm(
        &self,
        checkpoint_id: F,
        start_leaf_index: F,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::batch_append_user_registration_tree_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            start_leaf_index.to_canonical_u64(),
            sub_tree_height,
            leaf_hashes,
        )
        .await
    }

    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>>;

    async fn set_deposit_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        deposit_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_deposit_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        deposit_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_deposit_tree_leaf_hash_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64(),
            leaf_hash,
        )
        .await
    }

    async fn set_withdrawal_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_withdrawal_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_withdrawal_tree_leaf_hash_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64(),
            leaf_hash,
        )
        .await
    }

    async fn set_contract_function_whitelist_imm(&self, checkpoint_id: u64, contract_id: u64, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>>;
    async fn set_contract_function_whitelist_f_imm(&self, checkpoint_id: F, contract_id: F, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_contract_function_whitelist_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaves,
        )
        .await
    }

    async fn batch_append_contract_tree_imm(
        &self,
        checkpoint_id: u64,
        start_leaf_index: u64,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)>;

    async fn set_contract_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_contract_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_contract_tree_leaf_hash_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_hash,
        )
        .await
    }

    async fn set_checkpoint_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_checkpoint_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_checkpoint_tree_leaf_hash_imm(self, checkpoint_id.to_canonical_u64(), leaf_hash).await
    }

    async fn set_contract_leaf_data_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &PsyContractLeaf<F>) -> anyhow::Result<()>;
    async fn set_contract_leaf_data_f_imm(&self, checkpoint_id: F, contract_id: F, leaf_data: &PsyContractLeaf<F>) -> anyhow::Result<()> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_contract_leaf_data_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_data,
        )
        .await
    }

    async fn set_checkpoint_leaf_data_imm(&self, checkpoint_id: u64, leaf_data: &PsyCheckpointLeaf<F>) -> anyhow::Result<()>;
    async fn set_checkpoint_leaf_data_f_imm(&self, checkpoint_id: F, leaf_data: &PsyCheckpointLeaf<F>) -> anyhow::Result<()> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_checkpoint_leaf_data_imm(self, checkpoint_id.to_canonical_u64(), leaf_data).await
    }

    async fn set_contract_code_definition_imm(&self, checkpoint_id: u64, contract_id: u64, definition: &ContractCodeDefinition)
        -> anyhow::Result<()>;
    async fn set_contract_code_definition_f_imm(&self, checkpoint_id: F, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()> {
        <Self as PsyCoordinatorStoreWriterAsyncImm<F>>::set_contract_code_definition_imm(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            definition,
        )
        .await
    }

    async fn set_block_state_imm(&self, block_state: &PsyBlockState) -> anyhow::Result<()>;
    async fn set_checkpoint_sync_info_imm(&self, sync_info: PsyCheckpointSyncInfoCompact<F>) -> anyhow::Result<()>;
    async fn initialize_store(&self, params: Option<InitializeParams<F>>) -> anyhow::Result<u64>;

    async fn set_user_public_key_records(&self, records: &[PsyUserPublicKeyRecord<F>]) -> anyhow::Result<()>;

    async fn set_realm_status(&self, realm_id: u64, realm_status: &BasicRealmStatus<F>) -> anyhow::Result<()> {
        self.set_realm_statuses(&[realm_id], &[realm_status.clone()]).await
    }

    async fn set_realm_statuses(&self, realm_ids: &[u64], realm_statuses: &[BasicRealmStatus<F>]) -> anyhow::Result<()>;

    async fn set_contract_metadata(&self, contract_uuid: ContractUUID, contract_metadata: &ContractMetaData<F>) -> anyhow::Result<()> {
        self.set_contract_metadatas(&[contract_uuid], &[contract_metadata.clone()]).await
    }

    async fn set_contract_metadatas(&self, contract_uuids: &[ContractUUID], contract_metadatas: &[ContractMetaData<F>]) -> anyhow::Result<()>;
}
