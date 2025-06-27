use crate::{
    config::store_config::{
        BaseContractStateTreeStore, CheckpointHashHelperTableStore, CheckpointLeafTableStore, CheckpointSyncInfoTableStore, CheckpointTreeStore, QEDHasher, UserContractTreeStore, UserPublicKeyTableStore, UserRegistrationTreeStore, UserTreeStore, CONTRACT_STATE_TREE_ID, USER_CONTRACT_STATE_TREE_TABLE_TYPE
    },
    models::{
        checkpoint::{
            checkpoint_hash::QEDCheckpointHashHelperModelCore,
            checkpoint_leaf::QEDCheckpointLeafModelCore, sync_info::QEDCheckpointSyncInfoModelCore, user_public_keys::QEDUserPublicKeyHelperModelCore,
        },
        kvq_merkle::{key::KVQMerkleNodeKey, model::{
            KVQFixedConfigMerkleTreeModelCore, KVQFixedConfigMerkleTreeModelReaderCore,
            KVQMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore,
        }},
    },
    node::
        realm::QEDRealmStoreWriterAsyncImm
    ,
    store::imm::core::QEDStorageAdapterImmutable,
    traits::qdatastore::
        qmetadata::QMetaDataStoreWriterSync
    ,
};
use async_trait::async_trait;
use kvq::traits::KVQPair;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64},
    util::log2_ceil,
};
use qed_core::{
    config::network_constants::GLOBAL_USER_TREE_HEIGHT,
    data::qhashout::QHashOut,
};
use qed_crypto::{common::user_id::get_user_id_from_registration_id, hash::{
    merkle::{core::DeltaMerkleProofCore,
        utils::{common::{QMerkleNode, SimpleMerkleNode, SimpleMerkleNodeKey}, sub_tree_nca::UpdateNCAProofsWithDependencies}}
    ,
    traits::qhashable::QFieldHashable,
}};
use qed_data::{
    qdata::{
        contract::{ContractCodeDefinition, QEDContractLeaf}, user::QEDUserLeaf, user_public_key::QEDUserPublicKeyRecord
    }, qstore::uct_merkle_nodes::CSTUserUpdate, qsync::coordinator::{QEDCheckpointSyncInfo, QEDCheckpointSyncInfoCompact}
};
type F = GoldilocksField;
#[async_trait]
impl<T: QEDStorageAdapterImmutable + Send + Sync> QEDRealmStoreWriterAsyncImm<F> for T {
    async fn injest_user_leaves_batch_imm(&self, checkpoint_id: u64, leaves: &[QEDUserLeaf<F>]) -> anyhow::Result<()> {
        for l in leaves.iter() {
            self.set_user_leaf_data(checkpoint_id, l)?;
        }

        Ok(())
    }
    async fn injest_user_leaves_imm(&self, checkpoint_id: u64, root_level: u8, leaves: &[QEDUserLeaf<F>]) -> anyhow::Result<Vec<DeltaMerkleProofCore<QHashOut<F>>>> {
        for l in leaves.iter() {
            self.set_user_leaf_data(checkpoint_id, l)?;
        }

        let mut nodes = Vec::new();
        for l in leaves.iter() {
            nodes.push(SimpleMerkleNode {
                key: SimpleMerkleNodeKey {
                    index: l.user_id.to_canonical_u64(),
                    level: GLOBAL_USER_TREE_HEIGHT,
                },value: l.qfhash::<QEDHasher>()
            });
        }
        UserTreeStore::<Self>::smart_injest_nca_at_height_dmp_fc(self, root_level, checkpoint_id, &nodes)
    }
    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>> {
        UserTreeStore::smart_injest_nca_fc(self, root_level, checkpoint_id, nodes)
    }
    async fn injest_checkpoint_sync_data_imm(
        &self,
        sync_info: QEDCheckpointSyncInfo<F>,
    ) -> anyhow::Result<()> {
        let checkpoint_id = sync_info.core.l2_block_state.checkpoint_id;
        CheckpointTreeStore::<Self>::set_leaf_fc(
            self,
            checkpoint_id,
            checkpoint_id,
            sync_info.core.checkpoint_leaf_hash,
        )?;
        CheckpointLeafTableStore::<Self>::set_checkpoint_leaf(
            self,
            checkpoint_id,
            sync_info.core.checkpoint_leaf,
        )?;
        CheckpointHashHelperTableStore::<Self>::set_checkpoint_hash_helper_info(
            self,
            checkpoint_id,
            sync_info.core.checkpoint_leaf_hash,
            sync_info.core.checkpoint_tree_root,
        )?;
        let start_registration_user_id =
            sync_info.core.l2_block_state.next_user_id - (sync_info.registered_users.len() as u64);

        let new_user_records = sync_info.registered_users.iter().enumerate().map(|(i, x)| {
            let registration_id = start_registration_user_id + (i as u64);
            let user_id = get_user_id_from_registration_id(registration_id);
            QEDUserPublicKeyRecord {
                public_key_param: x.public_key_param,
                fingerprint: x.fingerprint,
                public_key: x.qfhash::<QEDHasher>(),
                user_id,
                checkpoint_id,
            }
        }).collect::<Vec<_>>();
        tracing::info!(
            "injest_checkpoint_sync_data_imm: start_registration_user_id: {}, new_user_records: {:?}",
            start_registration_user_id,
            new_user_records
        );
        UserPublicKeyTableStore::<Self>::set_user_public_key_records(self, &new_user_records)?;
        UserRegistrationTreeStore::<Self>::append_leaves_spider_man(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserRegistrationTreeStore::<Self>::new_leaf_key_fc(
                checkpoint_id,
                sync_info.core.l2_block_state.next_user_id
                    - (sync_info.registered_users.len() as u64),
            ),
            log2_ceil(sync_info.registered_users.len()).min(8) as u8,
            &new_user_records
                .iter()
                .map(|x| x.public_key)
                .collect::<Vec<_>>(),
        )?;

        let checkpoint_sync_info: QEDCheckpointSyncInfoCompact<F> = sync_info.into();
        CheckpointSyncInfoTableStore::set_checkpoint_sync_info(self, checkpoint_sync_info)?;

        Ok(())
    }

