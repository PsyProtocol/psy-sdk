use kvq::traits::KVQSerializable;

pub trait PsyBlobStoreReader {
    fn get_bin(&self, key: Vec<u8>) -> anyhow::Result<Vec<u8>>;
    fn exists_bin(&self, key: Vec<u8>) -> anyhow::Result<bool>;
    fn get<K: KVQSerializable, V: KVQSerializable>(&self, key: &K) -> anyhow::Result<V> {
        V::from_bytes(&self.get_bin(key.to_bytes()?)?)
    }
    fn exists<K: KVQSerializable>(&self, key: &K) -> anyhow::Result<bool> {
        self.exists_bin(key.to_bytes()?)
    }
}
pub trait PsyBlobStoreMut: PsyBlobStoreReader {
    fn set_bin_mut(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()>;
    fn remove_bin_mut(&mut self, key: Vec<u8>) -> anyhow::Result<bool>;
    fn pop_bin_mut(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>>;

    fn set_mut<K: KVQSerializable, V: KVQSerializable>(&mut self, key: &K, value: &V) -> anyhow::Result<()> {
        self.set_bin_mut(key.to_bytes()?, value.to_bytes()?)
    }
    fn remove_mut<K: KVQSerializable>(&mut self, key: &K) -> anyhow::Result<bool> {
        self.remove_bin_mut(key.to_bytes()?)
    }
    fn pop_mut<K: KVQSerializable, V: KVQSerializable>(&mut self, key: &K) -> anyhow::Result<Option<V>> {
        Ok(match self.pop_bin_mut(key.to_bytes()?)? {
            Some(data) => Some(V::from_bytes(&data)?),
            None => None,
        })
    }
}

pub trait PsyBlobStoreImm: PsyBlobStoreReader {
    fn set_bin_imm(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()>;
    fn get_bin_imm(&self, key: Vec<u8>) -> anyhow::Result<Vec<u8>>;
    fn remove_bin_imm(&self, key: Vec<u8>) -> anyhow::Result<bool>;
    fn pop_bin_imm(&self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>>;

    fn set_imm<K: KVQSerializable, V: KVQSerializable>(&self, key: &K, value: &V) -> anyhow::Result<()> {
        self.set_bin_imm(key.to_bytes()?, value.to_bytes()?)
    }
    fn remove_imm<K: KVQSerializable>(&self, key: &K) -> anyhow::Result<bool> {
        self.remove_bin_imm(key.to_bytes()?)
    }
    fn pop_imm<K: KVQSerializable, V: KVQSerializable>(&self, key: &K) -> anyhow::Result<Option<V>> {
        Ok(match self.pop_bin_imm(key.to_bytes()?)? {
            Some(data) => Some(V::from_bytes(&data)?),
            None => None,
        })
    }
}
