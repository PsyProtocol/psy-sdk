use async_trait::async_trait;
use kvq::traits::KVQBinaryStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::{
    merkle::{
        core::DeltaMerkleProofCore,
        spiderman::SpidermanUpdateProof,
        utils::{common::QMerkleNode, sub_tree_nca::UpdateNCAProofsWithDependencies},
    },
    traits::qhashable::QFieldHashable,
};
use psy_data::{
    config::store_config::{CheckpointSyncInfoTableStore, PsyHasher, UserTreeStore},
    models::{
        checkpoint::sync_info::PsyCheckpointSyncInfoModelCore, kvq_merkle::model::KVQFixedConfigMerkleTreeModelCore,
        realm_status::RealmStatusModelCore,
    },
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointLeaf, PsyCheckpointLeafStats},
        contract::{ContractCodeDefinition, PsyContractLeaf},
        realm_status::BasicRealmStatus,
    },
    qsync::coordinator::PsyCheckpointSyncInfoCompact,
    traits::qdatastore::{qmetadata::QMetaDataStoreWriterSync, qtreedata::QTreeDataStoreWriterSync},
};

use super::InitializeParams;
use crate::node::coordinator::{PsyCoordinatorStoreReaderAsync, PsyCoordinatorStoreWriterAsyncImm};

type F = GoldilocksField;
#[async_trait]
impl<T: KVQBinaryStore + PsyCoordinatorStoreReaderAsync<F>> PsyCoordinatorStoreWriterAsyncImm<F> for T {
    async fn batch_append_contract_tree_imm(
        &self,
        checkpoint_id: u64,
        start_leaf_index: u64,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)> {
        <Self as QTreeDataStoreWriterSync<F>>::batch_append_contract_tree(self, checkpoint_id, start_leaf_index, sub_tree_height, leaf_hashes)
    }
    async fn batch_append_user_registration_tree_imm(
        &self,
        checkpoint_id: u64,
        start_leaf_index: u64,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)> {
        <Self as QTreeDataStoreWriterSync<F>>::batch_append_user_registration_tree(
            self,
            checkpoint_id,
            start_leaf_index,
            sub_tree_height,
            leaf_hashes,
        )
    }
    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>> {
        UserTreeStore::<Self>::smart_injest_nca_fc(self, root_level, checkpoint_id, nodes)
    }
    async fn set_deposit_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        deposit_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_deposit_tree_leaf_hash(self, checkpoint_id, deposit_id, leaf_hash)
    }
    async fn set_withdrawal_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_withdrawal_tree_leaf_hash(self, checkpoint_id, withdrawal_id, leaf_hash)
    }
    async fn set_contract_function_whitelist_imm(&self, checkpoint_id: u64, contract_id: u64, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_function_whitelist(self, checkpoint_id, contract_id, leaves)
    }
    async fn set_contract_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_tree_leaf_hash(self, checkpoint_id, contract_id, leaf_hash)
    }
    async fn set_checkpoint_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_checkpoint_tree_leaf_hash(self, checkpoint_id, leaf_hash)
    }
    async fn set_contract_leaf_data_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &PsyContractLeaf<F>) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_leaf_data(self, checkpoint_id, contract_id, leaf_data)
    }
    async fn set_checkpoint_leaf_data_imm(&self, checkpoint_id: u64, leaf_data: &PsyCheckpointLeaf<F>) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_checkpoint_leaf_data(self, checkpoint_id, leaf_data)
    }
    async fn set_contract_code_definition_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_code_definition(self, checkpoint_id, contract_id, definition)
    }
    async fn set_block_state_imm(&self, block_state: &PsyBlockState) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_block_state(self, block_state)
    }
    async fn set_checkpoint_sync_info_imm(&self, sync_info: PsyCheckpointSyncInfoCompact<F>) -> anyhow::Result<()> {
        CheckpointSyncInfoTableStore::<Self>::set_checkpoint_sync_info(self, sync_info)
    }

    async fn initialize_store(&self, params: Option<InitializeParams<F>>) -> anyhow::Result<u64> {
        let latest_block_state_or_err = self.get_latest_block_state().await;
        if let Ok(v) = latest_block_state_or_err {
            Ok(v.checkpoint_id)
        } else {
            let mut genesis_block_state = PsyBlockState::get_genesis_value();
            let genesis_checkpoint_stats = PsyCheckpointLeafStats::get_genesis_value();
            let genesis_global_state_roots = self.get_checkpoint_global_state_roots(0).await?;

            if let Some(params) = params {
                genesis_block_state.next_contract_id = params.next_contract_id;
                genesis_block_state.next_user_id = params.next_user_id;

                if genesis_global_state_roots.contract_tree_root != params.deploy_contracts_root {
                    return Err(anyhow::anyhow!(
                        "Contract tree root mismatch: expected {:?}, got {:?}",
                        params.deploy_contracts_root,
                        genesis_global_state_roots.contract_tree_root
                    ));
                }

                if genesis_global_state_roots.user_tree_root != params.gutas_root {
                    return Err(anyhow::anyhow!(
                        "User tree root mismatch: expected {:?}, got {:?}",
                        params.gutas_root,
                        genesis_global_state_roots.user_tree_root
                    ));
                }

                if genesis_global_state_roots.user_registration_tree_root != params.register_users_root {
                    return Err(anyhow::anyhow!(
                        "User registration root mismatch: expected {:?}, got {:?}",
                        params.register_users_root,
                        genesis_global_state_roots.user_registration_tree_root
                    ));
                }
            }

            let genesis_checkpoint_leaf = PsyCheckpointLeaf {
                global_chain_root: genesis_global_state_roots.qfhash::<PsyHasher>(),
                stats: genesis_checkpoint_stats,
            };

            self.set_block_state_imm(&genesis_block_state).await?;
            self.set_checkpoint_leaf_data_imm(0, &genesis_checkpoint_leaf).await?;
            let r = self
                .set_checkpoint_tree_leaf_hash_imm(0, genesis_checkpoint_leaf.qfhash::<PsyHasher>())
                .await?;

            let sync_info = PsyCheckpointSyncInfoCompact {
                block_state: genesis_block_state,
                stats: genesis_checkpoint_stats,
                state_roots: genesis_global_state_roots,
                checkpoint_tree_update_siblings: r.siblings.clone(),
                regsitered_users_start_pivot_siblings: vec![],
                registered_users: vec![],
                old_checkpoint_leaf_hash: r.old_value,
                slot: 0,
            };
            self.set_checkpoint_sync_info_imm(sync_info).await?;

            Ok(0)
        }
    }

    async fn set_user_public_key_records(&self, records: &[psy_data::qdata::user_public_key::PsyUserPublicKeyRecord<F>]) -> anyhow::Result<()> {
        use psy_data::{config::store_config::UserPublicKeyTableStore, models::checkpoint::user_public_keys::PsyUserPublicKeyHelperModelCore};

        UserPublicKeyTableStore::<Self>::set_user_public_key_records(self, records)
    }

    async fn set_realm_statuses(&self, realm_ids: &[u64], realm_statuses: &[BasicRealmStatus<F>]) -> anyhow::Result<()> {
        use psy_data::config::store_config::RealmStatusTableStore;
        RealmStatusTableStore::<F, Self>::set_realm_statuses(self, realm_ids, realm_statuses)
    }
}
