pub mod simple;
use auto_impl::auto_impl;
use serde::{Deserialize, Serialize};
pub use simple::*;

use crate::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait KVQBinaryStoreCacheAsync: KVQBinaryStoreAsync {
    async fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;

    async fn clear_cache(&self);
    async fn flush_simple(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    async fn is_removed(&self, key: &Vec<u8>) -> bool;
    async fn get_non_removed_keys(&self) -> Vec<Vec<u8>>;
    async fn get_removed_keys(&self) -> Vec<Vec<u8>>;
}

#[auto_impl(&, Box, Arc)]
pub trait KVQBinaryStoreCache: KVQBinaryStore {
    fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    fn clear_cache(&self);
    fn flush_simple(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    fn is_removed(&self, key: &Vec<u8>) -> bool;
    fn get_non_removed_keys(&self) -> Vec<Vec<u8>>;
    fn get_removed_keys(&self) -> Vec<Vec<u8>>;
}
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheValueType {
    Bytes(Vec<u8>),
    Removed,
}
