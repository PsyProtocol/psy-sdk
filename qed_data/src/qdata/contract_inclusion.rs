use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::{QFeltSized, ToQFelts}};
use qed_crypto::hash::{merkle::core::MerkleProofCore, traits::{hasher::{FieldHasher, FieldQHasher, MerkleHasher}, qhashable::QFieldHashable}};
use serde::{Deserialize, Serialize};

use super::contract::QEDContractLeaf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractInclusionProof<F: RichField> {
    pub contract_tree_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub contract_leaf: QEDContractLeaf<F>,
}

impl<F: RichField> QEDContractInclusionProof<F> {
    pub fn verify<H: FieldQHasher<F>>(&self) -> bool {
        self.contract_tree_merkle_proof.value == self.contract_leaf.qfhash::<H>() && self.contract_tree_merkle_proof.verify::<H>()
        
    }
}

impl<F: RichField> KVQSerializable for QEDContractInclusionProof<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractFunctionInclusionProof<F: RichField> {
    pub contract_inclusion_proof: QEDContractInclusionProof<F>,
    pub contract_function_merkle_proof: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> QEDContractFunctionInclusionProof<F> {
    pub fn verify<H: FieldQHasher<F>>(&self) -> bool {
        true
    }
}

impl<F: RichField> KVQSerializable for QEDContractFunctionInclusionProof<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
