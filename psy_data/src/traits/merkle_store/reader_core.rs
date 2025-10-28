use kvq::traits::{KVQPair, KVQSerializable};
use psy_crypto::hash::{merkle::core::MerkleProofCore, traits::hasher::MerkleZeroHasherWithMarkedLeaf};

use crate::models::kvq_merkle::{key::KVQMerkleNodeKey, model::KVQMerkleTreeModelReaderCore};

use super::{MerkleNodeStoreImmutableAsync, MerkleNodeStoreReaderImmutableAsync};


use async_trait::async_trait;


#[async_trait]
pub trait QEDMerkleTreeModelReaderCoreAsync<
    S: MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> + Send + Sync,
    Hash: Copy + Clone + Send + Sync + KVQSerializable,
    Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
    const TABLE_TYPE: u16,
    const MARK_LEAVES: bool,
>
{
    async fn get_node_exact(store: &S, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash> {
        let r = store.get_node_if_exists(key).await?;
        match r {
            Some(x) => {
                if x.key == key.to_owned() {
                    return Ok(x.value);
                }else{
                    anyhow::bail!("node not found");
                }
            },
            None => anyhow::bail!("node not found"),
        }
    }
    async fn get_node_optional(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>> {
        let n = store.get_node_if_exists(key).await?;

        Ok(n)

    }
    async fn get_node(
        store: &S,
        tree_height: usize,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<Hash> {
        match store.get_node_if_exists(key).await? {
            Some(r) => Ok(r.value),
            None => if MARK_LEAVES && key.level as usize == tree_height {
                Ok(Hasher::get_zero_hash_marked(tree_height-(key.level as usize)))
            }else{
                Ok(Hasher::get_zero_hash(tree_height-(key.level as usize)))
            },
        }
    }
    async fn get_nodes(
        store: &S,
        tree_height: usize,
        keys: &[KVQMerkleNodeKey<TABLE_TYPE>],
    ) -> anyhow::Result<Vec<Hash>> {
        let result = store.get_node_values(keys).await?;
        
        Ok(result
            .into_iter()
            .enumerate()
            .map(|(i, v)| match v {
                Some(v) => v,
                None => Hasher::get_zero_hash(tree_height - (keys[i].level as usize)),
            })
            .collect())
    }
    async fn get_sub_tree_proof(
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
            let value = Self::get_node(store, real_tree_height, leaf_key).await?;
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
        ).await?;
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
    async fn get_leaf(
        store: &S,
        key: &KVQMerkleNodeKey<TABLE_TYPE>,
    ) -> anyhow::Result<MerkleProofCore<Hash>> {
        let nodes = Self::get_nodes(
            store,
            key.level as usize,
            &vec![vec![*key], key.siblings(), vec![key.root()]].concat(),
        ).await?;
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