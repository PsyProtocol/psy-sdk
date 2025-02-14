use super::key::KVQMerkleNodeKey;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQBinaryStoreImmutable;
use kvq::traits::KVQBinaryStoreReader;
use kvq::traits::KVQPair;
use kvq::traits::KVQSerializable;
use kvq::traits::KVQStoreAdapter;
use kvq::traits::KVQStoreAdapterImmutable;
use kvq::traits::KVQStoreAdapterReader;
use qed_core::utils::math::ceil_div_usize;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::hash::merkle::spiderman::SpidermanUpdateProof;
use qed_crypto::hash::traits::hasher::MerkleZeroHasherWithMarkedLeaf;
use std::marker::PhantomData;

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


pub trait KVQMerkleTreeModelCoreImmutable<
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStoreImmutable,
    KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>: KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
    fn set_node_kv(
        store: &S,
        kv: &KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    ) -> anyhow::Result<()> {
        KVA::imm_set_ref(store, &kv.key, &kv.value)
    }
    fn set_node(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
        value: &Hash,
    ) -> anyhow::Result<()> {
        KVA::imm_set_ref(store, key, value)
    }
    fn set_nodes_ref<'a>(
        store: &S,
        nodes: &[KVQPair<&'a KVQMerkleNodeKey<TABLE_TYPE>, &'a Hash>],
    ) -> anyhow::Result<()> {
        KVA::imm_set_many_ref(store, nodes)
    }
    fn set_nodes<'a>(
        store: &S,
        nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>],
    ) -> anyhow::Result<()> {
        KVA::imm_set_many(store, nodes)
    }
    fn injest_merkle_proof(
        store: &S,
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
    /* 
    fn injest_merkle_proof_set_leaf(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
        siblings: &[Hash],
        new_checkpoint: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        let height = siblings.len() as u8;

        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> =
            Vec::with_capacity((height as usize));
        let mut k = key.clone();
            
        for sibling in siblings.iter() {
            updates.push(KVQPair::<KVQMerkleNodeKey<TABLE_TYPE>, Hash> {
                key: k.sibling(),
                value: *sibling,
            });
            k = k.parent();
        }
        
        Self::set_nodes(store, &updates)?;
        Self::set_leaf(store, &key.at_checkpoint(new_checkpoint), value)

    }*/
    fn rehash_from_node_to_level(
        store: &S,
        tree_height: usize,
        node: KVQMerkleNodeKey<TABLE_TYPE>,
        root_level: u8
    )-> anyhow::Result<()> {
        // TODO: optimize to get all nodes at once
            
        let mut current = node;
        let mut current_value = Self::get_node(store, tree_height, &node)?;

        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> = Vec::with_capacity((current.level-root_level) as usize);
        while current.level > root_level {
            let parent_key = current.parent();
            let sibling_value = Self::get_node(store, tree_height,&current.sibling())?;

            let parent_value = if current.index & 1 == 0 {
                if MARK_LEAVES && current.level == tree_height as u8 {
                    Hasher::two_to_one_marked_leaf(&current_value, &sibling_value)
                } else {
                    Hasher::two_to_one(&current_value, &sibling_value)
                }
            } else {
                if MARK_LEAVES && current.level == tree_height as u8 {
                    Hasher::two_to_one_marked_leaf(&sibling_value, &current_value)
                } else {
                    Hasher::two_to_one(&sibling_value, &current_value)
                }
            };
            updates.push(KVQPair { key: parent_key, value: parent_value });
            current = parent_key;
            current_value = parent_value;
        }
        Self::set_nodes(store, &updates)?;
        Ok(())
    }

    fn rehash_sub_tree(
        store: &S,
        tree_height: usize,
        sub_root_key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<()>{

        let sub_tree_height = tree_height - (sub_root_key.level as usize);

        if sub_tree_height == 0 {
            return Ok(());
        } else if sub_tree_height == 1 {
            let left_key = sub_root_key.left_child();
            let right_key = sub_root_key.right_child();
            let nodes = Self::get_nodes(store, tree_height, &[left_key, right_key])?;


            let sub_root_value = if MARK_LEAVES {
                Hasher::two_to_one_marked_leaf(&nodes[0], &nodes[1])
            } else {
                Hasher::two_to_one(&nodes[0], &nodes[1])
            };
            Self::set_node(store, sub_root_key, &sub_root_value)?;
            return Ok(());
        }

        
        let mut child_base_key = sub_root_key.first_leaf_child(tree_height as u8);

        let mut nodes_at_current_level = 1usize << (sub_tree_height - 1);

        let mut child_values = Vec::with_capacity(nodes_at_current_level);
        let mut child_keys = Vec::with_capacity(nodes_at_current_level*2);
        let mut node_updates = Vec::with_capacity((1usize<<sub_tree_height)-1);


        for i in 0..(nodes_at_current_level as u64) {
            let left_key = child_base_key.at_index(i*2+child_base_key.index);
            child_keys.push(left_key);
            child_keys.push(left_key.sibling());
        }
        for (i, x) in Self::get_nodes(store, tree_height, &child_keys)?.chunks_exact(2).enumerate() {
            let parent_value = if MARK_LEAVES {
                Hasher::two_to_one_marked_leaf(&x[0], &x[1])
            } else {
                Hasher::two_to_one(&x[0], &x[1])
            };

            let parent_key = child_base_key.at_index((i as u64)*2+child_base_key.index).parent();

            node_updates.push(
                KVQPair{
                    key: parent_key,
                    value: parent_value,
                }
            );
            child_values.push(parent_value);
        }

        nodes_at_current_level = nodes_at_current_level >> 1;
        child_base_key = child_base_key.parent();

        while child_base_key.level > sub_root_key.level {
            let mut parent_values = Vec::with_capacity(nodes_at_current_level as usize);
            for i in 0..nodes_at_current_level {
                let parent_key = child_base_key.parent().at_index(i as u64 + (child_base_key.index >> 1u64));

                let parent_value = if MARK_LEAVES {
                    Hasher::two_to_one_marked_leaf(&child_values[i * 2], &child_values[i * 2 + 1])
                }else{
                    Hasher::two_to_one(&child_values[i * 2], &child_values[i * 2 + 1])
                };
                node_updates.push(KVQPair { key: parent_key, value: parent_value });

                parent_values.push(parent_value);
            }
            nodes_at_current_level = nodes_at_current_level >> 1;
            child_base_key = child_base_key.parent();
            child_values = parent_values;
        }

        Self::set_nodes(store, &node_updates)?;

        Self::rehash_from_node_to_level(store, tree_height, child_base_key, 0)?;

        Ok(())

    }
    fn rehash_sub_tree_dmp(
        store: &S,
        tree_height: usize,
        sub_root_key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        let old_sub_tree_root = Self::get_node(store, tree_height, sub_root_key)?;
        let old_tree_root = Self::get_node(store, tree_height, &sub_root_key.root())?;


        Self::rehash_sub_tree(store, tree_height, sub_root_key)?;

        let mut keys = Vec::with_capacity(sub_root_key.level as usize + 2);
        keys.extend_from_slice(&sub_root_key.siblings());
        keys.push(sub_root_key.root());
        keys.push(*sub_root_key);

        let mut values = Self::get_nodes(store, tree_height, &keys)?;


    

        let new_sub_tree_root = values.pop().unwrap();
        let new_tree_root = values.pop().unwrap();

        Ok(DeltaMerkleProofCore {
            old_root: old_tree_root,
            old_value: old_sub_tree_root,
            new_root: new_tree_root,
            new_value: new_sub_tree_root,
            index: sub_root_key.index,
            siblings: values,
        })
    }



    fn append_leaves_spider_man(
        store: &S,
        tree_height: usize,
        first_empty_leaf_key: &KVQMerkleNodeKey<TABLE_TYPE>,
        sub_tree_height: u8,
        leaves: &[Hash],
    ) -> anyhow::Result<Vec<SpidermanUpdateProof<Hash>>> {
        let leaves_per_subtree = 1usize << sub_tree_height;
        let max_leaves = 1u64 << tree_height;
        let append_index = first_empty_leaf_key.index;
        if (append_index) + (leaves.len() as u64) > max_leaves {
            anyhow::bail!("tree cannot fit an additional {} leaves", leaves.len());
        }
        let cur_sub_tree_id = append_index / (leaves_per_subtree as u64);
        let cur_sub_tree_leaf_index =
            ((append_index as u64) & ((leaves_per_subtree as u64) - 1u64)) as u64;

        let subtree_count =
            ceil_div_usize((append_index as usize) + leaves.len(), leaves_per_subtree)
                - (cur_sub_tree_id as usize);

        let mut results = Vec::with_capacity(subtree_count);
        if subtree_count == 0 {
            return Ok(results);
        }

        let mut old_leaves = Vec::with_capacity(leaves_per_subtree);
        let mut new_leaves = Vec::with_capacity(leaves_per_subtree);

        let start_existing_index = cur_sub_tree_id * (leaves_per_subtree as u64);
        let start_added_index = start_existing_index + cur_sub_tree_leaf_index;

        let existing_leaf_keys = (start_existing_index..start_added_index).map(|i| first_empty_leaf_key.at_index(i)).collect::<Vec<_>>();
        let existing_leaf_values = Self::get_nodes(store, tree_height, &existing_leaf_keys)?;
        old_leaves.extend_from_slice(&existing_leaf_values);
        new_leaves.extend_from_slice(&existing_leaf_values);

        let first_tree_new_slots = leaves_per_subtree - new_leaves.len();

        new_leaves.extend_from_slice(&leaves[0..first_tree_new_slots.min(leaves.len())]);

        let first_tree_zero_hashes = leaves_per_subtree - new_leaves.len();

        let zero_hash = Hasher::get_zero_hash(0);
        let mut leaves_used = leaves_per_subtree - (cur_sub_tree_leaf_index as usize);
        for _ in 0..leaves_used {
            old_leaves.push(zero_hash);
        }
        for _ in 0..first_tree_zero_hashes {
            new_leaves.push(zero_hash);
        }

        let mut node_updates = Vec::with_capacity(leaves_used.min(leaves.len()));

        for (i, l) in leaves[0..leaves_used.min(leaves.len())].iter().enumerate() {
            node_updates.push(KVQPair {
                key: first_empty_leaf_key.at_index(i as u64 + start_added_index),
                value: *l,
            });
        }

        Self::set_nodes(store, &node_updates)?;
        let dmp = Self::rehash_sub_tree_dmp(store, tree_height,&first_empty_leaf_key.n_th_ancestor(sub_tree_height))?;

        results.push(SpidermanUpdateProof {
            top_line_proof: dmp,
            web_proof_old_leaves: old_leaves,
            web_proof_new_leaves: new_leaves,
        });

        if subtree_count > 2 {
            let ll = leaves_used;
            let old_leaves = (0..leaves_per_subtree)
                .map(|_| zero_hash)
                .collect::<Vec<_>>();

            for t in 0..(subtree_count - 3) {
                let base_ind = ll + t * leaves_per_subtree;
                let new_leaves = leaves[base_ind..(base_ind + leaves_per_subtree)].to_vec();
                let bb1 = (cur_sub_tree_id as usize + t + 1) * leaves_per_subtree;
                let mut node_updates = Vec::with_capacity(new_leaves.len());

                for (i, l) in new_leaves.iter().enumerate() {
                    node_updates.push(
                        KVQPair {
                            key: first_empty_leaf_key.at_index((bb1 + i) as u64),
                            value: *l,
                        }
                    );
                }

                Self::set_nodes(store, &node_updates)?;

                results.push(SpidermanUpdateProof {
                    top_line_proof: Self::rehash_sub_tree_dmp(store, sub_tree_height as usize, &first_empty_leaf_key.at_index(bb1 as u64).n_th_ancestor(sub_tree_height))?,
                    web_proof_old_leaves: old_leaves.clone(),
                    web_proof_new_leaves: new_leaves,
                });
            }

            // OPT: don't waste the old_leaves.clone()

            let t = (subtree_count as usize)-3;
            let base_ind = ll + t * leaves_per_subtree;
            let new_leaves = leaves[base_ind..(base_ind + leaves_per_subtree)].to_vec();
            let bb1 = (cur_sub_tree_id as usize + t + 1) * leaves_per_subtree;

            let mut node_updates = Vec::with_capacity(new_leaves.len());

            for (i, l) in new_leaves.iter().enumerate() {
                node_updates.push(
                    KVQPair {
                        key: first_empty_leaf_key.at_index((bb1 + i) as u64),
                        value: *l,
                    }
                );
            }

            Self::set_nodes(store, &node_updates)?;

            results.push(SpidermanUpdateProof {
                top_line_proof: Self::rehash_sub_tree_dmp(store, sub_tree_height as usize, &first_empty_leaf_key.at_index(bb1 as u64).n_th_ancestor(sub_tree_height))?,
                web_proof_old_leaves: old_leaves.clone(),
                web_proof_new_leaves: new_leaves,
            });

            leaves_used += (subtree_count - 2) * leaves_per_subtree;
        }

        if subtree_count > 1 {
            let zero_hash = Hasher::get_zero_hash(0);

            let old_leaves = (0..leaves_per_subtree)
                .map(|_| zero_hash)
                .collect::<Vec<_>>();

            let mut new_leaves = Vec::with_capacity(leaves_per_subtree);
            new_leaves.extend_from_slice(&leaves[leaves_used..]);
            let remaining = leaves_per_subtree - new_leaves.len();
            for _ in 0..remaining {
                new_leaves.push(zero_hash);
            }
            let bb1 = ((cur_sub_tree_id as usize + subtree_count - 1) * leaves_per_subtree) as u64;

            let mut node_updates = Vec::with_capacity(leaves.len()-leaves_used);

            for (i, l) in leaves[leaves_used..].iter().enumerate() {
                node_updates.push(
                    KVQPair {
                        key: first_empty_leaf_key.at_index((bb1 + i as u64) as u64),
                        value: *l,
                    }
                );
            }

            Self::set_nodes(store, &node_updates)?;

            results.push(SpidermanUpdateProof {
                top_line_proof: Self::rehash_sub_tree_dmp(store, sub_tree_height as usize, &first_empty_leaf_key.at_index(bb1 as u64).n_th_ancestor(sub_tree_height))?,
                web_proof_old_leaves: old_leaves.clone(),
                web_proof_new_leaves: new_leaves,
            });
        }

        Ok(results)
    }
    fn set_leaf(
        store: &S,
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
}
pub trait KVQFixedConfigMerkleTreeModelReaderCore<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const PRIMARY_ID: u64,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStoreReader,
    KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>: KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
    fn new_node_key_fc(checkpoint_id: u64, level: u8, index: u64) -> KVQMerkleNodeKey<TABLE_TYPE> {
        KVQMerkleNodeKey::<TABLE_TYPE> {
            tree_id: TREE_ID,
            primary_id: PRIMARY_ID,
            secondary_id: SECONDARY_ID,
            level,
            index,
            checkpoint_id,
        }
    }
    fn new_leaf_key_fc(checkpoint_id: u64, index: u64) -> KVQMerkleNodeKey<TABLE_TYPE> {
        KVQMerkleNodeKey::<TABLE_TYPE> {
            tree_id: TREE_ID,
            primary_id: PRIMARY_ID,
            secondary_id: SECONDARY_ID,
            level: TREE_HEIGHT,
            index,
            checkpoint_id,
        }
    }
    fn get_sub_tree_proof_fc(
        store: &S,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        Self::get_sub_tree_proof(
            store, 
            TREE_HEIGHT as usize,
            root_level,
            &Self::new_node_key_fc(checkpoint_id, leaf_level, leaf_index)
        )
    }
    fn get_leaf_fc(
        store: &S,
        checkpoint_id: u64,
        index: u64,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        Self::get_leaf(store, &Self::new_leaf_key_fc(checkpoint_id, index))
    }
    fn get_leaf_value_fc(store: &S, checkpoint_id: u64, index: u64) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_leaf_key_fc(checkpoint_id, index),
        )
    }
    fn get_leaf_values_fc(
        store: &S,
        checkpoint_id: u64,
        indexes: &[u64],
    ) -> anyhow::Result<Vec<Hash>> {
        let leaf_keys = indexes
            .iter()
            .map(|index| Self::new_leaf_key_fc(checkpoint_id, *index))
            .collect::<Vec<_>>();
        Self::get_nodes(store, TREE_HEIGHT as usize, &leaf_keys)
    }
    fn get_node_value_fc(
        store: &S,
        checkpoint_id: u64,
        level: u8,
        index: u64,
    ) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_node_key_fc(checkpoint_id, level, index),
        )
    }
    fn get_root_fc(store: &S, checkpoint_id: u64) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_node_key_fc(checkpoint_id, 0, 0),
        )
    }
}


