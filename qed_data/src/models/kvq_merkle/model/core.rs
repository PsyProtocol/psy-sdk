
use super::super::key::KVQMerkleNodeKey;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQPair;
use kvq::traits::KVQSerializable;
use kvq::traits::KVQStoreAdapter;
use kvq::traits::KVQStoreAdapterReader;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::hash::merkle::spiderman::SpidermanUpdateProof;
use qed_crypto::hash::traits::hasher::MerkleZeroHasherWithMarkedLeaf;
use qed_crypto::hash::merkle::utils::sub_tree_nca::UpdateNCAProofsWithDependencies;
use qed_crypto::hash::merkle::utils::sub_tree_nca::UpdateNearestCommonAncestorProof;

pub const CHECKPOINT_ID_FUZZY_SIZE: usize = 8;

pub trait KVQMerkleTreeModelReaderCore<
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
    S,
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
    S,
    KVA: KVQStoreAdapter<S, KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    Hash: Copy + PartialEq + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
>: KVQMerkleTreeModelReaderCore<TABLE_TYPE, MARK_LEAVES, S, KVA, Hash, Hasher>
{
    fn set_node_kv(
        store: &S,
        kv: &KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
    ) -> anyhow::Result<()> {
        KVA::set_ref(store, &kv.key, &kv.value)
    }
    fn set_node(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
        value: &Hash,
    ) -> anyhow::Result<()> {
        KVA::set_ref(store, key, value)
    }
    fn set_nodes_ref<'a>(
        store: &S,
        nodes: &[KVQPair<&'a KVQMerkleNodeKey<TABLE_TYPE>, &'a Hash>],
    ) -> anyhow::Result<()> {
        KVA::set_many_ref(store, nodes)
    }
    fn set_nodes<'a>(
        store: &S,
        nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>],
    ) -> anyhow::Result<()> {
        KVA::set_many(store, nodes)
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

    fn smart_injest_nca_split_kv(
        store: &S,
        tree_height: usize,
        a: KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        b: KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>,
        updates: &mut Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>,
    ) -> anyhow::Result<(KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>, UpdateNearestCommonAncestorProof<Hash>)> {
        if a.key.is_direct_path_related(&b.key) {
            anyhow::bail!("cannot update two keys on the same path");
        }
        if a.key > b.key {
            return Self::smart_injest_nca_split_kv(store, tree_height, b, a, updates);
        }

        if a.key.is_sibling_for(&b.key) {
            let parent = a.key.parent();
            let a_b_parent = Self::get_nodes(store, tree_height,&[a.key, b.key, parent])?;

            let new_value = if MARK_LEAVES && a.key.level as usize == tree_height {
                Hasher::two_to_one_marked_leaf_swap(a.key.is_right_child(), &a.value, &b.value)
            }else{
                Hasher::two_to_one_swap(a.key.is_right_child(), &a.value, &b.value)
            };

            let r = UpdateNearestCommonAncestorProof {
                old_nearest_common_ancestor_value: a_b_parent[2],
                new_nearest_common_ancestor_value: new_value,
                child_a: DeltaMerkleProofCore::single_value(a.key.index,a_b_parent[0], a.value),
                child_b: DeltaMerkleProofCore::single_value(b.key.index,a_b_parent[1], b.value),
                nearest_common_ancestor_level: parent.level,
                nearest_common_ancestor_index: parent.index,
                level_a: a.key.level,
                level_b: b.key.level,
            };

            updates.reserve(2);

            updates.push(a);
            updates.push(b);

            return Ok((KVQPair {
                key: parent,
                value: new_value,
            }, r));
        }

        let nca = a.key.find_nearest_common_ancestor(&b.key);

        let a_root = nca.left_child();
        let b_root = nca.right_child();

        let child_a = Self::set_rehash_from_node_to_level_dmp_with_updates(store, tree_height, a.key, a.value, a_root.level, updates)?;
        let child_b = Self::set_rehash_from_node_to_level_dmp_with_updates(store, tree_height, b.key, b.value, b_root.level, updates)?;

        let old_nearest_common_ancestor_value = if MARK_LEAVES && a_root.level as usize == tree_height {
            Hasher::two_to_one_marked_leaf(&child_a.old_root, &child_b.old_root)
        }else{
            Hasher::two_to_one(&child_a.old_root, &child_b.old_root)
        };

        let new_nearest_common_ancestor_value = if MARK_LEAVES && a_root.level as usize == tree_height {
            Hasher::two_to_one_marked_leaf(&child_a.new_root, &child_b.new_root)
        }else{
            Hasher::two_to_one(&child_a.new_root, &child_b.new_root)
        };

        let update_nca_proof = UpdateNearestCommonAncestorProof {
            old_nearest_common_ancestor_value,
            new_nearest_common_ancestor_value,
            child_a,
            child_b,
            nearest_common_ancestor_level: nca.level,
            nearest_common_ancestor_index: nca.index,
            level_a: a.key.level,
            level_b: b.key.level,
        };

        Ok((
            KVQPair {
                key: nca,
                value: new_nearest_common_ancestor_value,
            },
            update_nca_proof
        ))
    }

    fn smart_injest_nca_at_height_dmp(
        store: &S,
        tree_height: usize,
        root_level: u8,
        nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>],
    ) -> anyhow::Result<Vec<DeltaMerkleProofCore<Hash>>> {
        let mut dmps = Vec::with_capacity(nodes.len());
        for n in nodes.iter() {
            let mut updates = Vec::new();
            dmps.push(Self::set_rehash_from_node_to_level_dmp_with_updates(
                store,
                tree_height,
                n.key,
                n.value,
                root_level,
                &mut updates
            )?);
            Self::set_nodes(store, &updates)?;
        }
        Ok(dmps)
    }

    fn smart_injest_nca(
        store: &S,
        tree_height: usize,
        root_level: u8,
        mut nodes: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<Hash>> {
        if nodes.len() == 1 {
            let mut updates = Vec::new();
            let dmp_a = Self::set_rehash_from_node_to_level_dmp_with_updates(
                store,
                tree_height,
                nodes[0].key,
                nodes[0].value,
                root_level,
                &mut updates
            )?;
            let root_key = nodes[0].key;
            let root_value = nodes[0].value;

            let link_proof = Self::set_rehash_from_node_to_level_dmp_with_updates(
                store,
                tree_height,
                root_key,
                root_value,
                root_level.min(root_key.level),
                &mut updates,
            )?;

            Self::set_nodes(store, &updates)?;

            return Ok(UpdateNCAProofsWithDependencies {
                nca_proofs: vec![UpdateNearestCommonAncestorProof{
                    old_nearest_common_ancestor_value: dmp_a.old_value,
                    new_nearest_common_ancestor_value: dmp_a.new_value,
                    child_a: DeltaMerkleProofCore::single_value(root_key.index, dmp_a.old_value, dmp_a.new_value),
                    child_b: DeltaMerkleProofCore::single_value(root_key.index, dmp_a.old_value, dmp_a.new_value),
                    nearest_common_ancestor_level: root_key.level,
                    nearest_common_ancestor_index: root_key.index,
                    level_a: root_key.level,
                    level_b: root_key.level,
                }],
                dependencies: vec![(-1, -1)],
                root_proof_index: 0,
                nearest_common_ancestor_level: root_key.level,
                nearest_common_ancestor_index: root_key.index,
                link_level: root_level.min(root_key.level),
                link_index: if root_level < root_key.level {
                    root_key.parent_at_level(root_level).index
                } else {
                    root_key.index
                },
                link_proof: link_proof,
            })
        }

        assert!(nodes.len() > 0, "can only call smart injest nca with multiple nodes");

        nodes.sort_by(|a,b| a.key.cmp(&b.key));
        let straggler = if nodes.len()&1 == 1 {
            Some(nodes.pop().unwrap())
        } else {
            None
        };
        let nodes = nodes;
        let full_nodes_len = nodes.len();

        let mut nca_proofs: Vec<UpdateNearestCommonAncestorProof<Hash>> = Vec::with_capacity(nodes.len());
        let mut dependencies: Vec<(i64, i64)> = Vec::with_capacity(nodes.len());
        let mut updates = Vec::with_capacity(nodes.len()*tree_height);
        let first_rung_len = full_nodes_len/2;

        let mut current_inds = Vec::with_capacity(first_rung_len);
        let mut current_nodes = Vec::with_capacity(first_rung_len);

        for i in 0..first_rung_len {
            let (node, proof) = Self::smart_injest_nca_split_kv(
                store,
                tree_height,
                nodes[i*2],
                nodes[i*2+1],
                &mut updates
            )?;
            current_nodes.push(node);
            current_inds.push(i);
            nca_proofs.push(proof);
            dependencies.push((-1, -1));
        }

        let mut next_nca_proof_index = nca_proofs.len();

        while current_nodes.len() > 1 {
            let current_nodes_len = current_nodes.len();
            let even_pairs = current_nodes_len/2;
            let new_nodes_len = even_pairs+(current_nodes_len&1);
            let has_odd = current_nodes_len&1 == 1;

            let mut new_nodes = Vec::with_capacity(new_nodes_len);
            let mut new_inds = Vec::with_capacity(new_nodes_len);

            for i in 0..even_pairs {
                let (node, proof) = Self::smart_injest_nca_split_kv(
                    store,
                    tree_height,
                    current_nodes[i*2],
                    current_nodes[i*2+1],
                    &mut updates
                )?;
                new_nodes.push(node);
                new_inds.push(next_nca_proof_index);
                dependencies.push((current_inds[i*2] as i64, current_inds[i*2+1] as i64));
                nca_proofs.push(proof);
                next_nca_proof_index += 1;
            }
            if has_odd {
                new_nodes.push(*current_nodes.last().unwrap());
                new_inds.push(*current_inds.last().unwrap());
            }
            current_nodes = new_nodes;
            current_inds = new_inds;
        }

        match straggler {
            Some(x) => {
                let (node, proof) = Self::smart_injest_nca_split_kv(
                    store,
                    tree_height,
                    current_nodes[0],
                    x,
                    &mut updates
                )?;
                dependencies.push((current_inds[0] as i64, -1));
                nca_proofs.push(proof);

                let link_proof = Self::set_rehash_from_node_to_level_dmp_with_updates(
                    store,
                    tree_height,
                    node.key,
                    node.value,
                    root_level.min(node.key.level),
                    &mut updates,
                )?;
                updates.push(node);

                Self::set_nodes(store, &updates)?;
                let root_proof_index = nca_proofs.len()-1;

                Ok(UpdateNCAProofsWithDependencies {
                    nca_proofs,
                    dependencies,
                    nearest_common_ancestor_level: node.key.level,
                    nearest_common_ancestor_index: node.key.index,
                    root_proof_index,
                    link_level: root_level.min(node.key.level),
                    link_index: if root_level < node.key.level {
                        node.key.parent_at_level(root_level).index
                    } else {
                        node.key.index
                    },
                    link_proof: link_proof,
                })
            },
            None => {
                let node = current_nodes[0];
                let link_proof = Self::set_rehash_from_node_to_level_dmp_with_updates(
                    store,
                    tree_height,
                    node.key,
                    node.value,
                    root_level.min(node.key.level),
                    &mut updates,
                )?;
                updates.push(node);

                Self::set_nodes(store, &updates)?;
                let root_proof_index = current_inds[0];

                Ok(UpdateNCAProofsWithDependencies {
                    nca_proofs,
                    dependencies,
                    nearest_common_ancestor_level: node.key.level,
                    nearest_common_ancestor_index: node.key.index,
                    root_proof_index,
                    link_level: root_level.min(node.key.level),
                    link_index: if root_level < node.key.level {
                        node.key.parent_at_level(root_level).index
                    } else {
                        node.key.index
                    },
                    link_proof: link_proof,
                })
            }
        }
    }

    fn rehash_from_node_to_level(
        store: &S,
        tree_height: usize,
        node: KVQMerkleNodeKey<TABLE_TYPE>,
        root_level: u8
    ) -> anyhow::Result<()> {
        // TODO: optimize to get all nodes at once

        let mut current = node;
        let mut current_value = Self::get_node(store, tree_height, &node)?;

        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> = Vec::with_capacity((current.level-root_level) as usize);
        while current.level > root_level {
            let parent_key = current.parent();
            let sibling_value = Self::get_node(store, tree_height, &current.sibling())?;

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

    fn set_rehash_from_node_to_level_dmp_with_updates(
        store: &S,
        tree_height: usize,
        node: KVQMerkleNodeKey<TABLE_TYPE>,
        new_value: Hash,
        root_level: u8,
        updates: &mut Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>,
    ) -> anyhow::Result<DeltaMerkleProofCore<Hash>> {
        let root_level = root_level.min(node.level);
        if root_level == node.level {
            let old_value = Self::get_node(store, tree_height, &node)?;
            Self::set_node(store, &node, &new_value)?;
            return Ok(DeltaMerkleProofCore {
                new_value,
                old_value,
                old_root: old_value,
                new_root: new_value,
                index: node.index,
                siblings: Vec::new(),
            });
        }
        let sub_height = (node.level-root_level) as usize;
        let mut siblings_old_value_root_keys = Vec::with_capacity(sub_height + 2);
        siblings_old_value_root_keys.extend_from_slice(&node.siblings_to_level(root_level));
        siblings_old_value_root_keys.push(node);
        siblings_old_value_root_keys.push(node.parent_at_level(root_level));
        let mut siblings_and_old = Self::get_nodes(store, tree_height, &siblings_old_value_root_keys)?;

        let old_sub_root = siblings_and_old.pop().unwrap();
        let old_value = siblings_and_old.pop().unwrap();
        let siblings = siblings_and_old;

        let mut current_value = new_value;
        let mut current = node;
        updates.reserve(sub_height+1);
        updates.push(KVQPair { key: node, value: new_value });
        for sibling_value in siblings.iter() {
            let parent_key = current.parent();

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
        Ok(DeltaMerkleProofCore {
            old_root: old_sub_root,
            old_value,
            new_root: current_value,
            new_value,
            index: node.index,
            siblings,
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
            qed_core::utils::math::ceil_div_usize((append_index as usize) + leaves.len(), leaves_per_subtree)
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

        // Fill old_leaves with zeros for the remaining slots
        let old_leaves_zeros = leaves_per_subtree - old_leaves.len();
        for _ in 0..old_leaves_zeros {
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
        let dmp = Self::rehash_sub_tree_dmp(store, tree_height, &first_empty_leaf_key.n_th_ancestor(sub_tree_height))?;

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
                    top_line_proof: Self::rehash_sub_tree_dmp(store, tree_height, &first_empty_leaf_key.at_index(bb1 as u64).n_th_ancestor(sub_tree_height))?,
                    web_proof_old_leaves: old_leaves.clone(),
                    web_proof_new_leaves: new_leaves,
                });
            }

            // OPT: don't waste the old_leaves.clone()

            // Process the second-to-last subtree
            let ll = leaves_used;
            let old_leaves = (0..leaves_per_subtree)
                .map(|_| zero_hash)
                .collect::<Vec<_>>();

            let t = subtree_count - 3;
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
                top_line_proof: Self::rehash_sub_tree_dmp(store, tree_height, &first_empty_leaf_key.at_index(bb1 as u64).n_th_ancestor(sub_tree_height))?,
                web_proof_old_leaves: old_leaves,
                web_proof_new_leaves: new_leaves,
            });

            leaves_used += (subtree_count - 2) * leaves_per_subtree;
        }

        if subtree_count > 1 {
            let old_leaves = (0..leaves_per_subtree)
                .map(|_| zero_hash)
                .collect::<Vec<_>>();

            let mut new_leaves = Vec::with_capacity(leaves_per_subtree);
            new_leaves.extend_from_slice(&leaves[leaves_used..]);

            let zeros_to_add = leaves_per_subtree - new_leaves.len();
            for _ in 0..zeros_to_add {
                new_leaves.push(zero_hash);
            }

            let bb1 = ((cur_sub_tree_id as usize + subtree_count - 1) * leaves_per_subtree) as u64;
            let mut node_updates = Vec::with_capacity(leaves.len() - leaves_used);

            for (i, l) in leaves[leaves_used..].iter().enumerate() {
                node_updates.push(KVQPair {
                    key: first_empty_leaf_key.at_index(bb1 + i as u64),
                    value: *l,
                });
            }

            Self::set_nodes(store, &node_updates)?;

            results.push(SpidermanUpdateProof {
                top_line_proof: Self::rehash_sub_tree_dmp(store, tree_height, &first_empty_leaf_key.at_index(bb1).n_th_ancestor(sub_tree_height))?,
                web_proof_old_leaves: old_leaves,
                web_proof_new_leaves: new_leaves,
            });
        }

        Ok(results)
    }

    fn rehash_sub_tree(
        store: &S,
        tree_height: usize,
        sub_root_key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<()> {
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
                } else {
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

    fn rehash_sub_tree_top(
        store: &S,
        tree_height: usize,
        first_node_at_level: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<()> {
        if first_node_at_level.level == 0 {
            return Ok(());
        }
        let current_leaf_keys_count = 1usize<<(first_node_at_level.level as usize);
        let current_leaf_keys = (0..current_leaf_keys_count).map(|x| {
            first_node_at_level.at_index(x as u64)
        }).collect::<Vec<_>>();

        let mut updates: Vec<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>> = Vec::with_capacity(current_leaf_keys_count-1);

        let mut current_leaves = Self::get_nodes(store, tree_height, &current_leaf_keys)?;

        let mut child_level = first_node_at_level.level;
        while child_level > 0 {
            let new_leaf_keys_count = 1usize<<(child_level as usize - 1);
            let mut new_leaves = Vec::with_capacity(new_leaf_keys_count);

            for i in 0..new_leaf_keys_count {
                let value = if MARK_LEAVES && child_level == (tree_height as u8) {
                    Hasher::two_to_one_marked_leaf(&current_leaves[i*2], &current_leaves[i*2+1])
                } else {
                    Hasher::two_to_one(&current_leaves[i*2], &current_leaves[i*2+1])
                };
                new_leaves.push(value);
                updates.push(KVQPair {
                    key: first_node_at_level.n_th_ancestor(first_node_at_level.level - child_level + 1).at_index(i as u64),
                    value,
                });
            }
            current_leaves = new_leaves;
            child_level -= 1;
        }

        Self::set_nodes(store, &updates)?;
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
}
