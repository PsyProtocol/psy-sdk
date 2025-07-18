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


#[cfg(test)]
mod tests {
    use crate::memory::simple::KVQSimpleMemoryBackingStore;
    use crate::snapshot::Snapshot;
    use crate::traits::{KVQBinaryStore, KVQPair};

    #[test]
    fn test_snapshot() {
        let store = KVQSimpleMemoryBackingStore::new();
        store.set_many_vec(vec![
            KVQPair{ key: vec![1,2,3], value: vec![4,5,6]},
            KVQPair{ key: vec![1,2,2,3,3], value: vec![4,5,6]},
            KVQPair{ key: vec![1,2,2,3,6], value: vec![7,8,9]},
            KVQPair{ key: vec![2,2,2,3,9], value: vec![1,7,8,9]},
        ]).unwrap();
        let result = store.get_exact(&vec![1, 2, 3]).unwrap();
        assert_eq!(result, vec![4, 5, 6]);

        let keys = vec![
            vec![1, 2, 3],
            vec![1, 2, 2, 3, 3],
            vec![1, 2, 2, 3, 6],
            vec![1, 2, 2, 3, 6],
            vec![2, 2, 2, 3, 0],
        ];
        let snapshot = store.create_snapshot(keys.clone()).unwrap();
        dbg!("snapshot: {:?}", snapshot.clone());
        assert_eq!(snapshot.affected_keys, keys);
        assert_eq!(snapshot.old_values.get(&vec![1, 2, 3]).unwrap().clone().unwrap(), vec![4,5,6]);
        assert_eq!(snapshot.old_values.get(&vec![1, 2, 2, 3, 3]).unwrap().clone().unwrap(), vec![4,5,6]);
        assert_eq!(snapshot.old_values.get(&vec![1, 2, 2, 3, 6]).unwrap().clone().unwrap(), vec![7,8,9]);
        assert_eq!(snapshot.old_values.get(&vec![1, 2, 2, 3, 6]).unwrap().clone().unwrap(), vec![7,8,9]);
        assert_eq!(snapshot.old_values.get(&vec![2, 2, 2, 3, 0]).unwrap().clone(), None);

        // delete key
        store.delete(&vec![1, 2, 2, 3, 6]).unwrap();
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 2, 3, 6]).unwrap(), None);
        
        let part_snapshot = snapshot.part_snapshot_entry(vec![vec![1, 2, 2, 3, 6]]);
        dbg!("part_snoapshot: {:?}", part_snapshot.clone());
        assert_eq!(part_snapshot.old_values.get(&vec![1, 2, 2, 3, 6]).unwrap().clone().unwrap(), vec![7, 8, 9]);

        // restore part snapshot entry
        store.restore_from_snapshot(part_snapshot).unwrap();
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 2, 3, 6]).unwrap().clone().unwrap(), vec![7, 8, 9]);

        // delete all
        store.delete_many(&keys).unwrap();
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 2, 3, 6]).unwrap(), None);
        assert_eq!(store.get_exact_if_exists(&vec![2, 2, 2, 3, 0]).unwrap(), None);
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 3]).unwrap(), None);

        store.clear();

        // restore all snapshot entry
        store.restore_from_snapshot(snapshot).unwrap();
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 2, 3, 6]).unwrap().clone().unwrap(), vec![7, 8, 9]);
        assert_eq!(store.get_exact_if_exists(&vec![2, 2, 2, 3, 0]).unwrap().clone(), None);
        assert_eq!(store.get_exact_if_exists(&vec![1, 2, 3]).unwrap().clone().unwrap(), vec![4,5,6]);
    }
}