pub trait KVQSemiFixedConfigMerkleTreeModelReaderCore<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStoreReader,
    KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>: KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
    fn new_node_key_sfc(checkpoint_id: u64, primary_id: u64, level: u8, index: u64) -> KVQMerkleNodeKey<TABLE_TYPE> {
        KVQMerkleNodeKey::<TABLE_TYPE> {
            tree_id: TREE_ID,
            primary_id: primary_id,
            secondary_id: SECONDARY_ID,
            level,
            index,
            checkpoint_id,
        }
    }
    fn new_leaf_key_sfc(checkpoint_id: u64, primary_id: u64, index: u64) -> KVQMerkleNodeKey<TABLE_TYPE> {
        KVQMerkleNodeKey::<TABLE_TYPE> {
            tree_id: TREE_ID,
            primary_id: primary_id,
            secondary_id: SECONDARY_ID,
            level: TREE_HEIGHT,
            index,
            checkpoint_id,
        }
    }
    fn get_leaf_sfc(
        store: &S,
        checkpoint_id: u64,
        primary_id: u64,
        index: u64,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        Self::get_leaf(store, &Self::new_leaf_key_sfc(checkpoint_id, primary_id, index))
    }
    fn get_leaf_value_fc(store: &S, checkpoint_id: u64, primary_id: u64, index: u64) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_leaf_key_sfc(checkpoint_id, primary_id, index),
        )
    }
    fn get_leaf_values_fc(
        store: &S,
        checkpoint_id: u64,
        primary_id: u64,
        indexes: &[u64],
    ) -> anyhow::Result<Vec<Hash>> {
        let leaf_keys = indexes
            .iter()
            .map(|index| Self::new_leaf_key_sfc(checkpoint_id, primary_id, *index))
            .collect::<Vec<_>>();
        Self::get_nodes(store, TREE_HEIGHT as usize, &leaf_keys)
    }
    fn get_node_value_fc(
        store: &S,
        checkpoint_id: u64,
        primary_id: u64,
        level: u8,
        index: u64,
    ) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_node_key_sfc(checkpoint_id, primary_id, level, index),
        )
    }
    fn get_root_fc(store: &S, checkpoint_id: u64, primary_id: u64) -> anyhow::Result<Hash> {
        Self::get_node(
            store,
            TREE_HEIGHT as usize,
            &Self::new_node_key_sfc(checkpoint_id, primary_id, 0, 0),
        )
    }
}
pub trait KVQSemiFixedConfigMerkleTreeModelCore<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStore,
    KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>:
    KVQMerkleTreeModelCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    + KVQSemiFixedConfigMerkleTreeModelReaderCore<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
    fn set_leaf_sfc(
        store: &mut S,
        checkpoint_id: u64,
        primary_id: u64,
        index: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::set_leaf(store, &Self::new_leaf_key_sfc(checkpoint_id, primary_id, index), value)
    }

    fn injest_merkle_proof_sfc(store: &mut S, 
        primary_id: u64, checkpoint_id: u64, merkle_proof: &MerkleProofCore<Hash>) -> anyhow::Result<()> {
        Self::injest_merkle_proof(store, TREE_ID, primary_id, SECONDARY_ID, checkpoint_id, merkle_proof)
    }
    fn injest_merkle_proof_set_leaf_sfc(
        store: &mut S, 
        primary_id: u64,
        old_checkpoint_id: u64, 
        merkle_proof: &MerkleProofCore<Hash>, 
        new_checkpoint_id: u64,
        new_value: Hash
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::injest_merkle_proof_sfc(store, primary_id, old_checkpoint_id, merkle_proof)?;
        Self::set_leaf(store, &Self::new_leaf_key_sfc(new_checkpoint_id, primary_id, merkle_proof.index), new_value)
    }
}
pub trait KVQSemiFixedConfigMerkleTreeModelCoreImmutable<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStoreImmutable,
    KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>:
    KVQMerkleTreeModelCoreImmutable<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    + KVQSemiFixedConfigMerkleTreeModelReaderCore<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{

    fn injest_merkle_proof_sfc_imm(store: &S, 
        primary_id: u64, checkpoint_id: u64, merkle_proof: &MerkleProofCore<Hash>) -> anyhow::Result<()> {
        Self::injest_merkle_proof(store, TREE_ID, primary_id, SECONDARY_ID, checkpoint_id, merkle_proof)
    }
    fn injest_merkle_proof_set_leaf_sfc_imm(
        store: &S, 
        primary_id: u64,
        old_checkpoint_id: u64, 
        merkle_proof: &MerkleProofCore<Hash>, 
        new_checkpoint_id: u64,
        new_value: Hash
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::injest_merkle_proof_sfc_imm(store, primary_id, old_checkpoint_id, merkle_proof)?;
        Self::set_leaf(store, &Self::new_leaf_key_sfc(new_checkpoint_id, primary_id, merkle_proof.index), new_value)
    }
    fn set_leaf_sfc_imm(
        store: &S,
        checkpoint_id: u64,
        primary_id: u64,
        index: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::set_leaf(store, &Self::new_leaf_key_sfc(checkpoint_id, primary_id, index), value)
    }
}
pub trait KVQFixedConfigMerkleTreeModelCore<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const PRIMARY_ID: u64,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S: KVQBinaryStore,
    KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>:
    KVQMerkleTreeModelCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    + KVQFixedConfigMerkleTreeModelReaderCore<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{

    fn injest_merkle_proof_fc(store: &mut S, checkpoint_id: u64, merkle_proof: &MerkleProofCore<Hash>) -> anyhow::Result<()> {
        Self::injest_merkle_proof(store, TREE_ID, PRIMARY_ID, SECONDARY_ID, checkpoint_id, merkle_proof)
    }
    fn injest_merkle_proof_set_leaf_fc(
        store: &mut S, 
        old_checkpoint_id: u64, 
        merkle_proof: &MerkleProofCore<Hash>, 
        new_checkpoint_id: u64,
        new_value: Hash
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::injest_merkle_proof_fc(store, old_checkpoint_id, merkle_proof)?;
        Self::set_leaf(store, &Self::new_leaf_key_fc(new_checkpoint_id, merkle_proof.index), new_value)
    }
    fn set_leaf_fc(
        store: &mut S,
        checkpoint_id: u64,
        index: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::set_leaf(store, &Self::new_leaf_key_fc(checkpoint_id, index), value)
    }
}
pub trait KVQFixedConfigMerkleTreeModelCoreImmutable<
const TREE_ID: u8,
const TREE_HEIGHT: u8,
const PRIMARY_ID: u64,
const SECONDARY_ID: u32,
const TABLE_TYPE: u16,
const MARK_LEAVES: bool,
S: KVQBinaryStoreImmutable,
KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
Hash: Copy + PartialEq + KVQSerializable,
Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>:
KVQMerkleTreeModelCoreImmutable<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
+ KVQFixedConfigMerkleTreeModelReaderCore<
    TREE_ID,
    TREE_HEIGHT,
    PRIMARY_ID,
    SECONDARY_ID,
    TABLE_TYPE,
    MARK_LEAVES,
    S,
    KVA,
    Hash,
    Hasher,
