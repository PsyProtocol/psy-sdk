use std::fmt::Debug;

use async_trait::async_trait;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::core::MerkleProofCore;
use serde::{Deserialize, Serialize};
use tokio::{runtime::Handle, sync::mpsc, task::block_in_place};
use tracing::error;

use super::Journal;
use crate::queue::redis_queue::QueueOffsetState;

/// Backup request sent to the backup task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupRequest {
    Checkpoint(BackupCheckpoint),
    PendingUsers(BackupPendingUsers),
    RealmState(BackupRealmState),
    PendingUsersQueueState(QueueOffsetState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCheckpoint {
    pub checkpoint_id: u64,
    pub pair_to_set: Vec<(Vec<u8>, Vec<u8>)>,
    pub removed_keys: Vec<Vec<u8>>,
    pub is_committed: bool,
}

type F = psy_data::config::store_config::PsyFelt;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPendingUsers {
    pub checkpoint_id: u64,
    pub start_user_index: u64,
    pub pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRealmState {
    pub checkpoint_id: u64,
    pub last_processed_user_num: u64,
    pub current_processed_user_num: u64,
    pub total_pending_users: u64,
    pub is_committed: bool,
}

/// Callback trait for handling backup operations
#[async_trait]
pub trait BackupHandler: Send + Sync {
    async fn handle_backup(&self, request: BackupRequest) -> anyhow::Result<()>;
}

/// BackupJournalStore wraps a Journal store and automatically triggers backup
/// on commit
#[derive(Clone)]
pub struct BackupJournalStore<J: Journal> {
    inner: J,
    backup_tx: Option<mpsc::UnboundedSender<BackupRequest>>,
}

impl<J: Journal> BackupJournalStore<J> {
    pub fn new(inner: J) -> Self {
        Self { inner, backup_tx: None }
    }

    pub fn new_with_backup(inner: J, backup_tx: mpsc::UnboundedSender<BackupRequest>) -> Self {
        Self {
            inner,
            backup_tx: Some(backup_tx),
        }
    }

    pub fn enable_backup(&mut self, backup_tx: mpsc::UnboundedSender<BackupRequest>) {
        self.backup_tx = Some(backup_tx);
    }

    pub fn disable_backup(&mut self) {
        self.backup_tx = None;
    }

    fn send_backup_request_inner(&self, request: BackupRequest) {
        if let Some(ref backup_tx) = self.backup_tx {
            if let Err(e) = backup_tx.send(request) {
                error!("❌ Failed to send backup request for checkpoint {:?}: {}", 0, e);
            }
        }
    }

    fn send_backup_checkpoint_request(
        &self,
        checkpoint_id: u64,
        pair_to_set: Vec<KVQPair<Vec<u8>, Vec<u8>>>,
        removed_keys: Vec<Vec<u8>>,
        is_committed: bool,
    ) {
        let request = BackupCheckpoint {
            checkpoint_id,
            pair_to_set: pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect(),
            removed_keys,
            is_committed,
        };
        self.send_backup_request_inner(BackupRequest::Checkpoint(request));
    }
}

#[async_trait]
impl<J: Journal> BackupHandler for BackupJournalStore<J> {
    async fn handle_backup(&self, request: BackupRequest) -> anyhow::Result<()> {
        self.send_backup_request_inner(request);
        Ok(())
    }
}

#[async_trait]
impl<J: Journal + Send + Sync> KVQBinaryStoreAsync for BackupJournalStore<J> {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        <J as KVQBinaryStoreAsync>::get_exact_if_exists(&self.inner, key).await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        <J as KVQBinaryStoreAsync>::get_exact(&self.inner, key).await
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        <J as KVQBinaryStoreAsync>::get_many_exact(&self.inner, keys).await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        <J as KVQBinaryStoreAsync>::get_leq(&self.inner, key, fuzzy_bytes).await
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        <J as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(&self.inner, key, fuzzy_bytes).await
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        <J as KVQBinaryStoreAsync>::get_leq_kv(&self.inner, key, fuzzy_bytes).await
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        <J as KVQBinaryStoreAsync>::get_many_leq(&self.inner, keys, fuzzy_bytes).await
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        <J as KVQBinaryStoreAsync>::get_many_leq_kv(&self.inner, keys, fuzzy_bytes).await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        <J as KVQBinaryStoreAsync>::set(&self.inner, key, value).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        <J as KVQBinaryStoreAsync>::set_ref(&self.inner, key, value).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        <J as KVQBinaryStoreAsync>::set_many_ref(&self.inner, items).await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        <J as KVQBinaryStoreAsync>::set_many_vec(&self.inner, items).await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        <J as KVQBinaryStoreAsync>::set_many_split_ref(&self.inner, keys, values).await
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        <J as KVQBinaryStoreAsync>::delete(&self.inner, key).await
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        <J as KVQBinaryStoreAsync>::delete_many(&self.inner, keys).await
    }
}

#[async_trait]
impl<J: Journal + Send + Sync> Journal for BackupJournalStore<J> {
    async fn commit(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let result = <J as Journal>::commit(&self.inner, checkpoint_id).await?;

        if let Some(checkpoint_id) = checkpoint_id {
            self.send_backup_checkpoint_request(checkpoint_id, result.0.clone(), result.1.clone(), true);
        }

        Ok(result)
    }

    async fn is_committed(&self) -> bool {
        <J as Journal>::is_committed(&self.inner).await
    }

    async fn rollback(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        <J as Journal>::rollback(&self.inner, checkpoint_id).await
    }

    async fn restore_cache(&self, cache: Vec<u8>) -> anyhow::Result<()> {
        <J as Journal>::restore_cache(&self.inner, cache).await
    }

    async fn get_cache(&self) -> anyhow::Result<Option<Vec<u8>>> {
        <J as Journal>::get_cache(&self.inner).await
    }

    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync {
        <J as Journal>::get_base_store(&self.inner).await
    }
}

impl<J: Journal + Send + Sync> KVQBinaryStore for BackupJournalStore<J> {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_exact_if_exists(self, key).await }))
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_exact(self, key).await }))
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_exact(self, keys).await }))
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_leq(self, key, fuzzy_bytes).await }))
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        block_in_place(|| Handle::current().block_on(async {
            <Self as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(self, key, fuzzy_bytes).await
        }))
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_leq_kv(self, key, fuzzy_bytes).await }))
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_leq(self, keys, fuzzy_bytes).await }))
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_leq_kv(self, keys, fuzzy_bytes).await }))
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set(self, key, value).await }))
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_ref(self, key, value).await }))
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_ref(self, items).await }))
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_vec(self, items).await }))
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete(self, key).await }))
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete_many(self, keys).await }))
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        block_in_place(|| Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_split_ref(self, keys, values).await }))
    }
}
