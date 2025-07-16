#[cfg(not(target_arch = "wasm32"))]
pub mod async_cache;
#[cfg(not(target_arch = "wasm32"))]
pub use async_cache::*;

pub mod sync_cache;
pub use sync_cache::*;

use auto_impl::auto_impl;
use crate::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait KVQBinaryStoreCachedTraitAsync: KVQBinaryStoreAsync {
    async fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;

    async fn clear_cache(&self);
    async fn flush_simple(&self) -> anyhow::Result<()>;
    async fn is_removed(&self, key: &Vec<u8>) -> bool;
    async fn get_non_removed_keys(&self) -> Vec<Vec<u8>>;
    async fn get_removed_keys(&self) -> Vec<Vec<u8>>;
}

#[auto_impl(&, Box, Arc)]
pub trait KVQBinaryStoreCachedTrait: KVQBinaryStore {
    fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    fn clear_cache(&self);
    fn flush_simple(&self) -> anyhow::Result<()>;
    fn is_removed(&self, key: &Vec<u8>) -> bool;
    fn get_non_removed_keys(&self) -> Vec<Vec<u8>>;
    fn get_removed_keys(&self) -> Vec<Vec<u8>>;
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CacheValueType {
    Bytes(Vec<u8>),
    Removed,
}


