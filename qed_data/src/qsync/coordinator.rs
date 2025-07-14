use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL, data::qhashout::QHashOut, job::{drain_queue::{DrainQueueMetadata, DrainQueueMetadataTagged}, history_queue::{HistoryQueueMetadata, HistoryQueueMetadataTagged}}};
use qed_crypto::{hash::{merkle::{core::{DeltaMerkleProofCore, MerkleProofCore}, utils::append_only_merkle_tree::get_merkle_proofs_for_compact}, traits::{hasher::{FieldQHasher, MerkleZeroHasher}, qhashable::QFieldHashable}}, signature::zk::data::ZKPublicKeyInfo};
use serde::{Deserialize, Serialize};

use crate::qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDCheckpointLeafStats, QEDL2BlockState};


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDCheckpointSyncInfoCompact<F: RichField> {
    pub l2_block_state: QEDL2BlockState,
    pub stats: QEDCheckpointLeafStats<F>,
    pub state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub checkpoint_tree_update_siblings: Vec<QHashOut<F>>,
    pub regsitered_users_start_pivot_siblings: Vec<QHashOut<F>>,
    pub registered_users: Vec<ZKPublicKeyInfo<F>>,
    pub old_checkpoint_leaf_hash: QHashOut<F>,
}

impl<F: RichField> QEDCheckpointSyncInfoCompact<F> {

    pub fn get_registered_user_merkle_proofs<H: MerkleZeroHasher<QHashOut<F>>+ FieldQHasher<F>>(&self) -> Vec<MerkleProofCore<QHashOut<F>>> {
        get_merkle_proofs_for_compact::<H, QHashOut<F>>(
            self.l2_block_state.next_user_id - (self.registered_users.len() as u64),
            &self.regsitered_users_start_pivot_siblings,
            &self.registered_users.iter().map(|x|x.qfhash::<H>()).collect::<Vec<_>>(),
        )
    }

}
impl<F: RichField>  DrainQueueMetadataTagged for QEDCheckpointSyncInfoCompact<F> {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            checkpoint_id: self.l2_block_state.checkpoint_id,
            item_id: self.l2_block_state.checkpoint_id,
        }
        
    }
}

impl<F: RichField>  HistoryQueueMetadataTagged for QEDCheckpointSyncInfoCompact<F> {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        HistoryQueueMetadata {
            channel_id: QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            checkpoint_id: self.l2_block_state.checkpoint_id,
            item_id: self.l2_block_state.checkpoint_id,
        }
        
    }
}

impl<F: RichField> QEDCheckpointSyncInfoCompact<F> {

    pub fn to_sync_info<H: FieldQHasher<F>>(self) -> QEDCheckpointSyncInfo<F> {
        let global_chain_root = self.state_roots.qfhash::<H>();
        let checkpoint_leaf = QEDCheckpointLeaf {
            global_chain_root,
            stats: self.stats,
        };
        let checkpoint_leaf_hash = checkpoint_leaf.qfhash::<H>();

        let checkpoint_tree_update_proof = DeltaMerkleProofCore::from_params::<H>(
            self.l2_block_state.checkpoint_id, 
            self.old_checkpoint_leaf_hash,
            checkpoint_leaf_hash,
            self.checkpoint_tree_update_siblings
        );
        let core = QEDCheckpointCoreSyncInfo{
            checkpoint_tree_root: checkpoint_tree_update_proof.new_root,
            checkpoint_leaf_hash,
            l2_block_state: self.l2_block_state,
            checkpoint_leaf,
            state_roots: self.state_roots,
        };
        QEDCheckpointSyncInfo {
            core,
            checkpoint_tree_update_proof,
            regsitered_users_start_pivot_siblings: self.regsitered_users_start_pivot_siblings,
            registered_users: self.registered_users,
        }


    }
}

impl<F: RichField> From<QEDCheckpointSyncInfo<F>> for QEDCheckpointSyncInfoCompact<F> {
    fn from(value: QEDCheckpointSyncInfo<F>) -> Self {
        Self {
            l2_block_state: value.core.l2_block_state,
            stats: value.core.checkpoint_leaf.stats,
            state_roots: value.core.state_roots,
            checkpoint_tree_update_siblings: value.checkpoint_tree_update_proof.siblings,
            regsitered_users_start_pivot_siblings: value.regsitered_users_start_pivot_siblings,
            registered_users: value.registered_users,
            old_checkpoint_leaf_hash: value.checkpoint_tree_update_proof.old_value,
        }
        
    }
}
impl<F: RichField> From<&QEDCheckpointSyncInfo<F>> for QEDCheckpointSyncInfoCompact<F> {
    fn from(value: &QEDCheckpointSyncInfo<F>) -> Self {
        Self {
            l2_block_state: value.core.l2_block_state,
            stats: value.core.checkpoint_leaf.stats,
            state_roots: value.core.state_roots,
            checkpoint_tree_update_siblings: value.checkpoint_tree_update_proof.siblings.clone(),
            regsitered_users_start_pivot_siblings: value.regsitered_users_start_pivot_siblings.clone(),
            registered_users: value.registered_users.clone(),
            old_checkpoint_leaf_hash: value.checkpoint_tree_update_proof.old_value,
        }
        
    }
}


impl<F: RichField> KVQSerializable for QEDCheckpointSyncInfoCompact<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDCheckpointCoreSyncInfo<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub checkpoint_leaf_hash: QHashOut<F>,
    pub l2_block_state: QEDL2BlockState,
    pub checkpoint_leaf: QEDCheckpointLeaf<F>,
    pub state_roots: QEDCheckpointGlobalStateRoots<F>,
}

impl<F: RichField> KVQSerializable for QEDCheckpointCoreSyncInfo<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDCheckpointSyncInfo<F: RichField> {
    pub core: QEDCheckpointCoreSyncInfo<F>,
    pub checkpoint_tree_update_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub regsitered_users_start_pivot_siblings: Vec<QHashOut<F>>,
    pub registered_users: Vec<ZKPublicKeyInfo<F>>,
}
impl<F: RichField> QEDCheckpointSyncInfo<F> {

    pub fn get_registered_user_merkle_proofs<H: MerkleZeroHasher<QHashOut<F>>+ FieldQHasher<F>>(&self) -> Vec<MerkleProofCore<QHashOut<F>>> {
        get_merkle_proofs_for_compact::<H, QHashOut<F>>(
            self.core.l2_block_state.next_user_id - (self.registered_users.len() as u64),
            &self.regsitered_users_start_pivot_siblings,
            &self.registered_users.iter().map(|x|x.qfhash::<H>()).collect::<Vec<_>>(),
        )
    }

}
impl<F: RichField> KVQSerializable for QEDCheckpointSyncInfo<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
