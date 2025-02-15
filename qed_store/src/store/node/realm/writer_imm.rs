use crate::{
    config::store_config::{
        CheckpointHashHelperTableStore, CheckpointLeafTableStore, CheckpointSyncInfoTableStore, CheckpointTreeStore, QEDHasher, UserPublicKeyTableStore, UserRegistrationTreeStore, UserTreeStore
    },
    models::{
        checkpoint::{
            checkpoint_hash::QEDCheckpointHashHelperModelCore,
            checkpoint_leaf::QEDCheckpointLeafModelCore, sync_info::QEDCheckpointSyncInfoModelCore, user_public_keys::QEDUserPublicKeyHelperModelCore,
        },
        kvq_merkle::model::{
            KVQFixedConfigMerkleTreeModelCoreImmutable, KVQFixedConfigMerkleTreeModelReaderCore,
            KVQMerkleTreeModelCoreImmutable,
        },
    },
    node::
        realm::QEDRealmStoreWriterAsyncImm
    ,
    store::imm::core::QEDStorageAdapterImmutable,
    traits::qdatastore::
        qmetadata::QMetaDataStoreWriterSync
    ,
};
fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
    registration_id.reverse_bits()
}
use async_trait::async_trait;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64},
    util::log2_ceil,
};
use qed_core::{
    config::network_constants::{GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use qed_crypto::hash::{
    merkle::
        utils::{common::QMerkleNode, sub_tree_nca::UpdateNCAProofsWithDependencies}
    ,
    traits::qhashable::QFieldHashable,
};
use qed_data::{
    qdata::{
        contract::{ContractCodeDefinition, QEDContractLeaf},
        user_public_key::QEDUserPublicKeyRecord,
    },
    qsync::coordinator::{QEDCheckpointSyncInfo, QEDCheckpointSyncInfoCompact},
};
type F = GoldilocksField;
#[async_trait]
impl<T: QEDStorageAdapterImmutable + Send + Sync> QEDRealmStoreWriterAsyncImm<F> for T {
    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>> {
        UserTreeStore::smart_injest_nca_fc_imm(self, REALM_USER_TREE_HEIGHT, checkpoint_id, nodes)
    }
    async fn injest_checkpoint_sync_data_imm(
        &self,
        sync_info: QEDCheckpointSyncInfo<F>,
    ) -> anyhow::Result<()> {
        let checkpoint_id = sync_info.core.l2_block_state.checkpoint_id;
        CheckpointTreeStore::<Self>::set_leaf_fc_imm(
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
}
