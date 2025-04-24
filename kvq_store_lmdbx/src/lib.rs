use kvq::traits::{KVQBinaryStoreReader, KVQBinaryStoreWriter, KVQPair};
use reth_libmdbx::{
    Cursor, Database, Environment, Transaction, TransactionKind, WriteFlags, RO, RW,
};
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct KVQlibmdbxStore<K: TransactionKind> {
    txn: Transaction<K>,
    db: Database,
}

impl<K: TransactionKind> KVQlibmdbxStore<K> {
    pub fn new(txn: Transaction<K>) -> anyhow::Result<Self> {
        Ok(Self {
            db: txn.open_db(None)?,
            txn,
        })
    }
}

impl<K: TransactionKind> KVQBinaryStoreReader for KVQlibmdbxStore<K> {
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

    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.txn.get::<Vec<u8>>(self.db.dbi(), key.as_slice())?)
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
}

impl KVQBinaryStoreWriter for KVQlibmdbxStore<RW> {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.set_ref(&key, &value)
    }

    fn set_ref(&mut self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.txn
            .put(self.db.dbi(), key, value, WriteFlags::empty())?;
        Ok(())
    }

    fn set_many_ref(&mut self, items: &[KVQPair<&Vec<u8>, &Vec<u8>>]) -> anyhow::Result<()> {
        for item in items {
            self.txn
                .put(self.db.dbi(), item.key, item.value, WriteFlags::empty())?;
        }
        Ok(())
    }

    fn set_many_vec(&mut self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
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

    fn delete(&mut self, key: &Vec<u8>) -> anyhow::Result<bool> {
        let removed = self.txn.del(self.db.dbi(), key, None)?;
        Ok(removed)
    }

    fn delete_many(&mut self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let removed = self.txn.del(self.db.dbi(), key, None)?;
            results.push(removed);
        }
        Ok(results)
    }

    fn set_many_split_ref(&mut self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
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

//warning: This is a read-only transaction, so all write operations will fail.
impl KVQBinaryStoreWriter for KVQlibmdbxStore<RO> {
    fn set(&mut self, _key: Vec<u8>, _value: Vec<u8>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_ref(&mut self, _key: &Vec<u8>, _value: &Vec<u8>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_many_ref(&mut self, _items: &[KVQPair<&Vec<u8>, &Vec<u8>>]) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn set_many_vec(&mut self, _items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }

    fn delete(&mut self, _key: &Vec<u8>) -> anyhow::Result<bool> {
        Err(anyhow::anyhow!(
            "Attempted to delete using a read-only LMDB transaction"
        ))
    }

    fn delete_many(&mut self, _keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        Err(anyhow::anyhow!(
            "Attempted to delete using a read-only LMDB transaction"
        ))
    }

    fn set_many_split_ref(&mut self, _keys: &[Vec<u8>], _values: &[Vec<u8>]) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Attempted to write using a read-only LMDB transaction"
        ))
    }
}
fn range_to_inclusive(start: &[u8], end: &[u8]) -> RangeInclusive<Vec<u8>> {
    let start_vec = start.to_vec();
    let end_vec = end.to_vec();
    start_vec..=end_vec
}
