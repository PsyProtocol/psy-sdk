use std::marker::PhantomData;

use crate::hash::{
    merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
    traits::hasher::MerkleZeroHasher,
};

use super::common::SimpleMerkleNodeKey;

#[derive(Debug, Clone)]
pub struct SimpleMerkleTree<Hasher, Hash: Copy + PartialEq + Default> {
    nodes: hashbrown::HashMap<SimpleMerkleNodeKey, Hash>,
    height: u8,
    _hasher: PhantomData<Hasher>,
}

impl<Hasher: MerkleZeroHasher<Hash>, Hash: Copy + PartialEq + Default>
    SimpleMerkleTree<Hasher, Hash>
{
    pub fn new(height: u8) -> Self {
        Self {
            nodes: hashbrown::HashMap::new(),
            height,
            _hasher: PhantomData::default(),
        }
    }
    fn set_node_value(&mut self, key: SimpleMerkleNodeKey, value: Hash) {
        self.nodes.insert(key, value);
    }
    fn get_node_value(&self, key: &SimpleMerkleNodeKey) -> Hash {
        if self.nodes.contains_key(key) {
            self.nodes[key]
        } else {
            assert!(
                self.height >= key.level,
                "requested node value of invalid key level for this tree"
            );
            Hasher::get_zero_hash((self.height - key.level) as usize)
        }
    }

    pub fn get_root(&self) -> Hash {
        self.get_node_value(&SimpleMerkleNodeKey::new_root())
    }

    pub fn get_leaf_value(&self, index: u64) -> Hash {
        self.get_node_value(&SimpleMerkleNodeKey::new(self.height, index))
    }

    pub fn get_leaf(&self, index: u64) -> MerkleProofCore<Hash> {
        let leaf_key = SimpleMerkleNodeKey::new(self.height, index);
        let value = self.get_leaf_value(index);

        let mut current_sibling = leaf_key.sibling();
        let mut siblings = Vec::with_capacity(self.height as usize);

        while current_sibling.level > 0 {
            siblings.push(self.get_node_value(&current_sibling));
            current_sibling = current_sibling.parent().sibling();
        }

        let root = self.get_root();

        MerkleProofCore {
            index,
            siblings,
            root,
            value,
        }
    }

    pub fn set_leaf(&mut self, index: u64, value: Hash) -> DeltaMerkleProofCore<Hash> {
        let old_proof = self.get_leaf(index);
        let mut current_value = value;
        let mut current_key = SimpleMerkleNodeKey::new(self.height, index);

        let height = self.height as usize;
        for i in 0..height {
            let new_key = current_key.parent();
            let index = current_key.index;
            self.set_node_value(current_key, current_value);

            current_value = if index & 1 == 0 {
                Hasher::two_to_one(&current_value, &old_proof.siblings[i])
            } else {
                Hasher::two_to_one(&old_proof.siblings[i], &current_value)
            };
            current_key = new_key;
        }
        self.set_node_value(current_key, current_value);
        DeltaMerkleProofCore {
            old_root: old_proof.root,
            old_value: old_proof.value,

            new_root: current_value,
            new_value: value,

            siblings: old_proof.siblings,
            index: index,
        }
    }
    pub fn get_subtree_merkle_proof(&self, root_level: u8, subtree_leaf_node: SimpleMerkleNodeKey) -> MerkleProofCore<Hash> {
        if root_level > subtree_leaf_node.level {
            panic!("root_level > leaf node level");
        }
        let level_difference = subtree_leaf_node.level - root_level;
        
        let leaf_key = subtree_leaf_node;
        let value = self.get_node_value(&leaf_key);
        if level_difference == 0 {
            return MerkleProofCore {
                root: value,
                value: value,
                siblings: Vec::new(),
                index: subtree_leaf_node.index,
            };
        }

        let mut current_sibling = leaf_key.sibling();
        let mut siblings = Vec::with_capacity(level_difference as usize);

        while current_sibling.level > root_level {
            siblings.push(self.get_node_value(&current_sibling));
            current_sibling = current_sibling.parent().sibling();
        }

        let root = self.get_node_value(&subtree_leaf_node.parent_at_level(root_level));

        MerkleProofCore {
            index: subtree_leaf_node.index,
            siblings,
            root,
            value,
        }

    }

    pub fn get_leaf_in_subtree(&self, root_level: u8, leaf_level: u8, leaf_index: u64) -> MerkleProofCore<Hash> {
        self.get_subtree_merkle_proof(
            root_level,
            SimpleMerkleNodeKey::new(leaf_level, leaf_index),
        )
    }

    pub fn gen_fast_tree_inclusion_proofs(
        height: u8,
        leaves: &[Hash],
    ) -> anyhow::Result<Vec<MerkleProofCore<Hash>>> {
        let max_leaves = (1u64 << (height as u64)) as usize;
        let leaves_count = leaves.len();
        if leaves_count > max_leaves {
            anyhow::bail!("too many leaves for a tree of height {} (tried to add {} leaves, but max is {} leaves for this height)", height, leaves_count, max_leaves);
        } else {
            let mut tmp_tree = Self::new(height);
            for i in 0..leaves_count {
                tmp_tree.set_leaf(i as u64, leaves[i]);
            }

            let inclusion_proofs = (0..leaves_count)
                .map(|i| tmp_tree.get_leaf(i as u64))
                .collect::<Vec<_>>();

            Ok(inclusion_proofs)
        }
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use qed_core::data::qhashout::QHashOut;

    use crate::hash::traits::hasher::PoseidonHasher;

    use super::SimpleMerkleTree;

    #[test]
    fn test_merkle_tree_poseidon() {
        type Hash = QHashOut<GoldilocksField>;
        type Hasher = PoseidonHasher;
        let mut simple_tree = SimpleMerkleTree::<Hasher, Hash>::new(12);

        let values = [
            (0, QHashOut::from_values(1337, 123, 555, 1273)),
            (133, QHashOut::from_values(15, 12, 1, 3)),
            (1, QHashOut::from_values(5, 123, 555, 22)),
            (19, QHashOut::from_values(1337, 123, 4, 11)),
        ];

        let dmp_0 = simple_tree.set_leaf(1, QHashOut::from_values(1, 2, 3, 4));
        assert!(
            dmp_0.verify::<Hasher>(),
            "error verifying delta merkle proof in simple_tree"
        );

        for (index, value) in values {
            let dmp = simple_tree.set_leaf(index, value);
            assert!(
                dmp.verify::<Hasher>(),
                "error verifying delta merkle proof in simple_tree"
            );
        }
        for (index, value) in values {
            let mp = simple_tree.get_leaf(index);
            assert_eq!(value, mp.value, "values not saved in merkle tree");
        }
    }

    #[test]
    fn test_merkle_tree_poseidon_big() {
        type Hash = QHashOut<GoldilocksField>;
        type Hasher = PoseidonHasher;
        let mut simple_tree = SimpleMerkleTree::<Hasher, Hash>::new(50);

        let values = [
            (0, QHashOut::from_values(1337, 123, 555, 1273)),
            (123812937128, QHashOut::from_values(15, 12, 1, 3)),
            (1, QHashOut::from_values(5, 123, 555, 22)),
            (1124149, QHashOut::from_values(1337, 123, 4, 11)),
        ];

        let dmp_0 = simple_tree.set_leaf(1, QHashOut::from_values(1, 2, 3, 4));
        assert!(
            dmp_0.verify::<Hasher>(),
            "error verifying delta merkle proof in simple_tree"
        );

        for (index, value) in values {
            let dmp = simple_tree.set_leaf(index, value);
            assert!(
                dmp.verify::<Hasher>(),
                "error verifying delta merkle proof in simple_tree"
            );
        }
        for (index, value) in values {
            let mp = simple_tree.get_leaf(index);
            assert_eq!(value, mp.value, "values not saved in merkle tree");
        }
    }
}
