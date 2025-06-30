use super::key::KVQMerkleNodeKey;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQSerializable;
use kvq::traits::KVQStoreAdapter;
use kvq::traits::KVQStoreAdapterReader;
use qed_crypto::hash::traits::hasher::MerkleZeroHasherWithMarkedLeaf;
use std::marker::PhantomData;

mod core;
mod fixed_config;
mod semi_fixed_config;
// pub mod core_imm; // Commented out due to missing traits in kvq

pub use core::{
    KVQMerkleTreeModelReaderCore,
    KVQMerkleTreeModelCore,
    CHECKPOINT_ID_FUZZY_SIZE
};

pub use fixed_config::{
    KVQFixedConfigMerkleTreeModelReaderCore,
    KVQFixedConfigMerkleTreeModelCore,
};

pub use semi_fixed_config::{KVQSemiFixedConfigMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore};


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
        S: KVQBinaryStore,
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
        S: KVQBinaryStore,
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
        S: KVQBinaryStore,
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
        S: KVQBinaryStore,
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
        S: KVQBinaryStore,
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