>
{
    fn injest_merkle_proof_fc_imm(store: &S, checkpoint_id: u64, merkle_proof: &MerkleProofCore<Hash>) -> anyhow::Result<()> {
        Self::injest_merkle_proof(store, TREE_ID, PRIMARY_ID, SECONDARY_ID, checkpoint_id, merkle_proof)
    }
    fn injest_merkle_proof_set_leaf_fc_imm(
        store: &S, 
        old_checkpoint_id: u64, 
        merkle_proof: &MerkleProofCore<Hash>, 
        new_checkpoint_id: u64,
        new_value: Hash
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::injest_merkle_proof_fc_imm(store, old_checkpoint_id, merkle_proof)?;
        Self::set_leaf(store, &Self::new_leaf_key_fc(new_checkpoint_id, merkle_proof.index), new_value)
    }
    fn set_leaf_fc(
        store: &S,
        checkpoint_id: u64,
        index: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::set_leaf(store, &Self::new_leaf_key_fc(checkpoint_id, index), value)
    }
    fn set_leaf_fc_imm(
        store: &S,
        checkpoint_id: u64,
        index: u64,
        value: Hash,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        Self::set_leaf(store, &Self::new_leaf_key_fc(checkpoint_id, index), value)
    }
}

pub struct KVQMerkleTreeModel<
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S,
    KVA,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
