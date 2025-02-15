
use super::super::key::KVQMerkleNodeKey;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQBinaryStoreReader;
use kvq::traits::KVQPair;
use kvq::traits::KVQSerializable;
use kvq::traits::KVQStoreAdapter;
use kvq::traits::KVQStoreAdapterReader;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::hash::traits::hasher::MerkleZeroHasherWithMarkedLeaf;

pub const CHECKPOINT_ID_FUZZY_SIZE: usize = 8;

pub trait KVQMerkleTreeModelReaderCore<
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStoreReader,
    KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>
{
    fn get_node_exact(store: &S, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash> {
        KVA::get_exact(store, key)
    }
    fn get_nodes_exact_vec(
        store: &S,
        keys: &[KVQMerkleNodeKey<TABLE_TYPE>],
    ) -> anyhow::Result<Vec<Hash>> {
        KVA::get_many_exact(store, keys)
    }
    fn get_node_optional(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        KVA::get_leq_kv(store, key, CHECKPOINT_ID_FUZZY_SIZE)
    }
    fn get_node(
        store: &S,
        tree_height: usize,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<Hash> {
        match KVA::get_leq(store, key, CHECKPOINT_ID_FUZZY_SIZE)? {
            Some(v) => Ok(v),
            None => {
                if MARK_LEAVES {
                    return Ok(Hasher::get_zero_hash_marked(
                        tree_height - (key.level as usize),
                    ));
                } else {
                    Ok(Hasher::get_zero_hash(tree_height - (key.level as usize)))
                }
            }
        }
    }
    fn get_nodes(
        store: &S,
        tree_height: usize,
        keys: &[KVQMerkleNodeKey<TABLE_TYPE>],
    ) -> anyhow::Result<Vec<Hash>> {
        let result = KVA::get_many_leq(store, keys, CHECKPOINT_ID_FUZZY_SIZE)?;
        Ok(result
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Some(v) => *v,
                None => Hasher::get_zero_hash(tree_height - (keys[i].level as usize)),
            })
            .collect())
    }
    fn get_sub_tree_proof(
        store: &S,
        real_tree_height: usize,
        root_level: u8,
        leaf_key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        if root_level > leaf_key.level {
            anyhow::bail!("cannot get a subtree proof for a root below the leaf");
        }

        let level_difference = (leaf_key.level - root_level) as usize;

        if level_difference == 0 {
            let value = Self::get_node(store, real_tree_height, leaf_key)?;
            return Ok(MerkleProofCore{
                root: value,
                value,
                index: leaf_key.index,
                siblings: Vec::new(),
            });

        }
        let mut node_keys = Vec::with_capacity(2+level_difference);
        node_keys.push(*leaf_key);
        node_keys.append(&mut leaf_key.siblings_above(level_difference));
        node_keys.push(leaf_key.parent_at_level(root_level));
        

        let nodes = Self::get_nodes(
            store,
            real_tree_height,
            &node_keys,
        )?;
        let value = nodes[0];
        let root_ind = nodes.len() - 1;
        let siblings = nodes[1..root_ind].to_vec();
        let root = nodes[root_ind];
        Ok(MerkleProofCore::<Hash> {
            root,
            value,
            siblings,
            index: leaf_key.index,
        })
    }
    fn get_leaf(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        let nodes = Self::get_nodes(
            store,
            key.level as usize,
            &vec![vec![*key], key.siblings(), vec![key.root()]].concat(),
        )?;
        let value = nodes[0];
        let root_ind = nodes.len() - 1;
        let siblings = nodes[1..root_ind].to_vec();
        let root = nodes[root_ind];
        Ok(MerkleProofCore::<Hash> {
            root,
            value,
            siblings,
            index: key.index,
        })
    }
}
pub trait KVQMerkleTreeModelCore<
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStore,
    KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>: KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
    fn set_node_kv(
        store: &mut S,
        kv: &KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    ) -> anyhow::Result<()> {
        KVA::set_ref(store, &kv.key, &kv.value)
    }
    fn set_node(
        store: &mut S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
        value: &Hash,
    ) -> anyhow::Result<()> {
        KVA::set_ref(store, key, value)
    }
    fn set_nodes_ref<'a>(
        store: &mut S,
        nodes: &[KVQPair<&'a KVQMerkleNodeKey<TABLE_TYPE>, &'a Hash>],
    ) -> anyhow::Result<()> {
        KVA::set_many_ref(store, nodes)
    }
    fn set_nodes<'a>(
        store: &mut S,
        nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>],
    ) -> anyhow::Result<()> {
        KVA::set_many(store, nodes)
    }
    fn set_leaf(
        store: &mut S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        let old_proof = Self::get_leaf(store, key)?;
        let mut current_value = value;
        let mut current_key = *key;

        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> =
            Vec::with_capacity((key.level as usize) + 1);

        let height = key.level as usize;
        if height > 0 {
            let new_key = current_key.parent();
            let index = current_key.index;
            updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
                key: current_key,
                value: current_value,
            });
            current_value = if index & 1 == 0 {
                if MARK_LEAVES {
                    Hasher::two_to_one_marked_leaf(&current_value, &old_proof.siblings[0])
                } else {
                    Hasher::two_to_one(&current_value, &old_proof.siblings[0])
                }
            } else {
                if MARK_LEAVES {
                    Hasher::two_to_one_marked_leaf(&old_proof.siblings[0], &current_value)
                } else {
                    Hasher::two_to_one(&old_proof.siblings[0], &current_value)
                }
            };
            current_key = new_key;
        }
        for i in 1..height {
            let new_key = current_key.parent();
            let index = current_key.index;
            updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
                key: current_key,
                value: current_value,
            });
            current_value = if index & 1 == 0 {
                Hasher::two_to_one(&current_value, &old_proof.siblings[i])
            } else {
                Hasher::two_to_one(&old_proof.siblings[i], &current_value)
            };
            current_key = new_key;
        }
        updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
            key: current_key,
            value: current_value,
        });

        Self::set_nodes(store, &updates)?;
        Ok(DeltaMerkleProofCore::<Hash> {
            old_root: old_proof.root,
            old_value: old_proof.value,

            new_root: current_value,
            new_value: value,

            siblings: old_proof.siblings,
            index: key.index,
        })
    }

    fn injest_merkle_proof(
        store: &mut S,
        tree_id: u8,
        primary_id: u64,
        secondary_id: u32,
        checkpoint_id: u64,
        merkle_proof: &MerkleProofCore<Hash>,
    ) -> anyhow::Result<()> {
        let base_leaf_key = KVQMerkleNodeKey::<TABLE_TYPE> {
            tree_id,
            primary_id,
            secondary_id,
            level: merkle_proof.siblings.len() as u8,
            index: merkle_proof.index,
            checkpoint_id,
        };


        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> =
            Vec::with_capacity(merkle_proof.siblings.len()*2+1);
        let mut k = base_leaf_key;
        let mut last_hash = merkle_proof.value;
            
        for sibling in merkle_proof.siblings.iter() {
            updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
                key: k,
                value: last_hash,
            });
            updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
                key: k.sibling(),
                value: *sibling,
            });

            last_hash = if k.index & 1 == 0 {
                if MARK_LEAVES {
                    Hasher::two_to_one_marked_leaf(&last_hash, &sibling)
                } else {
                    Hasher::two_to_one(&last_hash, &sibling)
                }
            } else {
                if MARK_LEAVES {
                    Hasher::two_to_one_marked_leaf(&sibling, &last_hash)
                } else {
                    Hasher::two_to_one(&sibling, &last_hash)
                }
            };
            k = k.parent();
        }
        
        updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
            key: k,
            value: last_hash,
        });
        Self::set_nodes(store, &updates)?;
        Ok(())
    }
}
