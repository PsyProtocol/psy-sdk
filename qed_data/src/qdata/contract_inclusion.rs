use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{
    merkle::core::MerkleProofCore,
    traits::{hasher::FieldQHasher, qhashable::QFieldHashable},
};
use serde::{Deserialize, Serialize};

use super::contract::QEDContractLeaf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractInclusionProof<F: RichField> {
    pub contract_leaf: QEDContractLeaf<F>,
    pub contract_tree_merkle_proof: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> QEDContractInclusionProof<F> {
    pub fn verify<H: FieldQHasher<F>>(&self) -> bool {
        self.contract_tree_merkle_proof.value == self.contract_leaf.qfhash::<H>()
            && self.contract_tree_merkle_proof.verify::<H>()
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractFunctionInclusionProof<F: RichField> {
    pub contract_inclusion_proof: QEDContractInclusionProof<F>,
    pub contract_function_merkle_proof: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> QEDContractFunctionInclusionProof<F> {
    pub fn verify<H: FieldQHasher<F>>(&self) -> bool {
        // must have a valid contract inclusion proof and a valid merkle proof with an even index
        // (even index is because each function uses two leaves) 
        self.contract_inclusion_proof.verify::<H>()
            && self.contract_function_merkle_proof.verify::<H>()
            && (self.contract_function_merkle_proof.index&1) == 0
    }

    // note that each function has two leaves:
    // **left** is the hash of the verifier key and **right** is [method_id, (num_outputs<<32)|num_inputs, 0, 0]

    pub fn get_function_verifier_fingerprint(&self) -> QHashOut<F> {
        self.contract_function_merkle_proof.value
    }
    pub fn get_method_id(&self) -> u32 {
        self.contract_function_merkle_proof.siblings[0].0.elements[0].to_canonical_u64() as u32
    }
    pub fn get_num_inputs(&self) -> usize {
        (self.contract_function_merkle_proof.siblings[0].0.elements[1].to_canonical_u64()
            & 0xFFFFFFFFu64) as usize
    }
    pub fn get_num_outputs(&self) -> usize {
        (self.contract_function_merkle_proof.siblings[0].0.elements[1].to_canonical_u64() >> 32u64)
            as usize
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
