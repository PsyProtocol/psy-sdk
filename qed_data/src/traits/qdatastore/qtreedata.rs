use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::{core::{ DeltaMerkleProofCore, MerkleProofCore}, spiderman::SpidermanUpdateProof}, traits::qhashable::QFieldHashable};
use crate::qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDCheckpointLeafStats, QEDL2BlockState};

use crate::config::store_config::QEDHasher;

use super::qmetadata::{QMetaDataStoreReaderSync, QMetaDataStoreWriterSync};


pub trait ActiveCheckpointReaderSync<F: RichField> {
    fn get_active_checkpoint(&self) -> anyhow::Result<u64>;
    fn get_active_checkpoint_f(&self) -> anyhow::Result<F>;
    fn get_active_writing_checkpoint(&self) -> anyhow::Result<u64>;
    fn get_active_writing_checkpoint_f(&self) -> anyhow::Result<F>;
}
pub trait ActiveCheckpointWriterSync<F: RichField> {
    fn set_active_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_checkpoint_f(&self, checkpoint_id: F) -> anyhow::Result<F>;
    fn set_active_writing_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_writing_checkpoint_f(&self, checkpoint_id: F) -> anyhow::Result<F>;

    fn set_active_checkpoint_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_checkpoint_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<F>;
    fn set_active_writing_checkpoint_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_writing_checkpoint_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<F>;
}

#[maybe_async::maybe_async(?Send)]
pub trait QTreeDataStoreReaderSync<F: RichField> {
    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_root_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_state_tree_root(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_user_contract_state_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_state_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        ).await
    }
    async fn get_user_contract_state_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_state_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_state_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        ).await
    }

    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_root_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_tree_root(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_user_contract_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_registration_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_registration_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            leaf_index.to_canonical_u64(),
        ).await
    }
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_registration_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            leaf_index.to_canonical_u64(),
        ).await
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }
    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }
    async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;

    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_contract_function_tree_root(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_contract_function_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_contract_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_contract_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_deposit_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_deposit_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_deposit_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_withdrawal_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_withdrawal_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64() as u32,
        ).await
    }
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_withdrawal_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_checkpoint_tree_root(checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_checkpoint_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        ).await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_checkpoint_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        ).await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>> {
        let contract_tree_root = self.get_contract_tree_root(checkpoint_id).await?;
        let deposit_tree_root = self.get_deposit_tree_root(checkpoint_id).await?;
        let user_tree_root = self.get_user_tree_root(checkpoint_id).await?;
        let withdrawal_tree_root = self.get_withdrawal_tree_root(checkpoint_id).await?;
        let user_registration_tree_root = self.get_user_registration_tree_root(checkpoint_id).await?;
        Ok(QEDCheckpointGlobalStateRoots {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
            user_registration_tree_root,
        })
    }
}


pub trait QTreeDataStoreWriterSync<F: RichField> {
    fn batch_append_user_registration_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>;
    fn batch_append_user_registration_tree_f(&self, checkpoint_id: F, start_leaf_index: F, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>> {
        self.batch_append_user_registration_tree(
            checkpoint_id.to_canonical_u64(),
            start_leaf_index.to_canonical_u64(),
            sub_tree_height,
            leaf_hashes,
        )
    }


    fn set_user_state_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_state_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_user_state_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_user_contract_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            leaf_hash,
        )
    }

    fn set_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_user_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_deposit_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_withdrawal_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64(),
            leaf_hash,
        )
    }


    fn set_contract_function_whitelist(&self, checkpoint_id: u64, contract_id: u64, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>>;
    fn set_contract_function_whitelist_f(&self, checkpoint_id: F, contract_id: F, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>> {
        self.set_contract_function_whitelist(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaves,
        )
    }

    fn batch_append_contract_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>;

    fn set_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_contract_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        self.set_checkpoint_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            leaf_hash,
        )
    }
}


#[maybe_async::maybe_async(?Send)]
pub trait QEDComboDataStoreReaderSync<F: RichField>: QMetaDataStoreReaderSync<F> + QTreeDataStoreReaderSync<F> {}
pub trait QEDComboDataStoreWriterSync<F: RichField>: QMetaDataStoreWriterSync<F> + QTreeDataStoreWriterSync<F> {}

#[maybe_async::maybe_async(?Send)]
pub trait QEDComboDataStoreReaderWriterSync<F: RichField>: QEDComboDataStoreReaderSync<F> + QEDComboDataStoreWriterSync<F> {
    async fn initialize_store(&self) -> anyhow::Result<u64> {

        let latest_l2_block_state_or_err = self.get_latest_l2_block_state().await;
        if latest_l2_block_state_or_err.is_ok() {
            let v = latest_l2_block_state_or_err.unwrap();
            Ok(v.checkpoint_id)
        }else{
            // database not initialized with data for the genesis block

            let genesis_l2_block_state = QEDL2BlockState::get_genesis_value();

            let genesis_checkpoint_stats = QEDCheckpointLeafStats::get_genesis_value();
            let stats_hash = genesis_checkpoint_stats.qfhash::<QEDHasher>();
            let genesis_global_state_roots = self.get_checkpoint_global_state_roots(1).await?;
            let genesis_checkpoint_leaf = QEDCheckpointLeaf{
                global_chain_root: genesis_global_state_roots.qfhash::<QEDHasher>(),
                stats: genesis_checkpoint_stats,
            };


            println!("genesis_stats_hash: {:?} ({})",stats_hash, serde_json::to_string_pretty(&stats_hash).unwrap());

            println!("genesis_global_state_roots: {}",serde_json::to_string_pretty(&genesis_global_state_roots).unwrap());
            println!("genesis_checkpoint_leaf: {}",serde_json::to_string_pretty(&genesis_checkpoint_leaf).unwrap());

            self.set_l2_block_state(&genesis_l2_block_state)?;
            self.set_checkpoint_leaf_data(0, &genesis_checkpoint_leaf)?;
            self.set_checkpoint_tree_leaf_hash(0, genesis_checkpoint_leaf.qfhash::<QEDHasher>())?;

            Ok(0)

        }

    }
}
