use kvq::traits::KVQBinaryStore;
use plonky2::field::types::PrimeField64;
use qed_core::{config::network_constants::{GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::hash::merkle::{core::{DeltaMerkleProofCore, MerkleProofCore}, spiderman::SpidermanUpdateProof};
use qed_data::qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
    contract::{ContractCodeDefinition, QEDContractLeaf},
    user::QEDUserLeaf,
};

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

pub trait QEDStorageAdapterImmutable: KVQBinaryStore {}
impl<T: KVQBinaryStore> QEDStorageAdapterImmutable for T {}

pub trait QEDStorageAdapterImmutableAsync: kvq::traits::KVQBinaryStoreAsync {}
impl<T: kvq::traits::KVQBinaryStoreAsync> QEDStorageAdapterImmutableAsync for T {}
type F = QEDFelt;

#[maybe_async::maybe_async(?Send)]
impl<T: QEDStorageAdapterImmutable + Sync> QMetaDataStoreReaderSync<F> for T {
    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        UserLeafTableStore::get_user_by_id(self, checkpoint_id, user_id)
    }

    async fn get_user_leaf_data_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        UserLeafTableStore::get_user_by_id(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        )
    }

    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>> {
        ContractLeafTableStore::get_contract_by_id(self, MAX_CHECKPOINT, contract_id)
    }

    async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>> {
        ContractLeafTableStore::get_contract_by_id(
            self,
            MAX_CHECKPOINT,
            contract_id.to_canonical_u64(),
        )
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        CheckpointLeafTableStore::get_checkpoint_leaf_by_id(self, checkpoint_id)
    }

    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        CheckpointLeafTableStore::get_checkpoint_leaf_by_id(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<ContractCodeDefinition> {
        ContractCodeTableStore::get_contract_code_by_id(self, MAX_CHECKPOINT, contract_id)
    }

    async fn get_contract_code_definition_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<ContractCodeDefinition> {
        ContractCodeTableStore::get_contract_code_by_id(
            self,
            MAX_CHECKPOINT,
            contract_id.to_canonical_u64(),
        )
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        L2BlockStateTableStore::get_block_state_by_id(self, checkpoint_id)
    }

    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        L2BlockStateTableStore::get_block_state_by_id(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        L2BlockStateTableStore::get_latest_block_state(self)
    }
}

impl<T: QEDStorageAdapterImmutable> QMetaDataStoreWriterSync<F> for T {
    fn set_user_leaf_data(&self, checkpoint_id: u64, leaf_data: &QEDUserLeaf<F>) -> anyhow::Result<()> {
        UserLeafTableStore::set_user_ref(self, checkpoint_id, leaf_data)
    }

    fn set_contract_leaf_data(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        ContractLeafTableStore::set_contract_ref(self, checkpoint_id, contract_id, leaf_data)
    }

    fn set_contract_leaf_data_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        ContractLeafTableStore::set_contract_ref(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_data,
        )
    }

    fn set_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
        leaf_data: &QEDCheckpointLeaf<F>,
    ) -> anyhow::Result<()> {
        CheckpointLeafTableStore::set_checkpoint_leaf_ref(self, checkpoint_id, leaf_data)
    }

    fn set_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
        leaf_data: &QEDCheckpointLeaf<F>,
    ) -> anyhow::Result<()> {
        CheckpointLeafTableStore::set_checkpoint_leaf_ref(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_data,
        )
    }

    fn set_contract_code_definition(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        ContractCodeTableStore::set_contract_code_ref(
            self,
            checkpoint_id,
            contract_id,
            definition,
        )
    }

    fn set_contract_code_definition_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        ContractCodeTableStore::set_contract_code_ref(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            definition,
        )
    }

    fn set_l2_block_state(&self, block_state: &QEDL2BlockState) -> anyhow::Result<()> {
        L2BlockStateTableStore::set_block_state_ref(self, block_state)
    }
}

#[maybe_async::maybe_async(?Send)]
impl<T: QEDStorageAdapterImmutable + Sync> QTreeDataStoreReaderSync<F> for T {
    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::get_leaf_value_fc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_contract_state_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_state_tree_root(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractStateTreeId::new(user_id, contract_id, height).get_leaf_value_ucs(
            self,
            checkpoint_id,
            leaf_id,
        )
    }

