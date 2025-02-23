use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
    contract::{ContractCodeDefinition, QEDContractLeaf},
    user::QEDUserLeaf,
};

use crate::traits::qdatastore::qtreedata::QEDComboDataStoreReaderSync;

use super::{
    cmd::{
        QSRCmdGetCheckpointLeafData, QSRCmdGetContractCodeDefinition, QSRCmdGetContractLeafData,
        QSRCmdGetL2BlockState, QSRCmdGetUserLeafData, QSRHashCmd, QSRMerkleCmd,
    },
    cmd_processor::{
        QEDReadCommandBatchInput, QEDReadCommandBatchOutput, QEDReadCommandProcessorSync,
    },
};

impl<F: RichField, R: QEDComboDataStoreReaderSync<F>> QEDReadCommandProcessorSync<F> for R {
    fn resolve_batch(
        &self,
        input: &QEDReadCommandBatchInput,
    ) -> anyhow::Result<QEDReadCommandBatchOutput<F>> {
        Ok(QEDReadCommandBatchOutput {
            get_user_leaf: input
                .get_user_leaf
                .iter()
                .map(|x| self.resolve_get_user_leaf(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_contract_leaf: input
                .get_contract_leaf
                .iter()
                .map(|x| self.resolve_get_contract_leaf(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_contract_code: input
                .get_contract_code
                .iter()
                .map(|x| self.resolve_get_contract_code(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_checkpoint_leaf: input
                .get_checkpoint_leaf
                .iter()
                .map(|x| self.resolve_get_checkpoint_leaf(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_l2_block_state: input
                .get_l2_block_state
                .iter()
                .map(|x| self.resolve_get_l2_block_state(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_merkle_proof: input
                .get_merkle_proof
                .iter()
                .map(|x| self.resolve_get_merkle_proof(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
            get_hash: input
                .get_hash
                .iter()
                .map(|x| self.resolve_get_hash(x))
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }

    fn resolve_get_hash(&self, input: &QSRHashCmd) -> anyhow::Result<QHashOut<F>> {
        match input {
            QSRHashCmd::GetUserContractStateTreeRoot(c) => {
                self.get_user_contract_tree_root(c.checkpoint_id, c.user_id)
            }
            QSRHashCmd::GetUserContractStateTreeLeafHash(c) => self
                .get_user_contract_state_tree_leaf_hash(
                    c.checkpoint_id,
                    c.user_id,
                    c.contract_id,
                    c.height,
                    c.leaf_id,
                ),
            QSRHashCmd::GetUserContractTreeRoot(c) => {
                self.get_user_contract_tree_root(c.checkpoint_id, c.user_id)
            }
            QSRHashCmd::GetUserContractTreeLeafHash(c) => {
                self.get_user_contract_tree_leaf_hash(c.checkpoint_id, c.user_id, c.contract_id)
            }
            QSRHashCmd::GetUserTreeRoot(c) => self.get_user_tree_root(c.checkpoint_id),
            QSRHashCmd::GetUserTreeLeafHash(c) => {
                self.get_user_tree_leaf_hash(c.checkpoint_id, c.user_id)
            }
            QSRHashCmd::GetContractFunctionTreeRoot(c) => {
                self.get_contract_function_tree_root(c.checkpoint_id, c.contract_id)
            }
            QSRHashCmd::GetContractFunctionTreeLeafHash(c) => self
                .get_contract_function_tree_leaf_hash(
                    c.checkpoint_id,
                    c.contract_id,
                    c.function_id,
                ),
            QSRHashCmd::GetContractTreeRoot(c) => self.get_contract_tree_root(c.checkpoint_id),
            QSRHashCmd::GetContractTreeLeafHash(c) => {
                self.get_contract_tree_leaf_hash(c.checkpoint_id, c.contract_id)
            }
            QSRHashCmd::GetDepositTreeRoot(c) => self.get_deposit_tree_root(c.checkpoint_id),
            QSRHashCmd::GetDepositTreeLeafHash(c) => {
                self.get_deposit_tree_leaf_hash(c.checkpoint_id, c.deposit_id)
            }
            QSRHashCmd::GetWithdrawalTreeRoot(c) => self.get_withdrawal_tree_root(c.checkpoint_id),
            QSRHashCmd::GetWithdrawalTreeLeafHash(c) => {
                self.get_withdrawal_tree_leaf_hash(c.checkpoint_id, c.withdrawal_id)
            }
            QSRHashCmd::GetCheckpointTreeRoot(c) => self.get_checkpoint_tree_root(c.checkpoint_id),
            QSRHashCmd::GetCheckpointTreeLeafHash(c) => {
                self.get_checkpoint_tree_leaf_hash(c.checkpoint_id, c.leaf_checkpoint_id)
            }
            QSRHashCmd::GetUserRegistrationTreeRoot(c) => self.get_user_registration_tree_root(c.checkpoint_id),
            QSRHashCmd::GetUserRegistrationTreeLeafHash(c) => self.get_user_registration_tree_leaf_hash(c.checkpoint_id, c.leaf_index),
        }
    }

    fn resolve_get_merkle_proof(
        &self,
        input: &QSRMerkleCmd,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        match input {
            QSRMerkleCmd::GetUserContractStateTreeMerkleProof(c) => self
                .get_user_contract_state_tree_merkle_proof(
                    c.checkpoint_id,
                    c.user_id,
                    c.contract_id,
                    c.height,
                    c.leaf_id,
                ),
            QSRMerkleCmd::GetUserContractTreeMerkleProof(c) => {
                self.get_user_contract_tree_merkle_proof(c.checkpoint_id, c.user_id, c.contract_id)
            }
            QSRMerkleCmd::GetUserTreeMerkleProof(c) => {
                self.get_user_tree_merkle_proof(c.checkpoint_id, c.user_id)
            }
            QSRMerkleCmd::GetContractFunctionTreeMerkleProof(c) => self
                .get_contract_function_tree_merkle_proof(
                    c.checkpoint_id,
                    c.contract_id,
                    c.function_id,
                ),
            QSRMerkleCmd::GetContractTreeMerkleProof(c) => {
                self.get_contract_tree_merkle_proof(c.checkpoint_id, c.contract_id)
            }
            QSRMerkleCmd::GetDepositTreeMerkleProof(c) => {
                self.get_deposit_tree_merkle_proof(c.checkpoint_id, c.deposit_id)
            }
            QSRMerkleCmd::GetWithdrawalTreeMerkleProof(c) => {
                self.get_withdrawal_tree_merkle_proof(c.checkpoint_id, c.withdrawal_id)
            }
            QSRMerkleCmd::GetCheckpointTreeMerkleProof(c) => {
                self.get_checkpoint_tree_merkle_proof(c.checkpoint_id, c.leaf_checkpoint_id)
            }
            QSRMerkleCmd::GetUserRegistrationTreeMerkleProof(c) => {
                self.get_user_registration_tree_merkle_proof(c.checkpoint_id, c.leaf_index)
            },
        }
    }

    fn resolve_get_user_leaf(
        &self,
        input: &QSRCmdGetUserLeafData,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        self.get_user_leaf_data(input.checkpoint_id, input.user_id)
    }

    fn resolve_get_contract_leaf(
        &self,
        input: &QSRCmdGetContractLeafData,
    ) -> anyhow::Result<QEDContractLeaf<F>> {
        self.get_contract_leaf_data(input.contract_id)
    }

    fn resolve_get_contract_code(
        &self,
        input: &QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<ContractCodeDefinition> {
        self.get_contract_code_definition(input.contract_id)
    }

    fn resolve_get_checkpoint_leaf(
        &self,
        input: &QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data(input.checkpoint_id)
    }

    fn resolve_get_l2_block_state(
        &self,
        input: &QSRCmdGetL2BlockState,
    ) -> anyhow::Result<QEDL2BlockState> {
        self.get_l2_block_state(input.checkpoint_id)
    }
    
    fn resolve_get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        self.get_latest_l2_block_state()
    }
}
