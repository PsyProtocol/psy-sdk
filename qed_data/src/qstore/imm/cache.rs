use std::collections::HashMap;

use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use crate::qdata::{checkpoint::{QEDCheckpointLeaf, QEDL2BlockState}, contract::{ContractCodeDefinition, QEDContractLeaf}, user::QEDUserLeaf};


use super::{cmd::{QSRCmdGetCheckpointLeafData, QSRCmdGetContractCodeDefinition, QSRCmdGetContractLeafData, QSRCmdGetL2BlockState, QSRCmdGetUserLeafData, QSRHashCmd, QSRMerkleCmd}, cmd_processor::{QEDReadCommandBatchInput, QEDReadCommandBatchOutput, QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}};

#[derive(Clone, Debug)]
pub struct QEDCmdDataStoreCache<F: RichField> {
    pub user_leaf_cache: HashMap<QSRCmdGetUserLeafData, QEDUserLeaf<F>>,
    pub contract_leaf_cache: HashMap<u64, QEDContractLeaf<F>>,
    pub checkpoint_leaf_cache: HashMap<u64, QEDCheckpointLeaf<F>>,
    pub contract_code_definition_cache: HashMap<u64, ContractCodeDefinition>,
    pub l2_block_state_cache: HashMap<u64, QEDL2BlockState>,
    pub hash_cmd_cache: HashMap<QSRHashCmd, QHashOut<F>>,
    pub merkle_cmd_cache: HashMap<QSRMerkleCmd, MerkleProofCore<QHashOut<F>>>,
}
impl<F: RichField> QEDCmdDataStoreCache<F> {
    pub fn new() -> Self {
        Self {
            user_leaf_cache: HashMap::new(),
            contract_leaf_cache: HashMap::new(),
            checkpoint_leaf_cache: HashMap::new(),
            contract_code_definition_cache: HashMap::new(),
            l2_block_state_cache: HashMap::new(),
            hash_cmd_cache: HashMap::new(),
            merkle_cmd_cache: HashMap::new(),
        }
    }

}
#[derive(Debug, Clone)]
pub struct QEDCmdStoreWithCache<F: RichField, S: QEDReadCommandProcessorSync<F>>{
    pub last_checkpoint: F,
    pub last_checkpoint_u64: u64,
    pub cache: QEDCmdDataStoreCache<F>,
    pub read_store: S,
}


impl <F: RichField, S: QEDReadCommandProcessorSync<F>> QEDCmdStoreWithCache<F, S> {
    pub fn new(last_checkpoint_u64: u64, read_store: S) -> Self {
        Self {
            last_checkpoint: F::from_noncanonical_u64(last_checkpoint_u64),
            last_checkpoint_u64,
            cache: QEDCmdDataStoreCache::new(),
            read_store,
        }
    }
    pub fn clear_cache_mut(&mut self){
        self.cache = QEDCmdDataStoreCache::new();
    }
}

#[maybe_async::maybe_async]
impl<F: RichField, S: QEDReadCommandProcessorSync<F> + Send> QEDReadCommandProcessorSyncMut<F> for QEDCmdStoreWithCache<F, S> {
    async fn resolve_batch_mut(&mut self, input: &QEDReadCommandBatchInput) -> anyhow::Result<QEDReadCommandBatchOutput<F>> {
        let filtered_get = QEDReadCommandBatchInput {
            get_user_leaf: input.get_user_leaf.iter().filter(|x| !self.cache.user_leaf_cache.contains_key(x)).cloned().collect(),
            get_contract_leaf: input.get_contract_leaf.iter().filter(|x| !self.cache.contract_leaf_cache.contains_key(&x.contract_id)).cloned().collect(),
            get_checkpoint_leaf: input.get_checkpoint_leaf.iter().filter(|x| !self.cache.checkpoint_leaf_cache.contains_key(&x.checkpoint_id)).cloned().collect(),
            get_contract_code: input.get_contract_code.iter().filter(|x| !self.cache.contract_code_definition_cache.contains_key(&x.contract_id)).cloned().collect(),
            get_l2_block_state: input.get_l2_block_state.iter().filter(|x| !self.cache.l2_block_state_cache.contains_key(&x.checkpoint_id)).cloned().collect(),
            get_hash: input.get_hash.iter().filter(|x| !self.cache.hash_cmd_cache.contains_key(x)).cloned().collect(),
            get_merkle_proof: input.get_merkle_proof.iter().filter(|x| !self.cache.merkle_cmd_cache.contains_key(x)).cloned().collect(),
        };
        let base_output = self.read_store.resolve_batch(&filtered_get).await?;
        self.cache.user_leaf_cache.extend(filtered_get.get_user_leaf.iter().zip(base_output.get_user_leaf.iter()).map(|(i, x)| (i.clone(), x.clone())));
        self.cache.contract_leaf_cache.extend(filtered_get.get_contract_leaf.iter().map(|x|x.contract_id).zip(base_output.get_contract_leaf.clone()));
        self.cache.checkpoint_leaf_cache.extend(filtered_get.get_checkpoint_leaf.iter().map(|x|x.checkpoint_id).zip(base_output.get_checkpoint_leaf.clone()));
        self.cache.contract_code_definition_cache.extend(filtered_get.get_contract_code.iter().map(|x|x.contract_id).zip(base_output.get_contract_code.clone()));
        self.cache.l2_block_state_cache.extend(filtered_get.get_l2_block_state.iter().map(|x|x.checkpoint_id).zip(base_output.get_l2_block_state.clone()));

        self.cache.hash_cmd_cache.extend(filtered_get.get_hash.iter().zip(base_output.get_hash.iter()).map(|(i, x)| (i.clone(), x.clone())));
        self.cache.merkle_cmd_cache.extend(filtered_get.get_merkle_proof.iter().zip(base_output.get_merkle_proof.iter()).map(|(i, x)| (i.clone(), x.clone())));

        Ok(QEDReadCommandBatchOutput {
            get_user_leaf: input.get_user_leaf.iter().map(|x| self.cache.user_leaf_cache.get(x).unwrap().clone()).collect(),
            get_contract_leaf: input.get_contract_leaf.iter().map(|x| self.cache.contract_leaf_cache.get(&x.contract_id).unwrap().clone()).collect(),
            get_checkpoint_leaf: input.get_checkpoint_leaf.iter().map(|x| self.cache.checkpoint_leaf_cache.get(&x.checkpoint_id).unwrap().clone()).collect(),
            get_contract_code: input.get_contract_code.iter().map(|x| self.cache.contract_code_definition_cache.get(&x.contract_id).unwrap().clone()).collect(),
            get_l2_block_state: input.get_l2_block_state.iter().map(|x| self.cache.l2_block_state_cache.get(&x.checkpoint_id).unwrap().clone()).collect(),
            get_hash: input.get_hash.iter().map(|x| self.cache.hash_cmd_cache.get(x).unwrap().clone()).collect(),
            get_merkle_proof: input.get_merkle_proof.iter().map(|x| self.cache.merkle_cmd_cache.get(x).unwrap().clone()).collect(),
        })

    }