> {
    _hasher: PhantomData<Hasher>,
    _hash: PhantomData<Hash>,
    _s: PhantomData<S>,
    _kva: PhantomData<KVA>,
}
impl<
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreReader,
        Hash: PartialEq + KVQSerializable + Copy,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
        KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    > KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQMerkleTreeModel<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
}
impl<
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStore,
        Hash: PartialEq + KVQSerializable + Copy,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
        KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    > KVQMerkleTreeModelCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQMerkleTreeModel<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
}
impl<
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreImmutable,
        Hash: PartialEq + KVQSerializable + Copy,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
        KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    > KVQMerkleTreeModelCoreImmutable<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQMerkleTreeModel<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
}

pub struct KVQFixedConfigMerkleTreeModel<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const PRIMARY_ID: u64,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S,
    KVA,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
> {
    _hasher: PhantomData<Hasher>,
    _hash: PhantomData<Hash>,
    _s: PhantomData<S>,
    _kva: PhantomData<KVA>,
}

impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreReader,
        KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreImmutable,
        KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelCoreImmutable<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreReader,
        KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQFixedConfigMerkleTreeModelReaderCore<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQFixedConfigMerkleTreeModelCore<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const PRIMARY_ID: u64,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreImmutable,
        KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQFixedConfigMerkleTreeModelCoreImmutable<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        PRIMARY_ID,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}

