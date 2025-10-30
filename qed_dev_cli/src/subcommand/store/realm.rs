use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::{
    checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState},
    user::QEDUserLeaf,
};
use qed_node::realm::error::RpcError;
use qed_store::{node::realm::QEDRealmStoreReaderAsync, store::journal::Journal};

use crate::subcommand::store::{
    utils::{C, D, F},
    StoreProvider,
};

#[rpc(server, client, namespace = "qed")]
pub trait RealmStoreRpc {
    #[method(name = "get_snapshot")]
    async fn get_snapshot(&self) -> RpcResult<Vec<u8>>;

    #[method(name = "restore_snapshot")]
    async fn restore_snapshot(&self, snapshot: Vec<u8>) -> RpcResult<()>;

    #[method(name = "commit")]
    async fn commit(&self, checkpoint_id: u64) -> RpcResult<()>;

    #[method(name = "rollback")]
    async fn rollback(&self, checkpoint_id: u64) -> RpcResult<()>;

    #[method(name = "get_checkpoint_leaf_data")]
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointLeaf<F>>;

    #[method(name = "get_latest_l2_block_state")]
    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_l2_block_state")]
    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState>;

    #[method(name = "get_user_registration_tree_root")]
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_latest_checkpoint_tree_root")]
    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_root")]
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_leaf_hash")]
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_checkpoint_tree_merkle_proof")]
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_checkpoint_global_state_roots")]
    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointGlobalStateRoots<F>>;

    #[method(name = "get_user_leaf_data")]
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QEDUserLeaf<F>>;

    #[method(name = "get_user_contract_state_tree_root")]
    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_leaf_hash")]
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_state_tree_merkle_proof")]
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_contract_tree_root")]
    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_leaf_hash")]
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_contract_tree_merkle_proof")]
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_root")]
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_tree_leaf_hash")]
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>>;

    #[method(name = "get_user_bottom_tree_merkle_proof")]
    async fn get_user_bottom_tree_merkle_proof(&self, root_level: u8, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_sub_tree_merkle_proof")]
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>>;

    #[method(name = "get_user_tree_merkle_proof")]
    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>>;
}

#[async_trait]
impl RealmStoreRpcServer for StoreProvider {
    async fn get_snapshot(&self) -> RpcResult<Vec<u8>> {
        match self.store.get_cache() {
            Ok(snapshot) => Ok(snapshot.unwrap_or(vec![])),
            Err(e) => Err(RpcError::Anyhow(e).into()),
        }
    }

    async fn restore_snapshot(&self, snapshot: Vec<u8>) -> RpcResult<()> {
        Ok(self.store.restore_cache(snapshot).map_err(RpcError::Anyhow)?)
    }
    async fn commit(&self, checkpoint_id: u64) -> RpcResult<()> {
        let _ = self.store.commit(Some(checkpoint_id)).map_err(RpcError::Anyhow)?;
        Ok(())
    }

    async fn rollback(&self, checkpoint_id: u64) -> RpcResult<()> {
        Ok(self.store.rollback(checkpoint_id).map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointLeaf<F>> {
        Ok(QEDRealmStoreReaderAsync::get_checkpoint_leaf_data(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState> {
        Ok(QEDRealmStoreReaderAsync::get_latest_l2_block_state(&self.store)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState> {
        Ok(QEDRealmStoreReaderAsync::get_l2_block_state(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_user_registration_tree_root(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_latest_checkpoint_tree_root(&self.store)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_checkpoint_tree_root(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(
            QEDRealmStoreReaderAsync::get_checkpoint_tree_leaf_hash(&self.store, checkpoint_id, leaf_checkpoint_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(
            QEDRealmStoreReaderAsync::get_checkpoint_tree_merkle_proof(&self.store, checkpoint_id, leaf_checkpoint_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<QEDCheckpointGlobalStateRoots<F>> {
        Ok(QEDRealmStoreReaderAsync::get_checkpoint_global_state_roots(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QEDUserLeaf<F>> {
        Ok(QEDRealmStoreReaderAsync::get_user_leaf_data(&self.store, checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_contract_state_tree_root(&self.store, checkpoint_id, user_id, contract_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_contract_state_tree_leaf_hash(&self.store, checkpoint_id, user_id, contract_id, height, leaf_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_contract_state_tree_merkle_proof(&self.store, checkpoint_id, user_id, contract_id, height, leaf_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_user_contract_tree_root(&self.store, checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_contract_tree_leaf_hash(&self.store, checkpoint_id, user_id, contract_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_contract_tree_merkle_proof(&self.store, checkpoint_id, user_id, contract_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_user_tree_root(&self.store, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(QEDRealmStoreReaderAsync::get_user_tree_leaf_hash(&self.store, checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_bottom_tree_merkle_proof(&self, root_level: u8, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_bottom_tree_merkle_proof(&self.store, root_level, checkpoint_id, user_id)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(
            QEDRealmStoreReaderAsync::get_user_sub_tree_merkle_proof(&self.store, checkpoint_id, root_level, leaf_level, leaf_index)
                .await
                .map_err(RpcError::Anyhow)?,
        )
    }

    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(QEDRealmStoreReaderAsync::get_user_tree_merkle_proof(&self.store, checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }
}
