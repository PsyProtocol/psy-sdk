use plonky2::hash::hash_types::RichField;
//use qed_core::config::network_constants::QEDTreeConfig;
use crate::qdata::{checkpoint::{QEDCheckpointLeaf, QEDL2BlockState}, contract::{ContractCodeDefinition, QEDContractLeaf}, user::QEDUserLeaf};


#[maybe_async::maybe_async(?Send)]
pub trait QMetaDataStoreReaderSync<F: RichField> {
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QEDUserLeaf<F>>;
    async fn get_user_leaf_data_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QEDUserLeaf<F>> {
        self.get_user_leaf_data(
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
        ).await
    }

    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>> {
        self.get_contract_leaf_data(contract_id.to_canonical_u64()).await
    }

    async  fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    async  fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data(checkpoint_id.to_canonical_u64()).await
    }

    async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition> {
        self.get_contract_code_definition(contract_id.to_canonical_u64()).await
    }

    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        self.get_l2_block_state(checkpoint_id.to_canonical_u64()).await
    }
}

pub trait QMetaDataStoreReaderSyncMut<F: RichField> {
    fn get_user_leaf_data_mut(&mut self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QEDUserLeaf<F>>;
    fn get_user_leaf_data_f_mut(&mut self, checkpoint_id: F, user_id: F) -> anyhow::Result<QEDUserLeaf<F>>;

    fn get_contract_leaf_data_mut(&mut self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    fn get_contract_leaf_data_f_mut(&mut self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;

    fn get_checkpoint_leaf_data_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    fn get_checkpoint_leaf_data_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;

    fn get_contract_code_definition_mut(&mut self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    fn get_contract_code_definition_f_mut(&mut self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;


    fn get_l2_block_state_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    fn get_l2_block_state_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
}

pub trait QMetaDataStoreWriterSync<F: RichField> {
    fn set_user_leaf_data(&self, checkpoint_id: u64, leaf_data: &QEDUserLeaf<F>) -> anyhow::Result<()>;

    fn set_contract_leaf_data(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;
    fn set_contract_leaf_data_f(&self, checkpoint_id: F, contract_id: F, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;

    fn set_checkpoint_leaf_data(&self, checkpoint_id: u64, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()>;
    fn set_checkpoint_leaf_data_f(&self, checkpoint_id: F, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()>;

    fn set_contract_code_definition(&self, checkpoint_id: u64, contract_id: u64, definition: &ContractCodeDefinition) -> anyhow::Result<()>;
    fn set_contract_code_definition_f(&self, checkpoint_id: F, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()>;

    fn set_l2_block_state(&self, block_state: &QEDL2BlockState) -> anyhow::Result<()>;
}
pub trait QMetaDataStoreWriterSyncMut<F: RichField> {
    fn set_user_leaf_data_mut(&mut self, checkpoint_id: u64, leaf_data: &QEDUserLeaf<F>) -> anyhow::Result<()>;

    fn set_contract_leaf_data_mut(&mut self, contract_id: u64, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;
    fn set_contract_leaf_data_f_mut(&mut self, contract_id: F, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;

    fn set_checkpoint_leaf_data_mut(&mut self, checkpoint_id: u64, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()>;
    fn set_checkpoint_leaf_data_f_mut(&mut self, checkpoint_id: F, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()>;

    fn set_contract_code_definition_mut(&mut self, contract_id: u64, definition: &ContractCodeDefinition) -> anyhow::Result<()>;
    fn set_contract_code_definition_f_mut(&mut self, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()>;
    fn set_l2_block_state_mut(&mut self, block_state: &QEDL2BlockState) -> anyhow::Result<()>;

}
