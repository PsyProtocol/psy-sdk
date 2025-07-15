use kvq::traits::{KVQBinaryStore, KVQPair};
use reth_libmdbx::{
    Cursor, Database, Environment, EnvironmentFlags, Geometry, Mode, SyncMode, TransactionKind, WriteFlags, RO, RW
};
use std::{ops::RangeInclusive, path::PathBuf};

#[derive(Debug)]
pub struct KVQlibmdbxStore {
    env: Environment,
}

#[derive(Debug)]
pub struct Transaction<K: TransactionKind> {
    pub txn: reth_libmdbx::Transaction<K>,
    pub db: Database,
}

impl<K: TransactionKind> Transaction<K> {
    pub fn commit(self) -> anyhow::Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

impl KVQlibmdbxStore {
    pub fn new_read(path: &str) -> anyhow::Result<Self> {
        let flags = EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadOnly,
            coalesce: true,
            ..Default::default()
        };

        let env = Environment::builder()
            .set_max_dbs(1)
            .set_flags(flags)
            .open(PathBuf::from(path).as_path())?;

        Ok(Self { env })
    }

    pub fn new_write(path: &str) -> anyhow::Result<Self> {
        let flags = EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadWrite {
                sync_mode: SyncMode::Durable,
            },
            coalesce: true,
            ..Default::default()
        };

        let env = Environment::builder()
            .set_max_dbs(1)
            .set_flags(flags)
            .open(PathBuf::from(path).as_path())?;

        Ok(Self { env })
    }

    pub fn new_write_with_size(path: &str, size_gb: usize) -> anyhow::Result<Self> {
        let flags = EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadWrite {
                sync_mode: SyncMode::Durable,
            },
            coalesce: true,
            ..Default::default()
        };

        let env = Environment::builder()
            .set_max_dbs(1)
            .set_flags(flags)
            .set_geometry(Geometry {
                size: Some(..=(size_gb * 1024 * 1024 * 1024)),
                ..Default::default()
            })
            .open(PathBuf::from(path).as_path())?;

        Ok(Self { env })
    }

    pub fn begin_read(&self) -> anyhow::Result<Transaction<RO>> {
        let txn = self.env.begin_ro_txn()?;
        Ok(Transaction {
            db: txn.open_db(None)?,
            txn,
        })
    }

    pub fn begin_write(&self) -> anyhow::Result<Transaction<RW>> {
        let txn = self.env.begin_rw_txn()?;
        Ok(Transaction {
            db: txn.open_db(None)?,
            txn,
        })
    }

    pub fn with_read_txn<R>(
        &self,
        f: impl FnOnce(&Transaction<RO>) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let txn = self.env.begin_ro_txn()?;
        let db = txn.open_db(None)?;
        let txn = Transaction { txn, db };
        let result = f(&txn)?;
        txn.commit()?;
        Ok(result)
    }

    pub fn with_write_txn<R>(
        &self,
        f: impl FnOnce(&mut Transaction<RW>) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let txn = self.env.begin_rw_txn()?;
        let db = txn.open_db(None)?;
        let mut txn = Transaction { txn, db };
        let result = f(&mut txn)?;
        txn.commit()?;
        Ok(result)
    }
}

impl KVQBinaryStore for KVQlibmdbxStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        self.with_read_txn(|txn| txn.get_exact_if_exists(key))
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.with_read_txn(|txn| txn.get_exact(key))
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        self.with_read_txn(|txn| txn.get_many_exact(keys))
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.with_read_txn(|txn| txn.get_leq(key, fuzzy_bytes))
    }

    fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.with_read_txn(|txn| txn.get_fuzzy_range_leq_kv(key, fuzzy_bytes))
    }

    fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.with_read_txn(|txn| txn.get_leq_kv(key, fuzzy_bytes))
    }

    fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        self.with_read_txn(|txn| txn.get_many_leq(keys, fuzzy_bytes))
    }

    fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.with_read_txn(|txn| txn.get_many_leq_kv(keys, fuzzy_bytes))
    }
    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.with_write_txn(|txn| txn.set(key, value))
    }
    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.with_write_txn(|txn| txn.set_ref(key, value))
    }
    fn set_many_ref(&self, items: &[KVQPair<&Vec<u8>, &Vec<u8>>]) -> anyhow::Result<()> {
        self.with_write_txn(|txn| txn.set_many_ref(items))
    }
    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.with_write_txn(|txn| txn.set_many_vec(items))
    }
    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        self.with_write_txn(|txn| txn.delete(key))
    }
    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        self.with_write_txn(|txn| txn.delete_many(keys))
    }
    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        self.with_write_txn(|txn| txn.set_many_split_ref(keys, values))
    }
}

