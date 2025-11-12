use std::fmt::Debug;

use async_trait::async_trait;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::core::MerkleProofCore;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::error;

use super::{Journal, JournalAsync};
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

impl<J: Journal> KVQBinaryStore for BackupJournalStore<J> {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_exact_if_exists(key)
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.inner.get_exact(key)
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        self.inner.get_many_exact(keys)
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_leq(key, fuzzy_bytes)
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_fuzzy_range_leq_kv(key, fuzzy_bytes)
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_leq_kv(key, fuzzy_bytes)
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        self.inner.get_many_leq(keys, fuzzy_bytes)
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.inner.get_many_leq_kv(keys, fuzzy_bytes)
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.inner.set(key, value)
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.inner.set_ref(key, value)
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        self.inner.set_many_ref(items)
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.inner.set_many_vec(items)
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        self.inner.set_many_split_ref(keys, values)
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        self.inner.delete(key)
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        self.inner.delete_many(keys)
    }
}

impl<J: Journal> Journal for BackupJournalStore<J> {
    fn commit(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let result = self.inner.commit(checkpoint_id)?;

        // Send backup request after successful commit
        if let Some(checkpoint_id) = checkpoint_id {
            self.send_backup_checkpoint_request(checkpoint_id, result.0.clone(), result.1.clone(), true);
        }

        Ok(result)
    }

    fn is_committed(&self) -> bool {
        self.inner.is_committed()
    }

    fn rollback(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.rollback(checkpoint_id)
    }

    fn restore_cache(&self, cache: Vec<u8>) -> anyhow::Result<()> {
        self.inner.restore_cache(cache)
    }

    fn get_cache(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_cache()
    }

    fn get_base_store(&self) -> &dyn KVQBinaryStore {
        self.inner.get_base_store()
    }
}

/// Async version of BackupJournalStore
pub struct BackupJournalStoreAsync<J: JournalAsync> {
    inner: J,
    backup_tx: Option<mpsc::UnboundedSender<BackupRequest>>,
}

impl<J: JournalAsync> BackupJournalStoreAsync<J> {
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

    fn send_backup_request(&self, request: BackupRequest) {
        if let Some(ref backup_tx) = self.backup_tx {
            if let Err(e) = backup_tx.send(request) {
                error!("❌ Failed to send backup request: {}", e);
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
        self.send_backup_request(BackupRequest::Checkpoint(request));
    }
}

#[async_trait]
impl<J: JournalAsync + Send + Sync> KVQBinaryStoreAsync for BackupJournalStoreAsync<J> {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_exact_if_exists(key).await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.inner.get_exact(key).await
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        self.inner.get_many_exact(keys).await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_leq(key, fuzzy_bytes).await
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_fuzzy_range_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        self.inner.get_many_leq(keys, fuzzy_bytes).await
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.inner.get_many_leq_kv(keys, fuzzy_bytes).await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.inner.set(key, value).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.inner.set_ref(key, value).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        self.inner.set_many_ref(items).await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.inner.set_many_vec(items).await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        self.inner.set_many_split_ref(keys, values).await
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        self.inner.delete(key).await
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        self.inner.delete_many(keys).await
    }
}

#[async_trait]
impl<J: JournalAsync + Send + Sync> JournalAsync for BackupJournalStoreAsync<J> {
    async fn commit(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let result = self.inner.commit(checkpoint_id).await?;

        // Send backup request after successful commit
        if let Some(checkpoint_id) = checkpoint_id {
            self.send_backup_checkpoint_request(checkpoint_id, result.0.clone(), result.1.clone(), true);
        }

        Ok(result)
    }

    async fn is_committed(&self) -> bool {
        self.inner.is_committed().await
    }

    async fn rollback(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.rollback(checkpoint_id).await
    }

    async fn restore_cache(&self, cache: Vec<u8>) -> anyhow::Result<()> {
        self.inner.restore_cache(cache).await
    }

    async fn get_cache(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_cache().await
    }

    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync {
        self.inner.get_base_store().await
    }
}