    async fn set_contract_leaf_data_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_leaf_data(
            &self,
            checkpoint_id,
            contract_id,
            leaf_data,
        )
    }
    async fn set_contract_leaf_data_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        <Self as QEDRealmStoreWriterAsyncImm<F>>::set_contract_leaf_data_imm(
            &self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_data,
        )
        .await
    }

    async fn set_contract_code_definition_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_code_definition(
            &self,
            checkpoint_id,
            contract_id,
            definition,
        )
    }
    async fn set_contract_code_definition_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        <Self as QEDRealmStoreWriterAsyncImm<F>>::set_contract_code_definition_imm(
            &self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            definition,
        )
        .await
    }

    async fn commit_block_imm(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        Ok(())
    }
    async fn injest_checked_cst_nodes_imm(&self, user_updates: &[CSTUserUpdate<QHashOut<F>>]) -> anyhow::Result<()> {
        for upd in user_updates.iter() {

            let nodes = upd.updates.iter().map(|x|{
                KVQPair {
                    key: KVQMerkleNodeKey::<USER_CONTRACT_STATE_TREE_TABLE_TYPE> {
                        tree_id: CONTRACT_STATE_TREE_ID,
                        primary_id: upd.user_id,
                        secondary_id: x.key.contract_id,
                        level: x.key.level,
                        index: x.key.index,
                        checkpoint_id: upd.checkpoint_id,
                    },
                    value: x.value,
                }
            }).collect::<Vec<_>>();
            eprintln!("DEBUGPRINT[572]: writer_imm.rs:215: nodes={}", serde_json::to_string_pretty(&nodes).unwrap());
            let uct_nodes = upd.uct_updates.iter().map(|x|{
                KVQPair {
                    key: UserContractTreeStore::<Self>::new_node_key_sfc(upd.checkpoint_id, upd.user_id, x.key.level, x.key.index),
                    value: x.value,
                }
            }).collect::<Vec<_>>();
            eprintln!("DEBUGPRINT[571]: writer_imm.rs:221: uct_nodes={}", serde_json::to_string_pretty(&uct_nodes).unwrap());
            BaseContractStateTreeStore::<Self>::set_nodes(self, &nodes)?;
            UserContractTreeStore::<Self>::set_nodes(self, &uct_nodes)?;
        }

        Ok(())
    }
}
