use super::key::KVQMerkleNodeKey;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQBinaryStoreImmutable;
use kvq::traits::KVQBinaryStoreReader;
use kvq::traits::KVQPair;
use kvq::traits::KVQSerializable;
use kvq::traits::KVQStoreAdapter;
use kvq::traits::KVQStoreAdapterImmutable;
use kvq::traits::KVQStoreAdapterReader;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_crypto::hash::merkle::core::MerkleProofCore;
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