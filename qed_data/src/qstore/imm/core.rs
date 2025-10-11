use crate::qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
    contract::{ContractCodeDefinition, QEDContractLeaf},
    user::QEDUserLeaf,
};
use kvq::traits::KVQBinaryStore;
use qed_core::{config::network_constants::{GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::hash::merkle::{core::{DeltaMerkleProofCore, MerkleProofCore}, spiderman::SpidermanUpdateProof};

use crate::{
    config::store_config::{
        CheckpointLeafTableStore, CheckpointTreeStore, ContractCodeTableStore, ContractFunctionTreeStore, ContractLeafTableStore, ContractTreeStore, DepositTreeStore, L2BlockStateTableStore, QEDFelt, UserContractTreeStore, UserLeafTableStore, UserRegistrationTreeStore, UserTreeStore, WithdrawalTreeStore, MAX_CHECKPOINT
    },
    models::{
        checkpoint::{
            block_state::{L2BlockStatesModelCore, L2BlockStatesModelReaderCore},
            checkpoint_leaf::{QEDCheckpointLeafModelCore, QEDCheckpointLeafModelReaderCore},
        },
        contract::{
            contract_code::{ContractCodeModelCore, ContractCodeModelReaderCore},
            contract_leaf::{ContractLeafModelCore, ContractLeafModelReaderCore},
        },
        kvq_merkle::model::{
            KVQFixedConfigMerkleTreeModelCore, KVQFixedConfigMerkleTreeModelReaderCore, KVQMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore
        },
        user::{
            contract_state_tree::UserContractStateTreeId,
            user_leaf::{UserLeafModelCore, UserLeafModelReaderCore},
        },
    },
    traits::qdatastore::{
        qmetadata::{QMetaDataStoreReaderSync, QMetaDataStoreWriterSync},
        qtreedata::{
            QEDComboDataStoreReaderSync, QEDComboDataStoreReaderWriterSync,
            QEDComboDataStoreWriterSync, QTreeDataStoreReaderSync, QTreeDataStoreWriterSync,
        },
    },
};

type F = QEDFelt;

#[maybe_async::maybe_async(?Send)]
impl<T: KVQBinaryStore> QMetaDataStoreReaderSync<F> for T {
    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        UserLeafTableStore::<T>::get_user_by_id(self, checkpoint_id, user_id)
    }

    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>> {
        ContractLeafTableStore::<T>::get_contract_by_id(self, MAX_CHECKPOINT, contract_id)
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        CheckpointLeafTableStore::<T>::get_checkpoint_leaf_by_id(self, checkpoint_id)
    }

    async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<ContractCodeDefinition> {
        ContractCodeTableStore::<T>::get_contract_code_by_id(self, MAX_CHECKPOINT, contract_id)
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        L2BlockStateTableStore::<T>::get_block_state_by_id(self, checkpoint_id)
    }

    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        L2BlockStateTableStore::<T>::get_latest_block_state(self)
    }
}

impl<T: KVQBinaryStore> QMetaDataStoreWriterSync<F> for T {
    fn set_user_leaf_data(&self, checkpoint_id: u64, leaf_data: &QEDUserLeaf<F>) -> anyhow::Result<()> {
        UserLeafTableStore::<T>::set_user_ref(self, checkpoint_id, leaf_data)
    }

    fn set_contract_leaf_data(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        ContractLeafTableStore::<T>::set_contract_ref(self, checkpoint_id, contract_id, leaf_data)
    }

    fn set_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
        leaf_data: &QEDCheckpointLeaf<F>,
    ) -> anyhow::Result<()> {
        CheckpointLeafTableStore::<T>::set_checkpoint_leaf_ref(self, checkpoint_id, leaf_data)
    }

    fn set_contract_code_definition(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        ContractCodeTableStore::<T>::set_contract_code_ref(
            self,
            checkpoint_id,
            contract_id,
            definition,
        )
    }

    fn set_l2_block_state(&self, block_state: &QEDL2BlockState) -> anyhow::Result<()> {
        L2BlockStateTableStore::<T>::set_block_state_ref(self, block_state)
    }
}

