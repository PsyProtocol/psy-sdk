mod backing_store;
mod tree_core;
mod reader_core;
mod node_cache;

use std::marker::PhantomData;

pub use backing_store::{
    MerkleNodeStoreImmutableAsync,
    MerkleNodeStoreReaderImmutableAsync,
    MerkleNodeStoreWriterImmutableAsync,
};
use kvq::traits::KVQSerializable;
use qed_crypto::hash::traits::hasher::MerkleZeroHasherWithMarkedLeaf;
use serde::Serialize;
pub use tree_core::QMerkleTreeModelCoreImmutableAsync;
pub use reader_core::QEDMerkleTreeModelReaderCoreAsync;



pub struct QMerkleTreeModel<
S,
Hash: Copy + Clone + Send + Sync + KVQSerializable + Default + PartialEq,
Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
const TABLE_TYPE: u16,
const MARK_LEAVES: bool,
> {
    _hasher: PhantomData<Hasher>,
    _hash: PhantomData<Hash>,
    _s: PhantomData<S>,
}


impl<
S: MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> + Send + Sync,
Hash: Copy + Clone + Send + Sync + KVQSerializable + Default + PartialEq,
Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
const TABLE_TYPE: u16,
const MARK_LEAVES: bool,
    > QEDMerkleTreeModelReaderCoreAsync<S,  Hash, Hasher, TABLE_TYPE, MARK_LEAVES>
    for QMerkleTreeModel<S, Hash, Hasher, TABLE_TYPE, MARK_LEAVES>
{
}

impl<
S: MerkleNodeStoreImmutableAsync<Hash, TABLE_TYPE> + Send + Sync,
Hash: Copy + Clone + Send + Sync + KVQSerializable + Default + PartialEq + Serialize,
Hasher: MerkleZeroHasherWithMarkedLeaf<Hash>,
const TABLE_TYPE: u16,
const MARK_LEAVES: bool,
    > QMerkleTreeModelCoreImmutableAsync<S,  Hash, Hasher, TABLE_TYPE, MARK_LEAVES>
    for QMerkleTreeModel<S, Hash, Hasher, TABLE_TYPE, MARK_LEAVES>
{
}