pub struct KVQSemiFixedConfigMerkleTreeModel<
    const TREE_ID: u8,
    const TREE_HEIGHT: u8,
    const SECONDARY_ID: u32,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S,
    KVA,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
> {
    _hasher: PhantomData<Hasher>,
    _hash: PhantomData<Hash>,
    _s: PhantomData<S>,
    _kva: PhantomData<KVA>,
}


impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreReader,
        KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreImmutable,
        KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    > KVQMerkleTreeModelCoreImmutable<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreReader,
        KVA: KVQStoreAdapterReader<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQSemiFixedConfigMerkleTreeModelReaderCore<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStore,
        KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQSemiFixedConfigMerkleTreeModelCore<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}
impl<
        const TREE_ID: u8,
        const TREE_HEIGHT: u8,
        const SECONDARY_ID: u32,
        const TABLE_TYPE: u16,
        const MARK_LEAVES: bool,
        S: KVQBinaryStoreImmutable,
        KVA: KVQStoreAdapterImmutable<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        Hash: Copy + PartialEq + KVQSerializable,
        Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    >
    KVQSemiFixedConfigMerkleTreeModelCoreImmutable<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
    for KVQSemiFixedConfigMerkleTreeModel<
        TREE_ID,
        TREE_HEIGHT,
        SECONDARY_ID,
        TABLE_TYPE,
        MARK_LEAVES,
        S,
        KVA,
        Hash,
        Hasher,
    >
{
}