#[maybe_async::maybe_async(?Send)]
impl<T: KVQBinaryStore> QTreeDataStoreReaderSync<F> for T {
    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractStateTreeId::<T>::new(user_id, contract_id, height).get_leaf_value_ucs(
            self,
            checkpoint_id,
            leaf_id,
        )
    }

    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserContractStateTreeId::<T>::new(user_id, contract_id, height).get_leaf_ucs(
            self,
            checkpoint_id,
            leaf_id,
        )
    }

    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::<T>::get_root_fc(self, checkpoint_id, user_id)
    }

    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserContractTreeStore::<T>::get_leaf_sfc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, user_id.into())
    }

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<T>::get_leaf_fc(self, checkpoint_id, user_id.into())
    }

    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<T>::get_sub_tree_proof_fc(
            self,
            checkpoint_id,
            root_level,
            leaf_level,
            leaf_index,
        )
    }

    async fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractFunctionTreeStore::<T>::get_root_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractFunctionTreeStore::<T>::get_leaf_value_fc(
            self,
            checkpoint_id,
            contract_id.into(),
            function_id.into(),
        )
    }

    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        // NOTE: two leaves per function
        ContractFunctionTreeStore::<T>::get_leaf_sfc(
            self,
            checkpoint_id,
            contract_id.into(),
            (function_id as u64) * 2u64,
        )
    }

    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::<T>::get_leaf_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, deposit_id.into())
    }

    async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::<T>::get_leaf_fc(self, checkpoint_id, deposit_id.into())
    }

    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, withdrawal_id.into())
    }

    async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::<T>::get_leaf_fc(self, checkpoint_id, withdrawal_id.into())
    }

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::<T>::get_root_fc(self, MAX_CHECKPOINT)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, leaf_checkpoint_id.into())
    }

    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::<T>::get_leaf_fc(self, checkpoint_id, leaf_checkpoint_id.into())
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::<T>::get_root_fc(self, checkpoint_id)
    }

    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::<T>::get_leaf_value_fc(self, checkpoint_id, leaf_index)

    }

    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserRegistrationTreeStore::<T>::get_leaf_fc(self, checkpoint_id, leaf_index)
    }
}

impl<T: KVQBinaryStore> QTreeDataStoreWriterSync<F> for T {
    fn set_user_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserContractStateTreeId::<T>::new(user_id, contract_id, height).set_leaf_ucs(
            self,
            checkpoint_id,
            leaf_id,
            leaf_hash,
        )
    }

    fn set_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserContractTreeStore::<T>::set_leaf_sfc(
            self,
            checkpoint_id,
            user_id,
            contract_id.into(),
            leaf_hash,
        )
    }

    fn set_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<T>::set_leaf_fc(self, checkpoint_id, user_id, leaf_hash)
    }

    fn set_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::<T>::set_leaf_fc(self, checkpoint_id, deposit_id, leaf_hash)
    }

    fn set_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::<T>::set_leaf_fc(self, checkpoint_id, withdrawal_id, leaf_hash)
    }

    // note that each function has two leaves -- left is the hash of the verifier key and right is [method_id, (num_outputs<<32)|num_inputs, 0, 0]
    fn set_contract_function_whitelist(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaves: &[QHashOut<F>],
    ) -> anyhow::Result<QHashOut<F>> {
        let mut root = QHashOut::from_values(0, 0, 0, 0);
        for (i, leaf) in leaves.iter().enumerate() {
            root = ContractFunctionTreeStore::<T>::set_leaf_sfc(
                self,
                checkpoint_id,
                contract_id,
                i as u64,
                *leaf,
            )?
            .new_root;
        }
        Ok(root)
    }

    fn set_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::<T>::set_leaf_fc(self, checkpoint_id, contract_id, leaf_hash)
    }

    fn set_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::<T>::set_leaf_fc(self, checkpoint_id, checkpoint_id, leaf_hash)
    }

    fn batch_append_user_registration_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)>{
        UserRegistrationTreeStore::<T>::append_leaves_spider_man(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserRegistrationTreeStore::<T>::new_leaf_key_fc(checkpoint_id, start_leaf_index),
            sub_tree_height,
            leaf_hashes,
        )
    }

    fn batch_append_contract_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<(Vec<usize>, Vec<SpidermanUpdateProof<QHashOut<F>>>)>{
        ContractTreeStore::<T>::append_leaves_spider_man(
            self,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            &ContractTreeStore::<T>::new_leaf_key_fc(checkpoint_id, start_leaf_index),
            sub_tree_height,
            leaf_hashes,
        )
    }
}

impl<T: KVQBinaryStore> QEDComboDataStoreWriterSync<F> for T {}

#[maybe_async::maybe_async]
impl<T: KVQBinaryStore> QEDComboDataStoreReaderSync<F> for T {}

#[maybe_async::maybe_async]
impl<T: KVQBinaryStore> QEDComboDataStoreReaderWriterSync<F> for T {}
