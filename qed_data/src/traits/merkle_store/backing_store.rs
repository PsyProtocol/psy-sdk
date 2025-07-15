use async_trait::async_trait;
use kvq::traits::{KVQPair, KVQSerializable};

use crate::models::kvq_merkle::key::KVQMerkleNodeKey;



pub trait QMerkleStoreHash: Copy + Clone + Send + Sync + KVQSerializable {

}

impl<T: Copy + Clone + Send + Sync + KVQSerializable> QMerkleStoreHash for T {
}


#[async_trait]
pub trait MerkleNodeStoreReaderImmutableAsync<Hash: QMerkleStoreHash, const TABLE_TYPE: u16> {
    async fn get_node_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>>;
    async fn get_node_value_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<Hash>>;
    async fn get_node_values(&self, keys: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Vec<Option<Hash>>>;



    /*
    async fn get_node_latest(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_node(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_node_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Option<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>>;
    async fn get_nodes(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_nodes_same_tree(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>>;
    async fn get_node_exact(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;


    async fn get_node_value_latest(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;
    async fn get_node_value(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;
    async fn get_node_value_if_exists(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;
    async fn get_node_values(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Hash>;
    async fn get_node_values_same_tree(&self, key: &[KVQMerkleNodeKey<TABLE_TYPE>]) -> anyhow::Result<Hash>;
    async fn get_node_value_exact(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>) -> anyhow::Result<Hash>;*/

}
#[async_trait]
pub trait MerkleNodeStoreWriterImmutableAsync<Hash: QMerkleStoreHash, const TABLE_TYPE: u16> {
    async fn set_node_params(&self, key: &KVQMerkleNodeKey<TABLE_TYPE>, value: Hash) -> anyhow::Result<()>;

    async fn set_node(&self, node: &KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>) -> anyhow::Result<()> {
        self.set_node_params(&node.key, node.value).await
    }
    async fn set_nodes(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()>;
    async fn set_nodes_same_tree(&self, nodes: &[KVQPair<KVQMerkleNodeKey<TABLE_TYPE>, Hash>]) -> anyhow::Result<()>;
}



pub trait MerkleNodeStoreImmutableAsync<Hash: QMerkleStoreHash, const TABLE_TYPE: u16>: MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> + MerkleNodeStoreWriterImmutableAsync<Hash, TABLE_TYPE> {

}

impl<S:MerkleNodeStoreReaderImmutableAsync<Hash, TABLE_TYPE> + MerkleNodeStoreWriterImmutableAsync<Hash, TABLE_TYPE>, Hash: QMerkleStoreHash, const TABLE_TYPE: u16> MerkleNodeStoreImmutableAsync<Hash, TABLE_TYPE> for S {

}