    async fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_state_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        ).await
    }

    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserContractStateTreeId::new(user_id, contract_id, height).get_leaf_ucs(
            self,
            checkpoint_id,
            leaf_id,
        )
    }

    async fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_state_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64(),
        ).await
    }

    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::get_root_fc(self, checkpoint_id, user_id)
    }

    async fn get_user_contract_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_tree_root(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }

    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        UserContractTreeStore::get_leaf_value_fc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_user_contract_tree_leaf_hash(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserContractTreeStore::get_leaf_sfc(self, checkpoint_id, user_id, contract_id.into())
    }

    async fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_contract_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::get_root_fc(self, checkpoint_id)
    }

    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        self.get_user_tree_root(checkpoint_id.to_canonical_u64()).await
    }

    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::get_leaf_value_fc(self, checkpoint_id, user_id.into())
    }

    async fn get_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_user_tree_leaf_hash(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::get_leaf_fc(self, checkpoint_id, user_id.into())
    }

    async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }

    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::get_sub_tree_proof_fc(
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
        ContractFunctionTreeStore::get_root_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_contract_function_tree_root_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        self.get_contract_function_tree_root(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractFunctionTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id,
            contract_id.into(),
            function_id.into(),
        )
    }

    async fn get_contract_function_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractFunctionTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            function_id.to_canonical_u64(),
        )
    }

    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        // NOTE: two leaves per function
        ContractFunctionTreeStore::get_leaf_sfc(
            self,
            checkpoint_id,
            contract_id.into(),
            (function_id as u64) * 2u64,
        )
    }

    async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            function_id.to_canonical_u64() as u32,
        ).await
    }

    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::get_root_fc(self, checkpoint_id)
    }

    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::get_root_fc(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::get_leaf_value_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        ContractTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
        )
    }

    async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::get_leaf_fc(self, checkpoint_id, contract_id.into())
    }

    async fn get_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::get_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
        )
    }

    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::get_root_fc(self, checkpoint_id)
    }

    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::get_root_fc(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::get_leaf_value_fc(self, checkpoint_id, deposit_id.into())
    }

    async fn get_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        DepositTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64(),
        )
    }

    async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::get_leaf_fc(self, checkpoint_id, deposit_id.into())
    }

    async fn get_deposit_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::get_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64(),
        )
    }

    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::get_root_fc(self, checkpoint_id)
    }

    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::get_root_fc(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::get_leaf_value_fc(self, checkpoint_id, withdrawal_id.into())
    }

    async fn get_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        WithdrawalTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64(),
        )
    }

    async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::get_leaf_fc(self, checkpoint_id, withdrawal_id.into())
    }

    async fn get_withdrawal_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::get_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64(),
        )
    }

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::get_root_fc(self, MAX_CHECKPOINT)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::get_root_fc(self, checkpoint_id)
    }

    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::get_root_fc(self, checkpoint_id.to_canonical_u64())
    }

    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::get_leaf_value_fc(self, checkpoint_id, leaf_checkpoint_id.into())
    }

    async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        CheckpointTreeStore::get_leaf_value_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
    }

    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::get_leaf_fc(self, checkpoint_id, leaf_checkpoint_id.into())
    }

    async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::get_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_checkpoint_id.to_canonical_u64(),
        )
    }
    
    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::get_root_fc(self, checkpoint_id)
    }
    
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::get_root_fc(self, checkpoint_id.to_canonical_u64())
    }
    
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::get_leaf_value_fc(self, checkpoint_id, leaf_index)

    }
    
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        UserRegistrationTreeStore::get_leaf_value_fc(self, checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64())
    }
    
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserRegistrationTreeStore::get_leaf_fc(self, checkpoint_id, leaf_index)
    }
    
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserRegistrationTreeStore::get_leaf_fc(self, checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64())
    }
}

impl<T: QEDStorageAdapterImmutable> QTreeDataStoreWriterSync<F> for T {
    fn set_user_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserContractStateTreeId::new(user_id, contract_id, height).set_leaf_ucs(
            self,
            checkpoint_id,
            leaf_id,
            leaf_hash,
        )
    }

    fn set_user_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserContractStateTreeId::new(
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
        )
        .set_leaf_ucs(
            self,
            checkpoint_id.to_canonical_u64(),
            leaf_id.to_canonical_u64(),
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
        UserContractTreeStore::set_leaf_sfc(
            self,
            checkpoint_id,
            user_id,
            contract_id.into(),
            leaf_hash,
        )
    }

    fn set_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserContractTreeStore::set_leaf_sfc(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserTreeStore::set_leaf_fc(self, checkpoint_id, user_id, leaf_hash)
    }

    fn set_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        UserTreeStore::set_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::set_leaf_fc(self, checkpoint_id, deposit_id, leaf_hash)
    }

    fn set_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        DepositTreeStore::set_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            deposit_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::set_leaf_fc(self, checkpoint_id, withdrawal_id, leaf_hash)
    }

    fn set_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        WithdrawalTreeStore::set_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            withdrawal_id.to_canonical_u64(),
            leaf_hash,
        )
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
            root = ContractFunctionTreeStore::set_leaf_sfc(
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

    fn set_contract_function_whitelist_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaves: &[QHashOut<F>],
    ) -> anyhow::Result<QHashOut<F>> {
        self.set_contract_function_whitelist(
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaves,
        )
    }

    fn set_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::set_leaf_fc(self, checkpoint_id, contract_id, leaf_hash)
    }

    fn set_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        ContractTreeStore::set_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            contract_id.to_canonical_u64(),
            leaf_hash,
        )
    }

    fn set_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::set_leaf_fc(self, checkpoint_id, checkpoint_id, leaf_hash)
    }

    fn set_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        CheckpointTreeStore::set_leaf_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            checkpoint_id.to_canonical_u64(),
            leaf_hash,
        )
    }
    
    fn batch_append_user_registration_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>{
        UserRegistrationTreeStore::append_leaves_spider_man(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserRegistrationTreeStore::<Self>::new_leaf_key_fc(checkpoint_id, start_leaf_index),
            sub_tree_height,
            leaf_hashes,
        )
    }
    
    fn batch_append_user_registration_tree_f(&self, checkpoint_id: F, start_leaf_index: F, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>{
        UserRegistrationTreeStore::append_leaves_spider_man(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserRegistrationTreeStore::<Self>::new_leaf_key_fc(checkpoint_id.to_canonical_u64(), start_leaf_index.to_canonical_u64()),
            sub_tree_height,
            leaf_hashes,
        )
    }

    fn batch_append_contract_tree(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>{
        ContractTreeStore::append_leaves_spider_man(
            self,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            &ContractTreeStore::<Self>::new_leaf_key_fc(checkpoint_id, start_leaf_index),
            sub_tree_height,
            leaf_hashes,
        )
    }
}

impl<T: QEDStorageAdapterImmutable> QEDComboDataStoreWriterSync<F> for T {}

#[maybe_async::maybe_async]
impl<T: QEDStorageAdapterImmutable + Sync> QEDComboDataStoreReaderSync<F> for T {}

#[maybe_async::maybe_async]
impl<T: QEDStorageAdapterImmutable + Sync> QEDComboDataStoreReaderWriterSync<F> for T {}
