use crate::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    pub snapshot_id: String,
    pub affected_keys: Vec<Vec<u8>>,
    pub old_values: HashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl SnapshotEntry {
    pub fn part_snapshot_entry(&self, affected_keys: Vec<Vec<u8>>) -> Self {
        let mut entry = SnapshotEntry{
            snapshot_id: self.snapshot_id.clone(),
            affected_keys: Vec::with_capacity(affected_keys.len()),
            old_values: HashMap::new(),
        };

        for key in affected_keys {
            if let Some(value) = self.old_values.get(&key) {
                entry.old_values.insert(key.clone(), value.clone());
                entry.affected_keys.push(key);
            }
        }
        entry
    }
}

pub trait Snapshot {
    fn create_snapshot(&self, keys: Vec<Vec<u8>>) -> anyhow::Result<SnapshotEntry>;
    fn restore_from_snapshot(&self, snapshot: SnapshotEntry) -> anyhow::Result<()>;
}

impl<T: KVQBinaryStore> Snapshot for T {
    fn create_snapshot(&self, keys: Vec<Vec<u8>>) -> anyhow::Result<SnapshotEntry> {
        let mut snapshot = SnapshotEntry {
            snapshot_id: Uuid::new_v4().to_string(),
            affected_keys: keys.clone(),
            old_values: HashMap::with_capacity(keys.len()),
        };
        for key in keys {
            let value = self.get_exact_if_exists(&key)?;
            snapshot.old_values.insert(key, value);
        }
        Ok(snapshot)
    }

    fn restore_from_snapshot(&self, snapshot: SnapshotEntry) -> anyhow::Result<()> {
        let affected_keys = snapshot.affected_keys.clone();
        let old_values = snapshot.old_values.clone();

        let mut operations = Vec::new();
        for key in affected_keys {
            if let Some(Some(value)) = old_values.get(&key) {
                operations.push(KVQPair {
                    key,
                    value: value.clone(),
                });
            }
        }
        self.set_many_vec(operations)?;

        Ok(())
    }
}

#[async_trait::async_trait]
pub trait SnapshotAsync {
    async fn create_snapshot(&self, keys: Vec<Vec<u8>>) -> anyhow::Result<SnapshotEntry>;
    async fn restore_from_snapshot(&self, snapshot: SnapshotEntry) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<T: KVQBinaryStoreAsync + Sync> SnapshotAsync for T {
    async fn create_snapshot(&self, keys: Vec<Vec<u8>>) -> anyhow::Result<SnapshotEntry> {
        let mut snapshot = SnapshotEntry {
            snapshot_id: Uuid::new_v4().to_string(),
            affected_keys: keys.clone(),
            old_values: HashMap::with_capacity(keys.len()),
        };
        for key in keys {
            let value = self.get_exact_if_exists(&key).await?;
            snapshot.old_values.insert(key, value);
        }
        Ok(snapshot)
    }

    async fn restore_from_snapshot(&self, snapshot: SnapshotEntry) -> anyhow::Result<()> {
        let affected_keys = snapshot.affected_keys.clone();
        let old_values = snapshot.old_values.clone();

        let mut set_ops = Vec::new();
        let mut del_ops = Vec::new();
        for key in affected_keys {
            if let Some(value) = old_values.get(&key) {
                if let Some(value) = value {
                    set_ops.push(KVQPair {
                        key,
                        value: value.clone(),
                    });
                } else {
                    del_ops.push(key);
                }
            }
        }
        self.set_many_vec(set_ops).await?;
        self.delete_many(&del_ops).await?;

        Ok(())
    }
}