    async fn resolve_get_hash_mut(&mut self, input: &QSRHashCmd) -> anyhow::Result<QHashOut<F>> {
        if self.cache.hash_cmd_cache.contains_key(input) {
            Ok(self.cache.hash_cmd_cache.get(input).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_hash(input).await?;
            self.cache.hash_cmd_cache.insert(input.clone(), result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_merkle_proof_mut(&mut self, input: &QSRMerkleCmd) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        if self.cache.merkle_cmd_cache.contains_key(input) {
            Ok(self.cache.merkle_cmd_cache.get(input).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_merkle_proof(input).await?;
            self.cache.merkle_cmd_cache.insert(input.clone(), result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_user_leaf_mut(&mut self, input: &QSRCmdGetUserLeafData) -> anyhow::Result<QEDUserLeaf<F>> {
        if self.cache.user_leaf_cache.contains_key(input) {
            Ok(self.cache.user_leaf_cache.get(input).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_user_leaf(input).await?;
            self.cache.user_leaf_cache.insert(input.clone(), result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_contract_leaf_mut(&mut self, input: &QSRCmdGetContractLeafData) -> anyhow::Result<QEDContractLeaf<F>> {
        if self.cache.contract_leaf_cache.contains_key(&input.contract_id) {
            Ok(self.cache.contract_leaf_cache.get(&input.contract_id).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_contract_leaf(input).await?;
            self.cache.contract_leaf_cache.insert(input.contract_id, result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_contract_code_mut(&mut self, input: &QSRCmdGetContractCodeDefinition) -> anyhow::Result<ContractCodeDefinition> {
        if self.cache.contract_code_definition_cache.contains_key(&input.contract_id) {
            Ok(self.cache.contract_code_definition_cache.get(&input.contract_id).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_contract_code(input).await?;
            self.cache.contract_code_definition_cache.insert(input.contract_id, result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_checkpoint_leaf_mut(&mut self, input: &QSRCmdGetCheckpointLeafData) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        if self.cache.checkpoint_leaf_cache.contains_key(&input.checkpoint_id) {
            Ok(self.cache.checkpoint_leaf_cache.get(&input.checkpoint_id).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_checkpoint_leaf(input).await?;
            self.cache.checkpoint_leaf_cache.insert(input.checkpoint_id, result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_l2_block_state_mut(&mut self, input: &QSRCmdGetL2BlockState) -> anyhow::Result<QEDL2BlockState> {
        if self.cache.l2_block_state_cache.contains_key(&input.checkpoint_id) {
            Ok(self.cache.l2_block_state_cache.get(&input.checkpoint_id).unwrap().clone())
        }else{
            let result = self.read_store.resolve_get_l2_block_state(input).await?;
            self.cache.l2_block_state_cache.insert(input.checkpoint_id, result.clone());
            Ok(result)
        }
    }

    async fn resolve_get_latest_l2_block_state_mut(&mut self) -> anyhow::Result<QEDL2BlockState> {
        self.read_store.resolve_get_latest_l2_block_state().await
    }
}
