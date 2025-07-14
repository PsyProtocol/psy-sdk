use std::hash::Hash;

use crate::models::kvq_merkle::key::KVQMerkleNodeKey;

#[derive(Debug, Clone)]
pub struct TreeNodeCache<H: Copy + PartialEq + Clone + Hash, const TABLE_TYPE: u16> {
    pub node_cache: hashbrown::HashMap<KVQMerkleNodeKey<TABLE_TYPE>, H>,
}
