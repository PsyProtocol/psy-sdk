
use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::CommonCircuitData,
        config::{AlgebraicHasher, GenericConfig},
    },
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::
    hash::{
        merkle::
            utils::simple_merkle_tree::SimpleMerkleTree
        ,
        traits::hasher::MerkleZeroHasher,
    }
;

use crate::{
    circuits::traits::qstandard::QStandardCircuit,
    treeprover::qrecursion::standard::{
        config::QRECURSION_CIRCUIT_WHITELIST_HEIGHT,
        manager::leaf_circuit_set::QStandardBinaryRecursionTreeCircuitSet,
    },
};

use qed_crypto::common::witnesses::qrecursion::proof_data::{
    QStandardBinaryTreeCircuitType, SimpleQTreeRecursionManagerInclusionProofs,
};
#[derive(Debug)]
pub struct PortableQTreeRecursionCircuits<C: GenericConfig<D>, const D: usize>
where
    C::Hasher:
       AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub circuit_set: QStandardBinaryRecursionTreeCircuitSet<C, D>,
    pub circuit_inclusion_proofs: SimpleQTreeRecursionManagerInclusionProofs<C::F>,
}

impl<C: GenericConfig<D>, const D: usize> PortableQTreeRecursionCircuits<C, D>
where
    C::Hasher:
       AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub fn new(
        q_recursion_tree_height: usize,
        leaf_circuit_config_id: u64,
        leaf_verifier_data_cap_height: usize,
        leaf_child_common_data: &CommonCircuitData<C::F, D>,
    ) -> Self {
        let circuit_set = QStandardBinaryRecursionTreeCircuitSet::<C, D>::new(
            q_recursion_tree_height,
            leaf_circuit_config_id,
            leaf_verifier_data_cap_height,
            leaf_child_common_data,
        );
        let mut tmp_circuit_whitelist_tree = SimpleMerkleTree::<C::Hasher, QHashOut<C::F>>::new(
            QRECURSION_CIRCUIT_WHITELIST_HEIGHT as u8,
        );

        tmp_circuit_whitelist_tree.set_leaf(
            QStandardBinaryTreeCircuitType::SingleLeaf.into(),
            circuit_set.single_leaf_circuit.get_fingerprint(),
        );

        tmp_circuit_whitelist_tree.set_leaf(
            QStandardBinaryTreeCircuitType::TwoLeaf.into(),
            circuit_set.two_leaf_circuit.get_fingerprint(),
        );

        tmp_circuit_whitelist_tree.set_leaf(
            QStandardBinaryTreeCircuitType::TwoAgg.into(),
            circuit_set.two_agg_circuit.get_fingerprint(),
        );

        tmp_circuit_whitelist_tree.set_leaf(
            QStandardBinaryTreeCircuitType::LeftAggRightLeaf.into(),
            circuit_set.left_agg_right_leaf_circuit.get_fingerprint(),
        );

        tmp_circuit_whitelist_tree.set_leaf(
            QStandardBinaryTreeCircuitType::LeftLeafRightAgg.into(),
            circuit_set.left_leaf_right_agg_circuit.get_fingerprint(),
        );

        let circuit_inclusion_proofs = SimpleQTreeRecursionManagerInclusionProofs {
            single_leaf_circuit_merkle_proof: tmp_circuit_whitelist_tree
                .get_leaf(QStandardBinaryTreeCircuitType::SingleLeaf.into()),
            two_leaf_circuit_merkle_proof: tmp_circuit_whitelist_tree
                .get_leaf(QStandardBinaryTreeCircuitType::TwoLeaf.into()),
            two_agg_circuit_merkle_proof: tmp_circuit_whitelist_tree
                .get_leaf(QStandardBinaryTreeCircuitType::TwoAgg.into()),
            left_leaf_right_agg_circuit_merkle_proof: tmp_circuit_whitelist_tree
                .get_leaf(QStandardBinaryTreeCircuitType::LeftLeafRightAgg.into()),
            left_agg_right_leaf_circuit_merkle_proof: tmp_circuit_whitelist_tree
                .get_leaf(QStandardBinaryTreeCircuitType::LeftAggRightLeaf.into()),
            circuit_whitelist_tree_root: tmp_circuit_whitelist_tree.get_root(),
        };
        Self {
            circuit_set,
            circuit_inclusion_proofs,
        }
    }
}