// Read-only transaction implementation
impl KVQBinaryStore for Transaction<RO> {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let value = self.txn.get::<Vec<u8>>(self.db.dbi(), key.as_slice())?;
        Ok(value)
    }
    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let value = self
            .txn
            .get::<Vec<u8>>(self.db.dbi(), key.as_slice())?
            .ok_or(anyhow::anyhow!("Key not found"))?;
        Ok(value)
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            let value = self
                .txn
                .get::<Vec<u8>>(self.db.dbi(), key.as_slice())?
                .ok_or(anyhow::anyhow!("Key not found"))?;
            result.push(value);
        }
        Ok(result)
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut last_value = None;
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        last_value = Some(v);
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(last_value)
    }

    fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut last_kv = None;
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        last_kv = Some(KVQPair { key: k, value: v });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(last_kv)
    }

    fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let result = self.get_leq(key, fuzzy_bytes)?;
            results.push(result);
        }
        Ok(results)
    }

    fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let result = self.get_leq_kv(key, fuzzy_bytes)?;
            results.push(result);
        }
        Ok(results)
    }

    fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut result = Vec::new();
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        result.push(KVQPair { key: k, value: v });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(result)
    }

    //warning: This is a read-only transaction, so all write operations will fail.
    fn set(&self, _key: Vec<u8>, _value: Vec<u8>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_ref(&self, _key: &Vec<u8>, _value: &Vec<u8>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_many_ref(&self, _items: &[KVQPair<&Vec<u8>, &Vec<u8>>]) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_many_vec(&self, _items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn delete(&self, _key: &Vec<u8>) -> anyhow::Result<bool> {
        Err(anyhow::anyhow!(
            "Attempted to delete using a read-only LMDB transaction"
        ))
    }

    fn delete_many(&self, _keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        Err(anyhow::anyhow!(
            "Attempted to delete using a read-only LMDB transaction"
        ))
    }

    fn set_many_split_ref(&self, _keys: &[Vec<u8>], _values: &[Vec<u8>]) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }
}

// Transaction<RW> has both read and write operations
impl KVQBinaryStore for Transaction<RW> {
    // Read operations (delegates to generic impl)
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let value = self.txn.get::<Vec<u8>>(self.db.dbi(), key.as_slice())?;
        Ok(value)
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let value = self
            .txn
            .get::<Vec<u8>>(self.db.dbi(), key.as_slice())?
            .ok_or(anyhow::anyhow!("Key not found"))?;
        Ok(value)
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            let value = self
                .txn
                .get::<Vec<u8>>(self.db.dbi(), key.as_slice())?
                .ok_or(anyhow::anyhow!("Key not found"))?;
            result.push(value);
        }
        Ok(result)
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut last_value = None;
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        last_value = Some(v);
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(last_value)
    }

    fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut result = Vec::new();
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        result.push(KVQPair { key: k, value: v });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(result)
    }

    fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_end = key.clone();
        let mut base_key = key.clone();
        let key_len = base_key.len();

        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let mut cursor = self.txn.cursor(&self.db)?;
        let range = range_to_inclusive(base_key.as_slice(), key_end.as_slice());

        let mut last_kv = None;
        for item in cursor.iter_from::<Vec<u8>, Vec<u8>>(range.start()) {
            match item {
                Ok((k, v)) => {
                    if k <= key_end {
                        last_kv = Some(KVQPair { key: k, value: v });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(last_kv)
    }

    fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get_leq(key, fuzzy_bytes)?);
        }
        Ok(results)
    }

    fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get_leq_kv(key, fuzzy_bytes)?);
        }
        Ok(results)
    }

    // Write operations
    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.set_ref(&key, &value)
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.txn
            .put(self.db.dbi(), key, value, WriteFlags::empty())?;
        Ok(())
    }

    fn set_many_ref(&self, items: &[KVQPair<&Vec<u8>, &Vec<u8>>]) -> anyhow::Result<()> {
        for item in items {
            self.txn
                .put(self.db.dbi(), item.key, item.value, WriteFlags::empty())?;
        }
        Ok(())
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.set_many_ref(
            &items
                .iter()
                .map(|x| KVQPair {
                    key: &x.key,
                    value: &x.value,
                })
                .collect::<Vec<_>>(),
        )
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        let removed = self.txn.del(self.db.dbi(), key, None)?;
        Ok(removed)
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let removed = self.txn.del(self.db.dbi(), key, None)?;
            results.push(removed);
        }
        Ok(results)
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!(
                "Keys and values must be of the same length"
            ));
        }
        for (key, value) in keys.iter().zip(values.iter()) {
            self.txn
                .put(self.db.dbi(), key, value, WriteFlags::empty())?;
        }
        Ok(())
    }
}
fn range_to_inclusive(start: &[u8], end: &[u8]) -> RangeInclusive<Vec<u8>> {
    let start_vec = start.to_vec();
    let end_vec = end.to_vec();
    start_vec..=end_vec
}
