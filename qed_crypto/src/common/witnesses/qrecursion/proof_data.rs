use plonky2::{hash::hash_types::RichField, plonk::{circuit_data::{VerifierCircuitData, VerifierOnlyCircuitData}, config::GenericConfig, proof::ProofWithPublicInputs}};
use qed_core::data::qhashout::QHashOut;
use crate::{common::witnesses::qrecursion::header::QRecursionAggStandardHeader, hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore}};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum QStandardBinaryTreeCircuitType {
    None = 0,
    SingleLeaf = 1,
    TwoLeaf = 2,
    TwoAgg = 3,
    LeftLeafRightAgg = 4,
    LeftAggRightLeaf = 5,
}

impl QStandardBinaryTreeCircuitType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<QStandardBinaryTreeCircuitType> for u64 {
    fn from(value: QStandardBinaryTreeCircuitType) -> Self {
        (value as u8) as u64
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SimpleQTreeRecursionManagerInclusionProofs<F: RichField> {
    pub single_leaf_circuit_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub two_leaf_circuit_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub two_agg_circuit_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub left_leaf_right_agg_circuit_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub left_agg_right_leaf_circuit_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub circuit_whitelist_tree_root: QHashOut<F>,
}
impl<F: RichField> SimpleQTreeRecursionManagerInclusionProofs<F> {
    pub fn get_inclusion_proof_for_type(
        &self,
        circuit_type: QStandardBinaryTreeCircuitType,
    ) -> &MerkleProofCore<QHashOut<F>> {
        match circuit_type {
            QStandardBinaryTreeCircuitType::None => {
                panic!("tried to get an inclusion proof for circuit type 'None'")
            }
            QStandardBinaryTreeCircuitType::SingleLeaf => &self.single_leaf_circuit_merkle_proof,
            QStandardBinaryTreeCircuitType::TwoLeaf => &self.two_leaf_circuit_merkle_proof,
            QStandardBinaryTreeCircuitType::TwoAgg => &self.two_agg_circuit_merkle_proof,
            QStandardBinaryTreeCircuitType::LeftLeafRightAgg => {
                &self.left_leaf_right_agg_circuit_merkle_proof
            }
            QStandardBinaryTreeCircuitType::LeftAggRightLeaf => {
                &self.left_agg_right_leaf_circuit_merkle_proof
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct AggProofRecord<C: GenericConfig<D>, const D: usize> {
    pub circuit_type: QStandardBinaryTreeCircuitType,
    pub fingerprint: QHashOut<C::F>,
    pub agg_header: QRecursionAggStandardHeader<C::F>,
    pub proof: ProofWithPublicInputs<C::F, C, D>,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
pub struct LeafProofRecord<C: GenericConfig<D>, const D: usize> {
    pub fingerprint: QHashOut<C::F>,
    pub insertion_proof: DeltaMerkleProofCore<QHashOut<C::F>>,
    pub proof: ProofWithPublicInputs<C::F, C, D>,
    pub verifier_data: VerifierOnlyCircuitData<C, D>,
}


#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
pub struct InputLeafProof<C: GenericConfig<D>, const D: usize> {
    pub fingerprint: QHashOut<C::F>,
    pub proof: ProofWithPublicInputs<C::F, C, D>,
    pub verifier_data: VerifierOnlyCircuitData<C, D>,
}

