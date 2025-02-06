

use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::{treeprover::subtree::SubTreeNodeStateTransition, utils::sub_tree_nca::PartialUpdateNearestCommonAncestorProof}, traits::{hasher::FieldQHasher, qhashable::QFieldHashable}};
use serde::{Deserialize, Serialize};

use super::{header::GlobalUserTreeAggregatorHeader, stats::GUTAStats};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyTwoGUTAProofGadgetStandardInput<F: RichField> {
    pub stats_a: GUTAStats<F>,
    pub stats_b: GUTAStats<F>,
    pub nca_proof: PartialUpdateNearestCommonAncestorProof<QHashOut<F>>,    
}

impl<F: RichField> KVQSerializable for VerifyTwoGUTAProofGadgetStandardInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

