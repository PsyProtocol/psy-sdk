

use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{data::qhashout::QHashOut, job::id::QProvingJobDataID};
use qed_crypto::hash::merkle::{core::MerkleProofCore, treeprover::subtree::SubTreeNodeStateTransition, utils::sub_tree_nca::PartialUpdateNearestCommonAncestorProof};
use serde::{Deserialize, Serialize};

use crate::qdata::ups_end_cap_result::UPSEndCapResultCompact;

use super::{header::GlobalUserTreeAggregatorHeader, stats::GUTAStats};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyTwoGUTAProofGadgetStandardInput<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub stats_a: GUTAStats<F>,
    pub stats_b: GUTAStats<F>,
    pub nca_proof: PartialUpdateNearestCommonAncestorProof<QHashOut<F>>,

    pub guta_inclusion_proof_a: MerkleProofCore<QHashOut<F>>,
    pub guta_inclusion_proof_b: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for VerifyTwoGUTAProofGadgetStandardInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


impl<F: RichField> VerifyTwoGUTAProofGadgetStandardInput<F> {

    pub fn get_guta_header_a(&self) -> GlobalUserTreeAggregatorHeader<F> {
        GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.guta_inclusion_proof_a.root,
            checkpoint_tree_root: self.checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: self.nca_proof.child_a.old_value,
                new_node_value: self.nca_proof.child_a.new_value,
                node_index: F::from_canonical_u64(self.nca_proof.get_a_node_key().index),
                node_level: F::from_canonical_u8(self.nca_proof.get_level_a()),
            },
            stats: self.stats_a,
        }
    }
    pub fn get_guta_header_b(&self) -> GlobalUserTreeAggregatorHeader<F> {
        GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.guta_inclusion_proof_b.root,
            checkpoint_tree_root: self.checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: self.nca_proof.child_b.old_value,
                new_node_value: self.nca_proof.child_b.new_value,
                node_index: F::from_canonical_u64(self.nca_proof.get_b_node_key().index),
                node_level: F::from_canonical_u8(self.nca_proof.get_level_b()),
            },
            stats: self.stats_b,
        }
    }
    
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyEndCapSimpleStandardInput<F: RichField> {
    pub guta_stats: GUTAStats<F>,
    pub checkpoint_root: QHashOut<F>,
    pub checkpoint_historical_merkle_proof: MerkleProofCore<QHashOut<F>>,
}
impl<F: RichField> KVQSerializable for VerifyEndCapSimpleStandardInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyTwoEndCapCircuitWithIdsInput<F: RichField> {
    pub input: VerifyTwoEndCapCircuitInput<F>,

    pub proof_a_id: QProvingJobDataID,
    pub proof_b_id: QProvingJobDataID,
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyTwoEndCapCircuitInput<F: RichField> {
    pub guta_circuit_whitelist: QHashOut<F>,

    pub a_end_cap: VerifyEndCapSimpleStandardInput<F>,

    pub b_end_cap: VerifyEndCapSimpleStandardInput<F>,

    pub nca_proof: PartialUpdateNearestCommonAncestorProof<QHashOut<F>>,  
}

impl<F: RichField> VerifyTwoEndCapCircuitInput<F> {

    pub fn get_end_result_a(&self) -> UPSEndCapResultCompact<F> {
        UPSEndCapResultCompact {
            start_user_leaf_hash: self.nca_proof.child_a.old_value,
            end_user_leaf_hash: self.nca_proof.child_a.new_value,
            checkpoint_tree_root_hash: self.a_end_cap.checkpoint_root,
            user_id: F::from_canonical_u64(self.nca_proof.child_a.index),
        }
    }
    pub fn get_end_result_b(&self) -> UPSEndCapResultCompact<F> {
        UPSEndCapResultCompact {
            start_user_leaf_hash: self.nca_proof.child_b.old_value,
            end_user_leaf_hash: self.nca_proof.child_b.new_value,
            checkpoint_tree_root_hash: self.b_end_cap.checkpoint_root,
            user_id: F::from_canonical_u64(self.nca_proof.child_b.index),
        }
    }
    
}
impl<F: RichField> KVQSerializable for VerifyTwoEndCapCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}





#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifySingleEndCapInput<F: RichField> {
    pub guta_circuit_whitelist: QHashOut<F>,

    pub a_end_cap: VerifyEndCapSimpleStandardInput<F>,

    pub start_user_leaf_hash: QHashOut<F>,
    pub end_user_leaf_hash: QHashOut<F>,
    pub user_id: F,
}

impl<F: RichField> VerifySingleEndCapInput<F> {

    pub fn get_end_result_a(&self) -> UPSEndCapResultCompact<F> {
        UPSEndCapResultCompact {
            start_user_leaf_hash: self.start_user_leaf_hash,
            end_user_leaf_hash: self.end_user_leaf_hash,
            checkpoint_tree_root_hash: self.a_end_cap.checkpoint_root,
            user_id: self.user_id,
        }
    }
    
}
impl<F: RichField> KVQSerializable for VerifySingleEndCapInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyLeftGUTARightEndCapInput<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub stats_a: GUTAStats<F>,
    pub b_end_cap: VerifyEndCapSimpleStandardInput<F>,
    pub nca_proof: PartialUpdateNearestCommonAncestorProof<QHashOut<F>>,

    pub guta_inclusion_proof_a: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for VerifyLeftGUTARightEndCapInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


impl<F: RichField> VerifyLeftGUTARightEndCapInput<F> {

    pub fn get_guta_header_a(&self) -> GlobalUserTreeAggregatorHeader<F> {
        GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.guta_inclusion_proof_a.root,
            checkpoint_tree_root: self.checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: self.nca_proof.child_a.old_value,
                new_node_value: self.nca_proof.child_a.new_value,
                node_index: F::from_canonical_u64(self.nca_proof.get_a_node_key().index),
                node_level: F::from_canonical_u8(self.nca_proof.get_level_a()),
            },
            stats: self.stats_a,
        }
    }
    pub fn get_end_result_b(&self) -> UPSEndCapResultCompact<F> {
        UPSEndCapResultCompact {
            start_user_leaf_hash: self.nca_proof.child_b.old_value,
            end_user_leaf_hash: self.nca_proof.child_b.new_value,
            checkpoint_tree_root_hash: self.b_end_cap.checkpoint_root,
            user_id: F::from_canonical_u64(self.nca_proof.child_b.index),
        }
    }
    
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyLeftEndCapRightGUTAInput<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub stats_b: GUTAStats<F>,
    pub a_end_cap: VerifyEndCapSimpleStandardInput<F>,
    pub nca_proof: PartialUpdateNearestCommonAncestorProof<QHashOut<F>>,

    pub guta_inclusion_proof_b: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for VerifyLeftEndCapRightGUTAInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


impl<F: RichField> VerifyLeftEndCapRightGUTAInput<F> {

    pub fn get_guta_header_b(&self) -> GlobalUserTreeAggregatorHeader<F> {
        GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.guta_inclusion_proof_b.root,
            checkpoint_tree_root: self.checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: self.nca_proof.child_b.old_value,
                new_node_value: self.nca_proof.child_b.new_value,
                node_index: F::from_canonical_u64(self.nca_proof.get_b_node_key().index),
                node_level: F::from_canonical_u8(self.nca_proof.get_level_b()),
            },
            stats: self.stats_b,
        }
    }
    pub fn get_end_result_a(&self) -> UPSEndCapResultCompact<F> {
        UPSEndCapResultCompact {
            start_user_leaf_hash: self.nca_proof.child_a.old_value,
            end_user_leaf_hash: self.nca_proof.child_a.new_value,
            checkpoint_tree_root_hash: self.a_end_cap.checkpoint_root,
            user_id: F::from_canonical_u64(self.nca_proof.child_a.index),
        }
    }
    
}