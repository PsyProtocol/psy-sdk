use kvq::memory::{arc_imm::KVQArcImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore};
use plonky2::{field::{goldilocks_field::GoldilocksField, types::PrimeField64}, hash::hash_types::RichField};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore};
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use serde::{Deserialize, Serialize};

use crate::{config::store_config::QEDDeltaMerkleProof, models::{kvq_merkle::model::KVQMerkleTreeModelCore, user::contract_state_tree::UserContractStateTreeId}, store::imm::{cache::QEDCmdStoreWithCache, cmd::{QSRCmdGetContractLeafData, QSRMerkleCmd, QSRMerkleCmdGetUserContractStateTreeMerkleProof}, cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}, traits::qdatastore::qtreedata::QEDComboDataStoreReaderSync};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalTransactionCallInfo<F: RichField> {
    pub start_checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub function_id: F,
    pub caller_contract_id: F,
    pub flat_args: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalTransactionStateHints<F: RichField> {
    pub call_info: QEDLocalTransactionCallInfo<F>,
    pub start_contract_state_tree_root: QHashOut<F>,
    pub contract_state_tree_height: u16,

    pub start_checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub function_id: F,
    pub caller_contract_id: F,
    pub flat_args: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalStateSet<F: RichField> {
    pub contract: F,
    pub slot: F,
    pub contract_state_transition_proof: DeltaMerkleProofCore<QHashOut<F>>,
}


#[derive(Clone, Debug)]
pub struct QEDLocalProvingSessionStore<F: RichField, R: QEDReadCommandProcessorSync<F>> {
    pub cmd_store: QEDCmdStoreWithCache<F, R>,
    pub state_tree_store: KVQSimpleMemoryBackingStore,
    //pub delta_merkle_proof_cache: Vec<QEDLocalStateSet<F>>,


    pub start_checkpoint: F,
    pub write_checkpoint: F,
    pub user_id: F,
    pub current_contract_id: F,

    pub start_checkpoint_u64: u64,
    pub write_checkpoint_u64: u64,
    pub user_id_u64: u64,
    pub user_id_u32: u32,
    pub nonce: F,
}
impl<F: RichField, R: QEDReadCommandProcessorSync<F>> QEDLocalProvingSessionStore<F, R> {
    pub fn new_at(read_store: R, start_checkpoint: F, user_id: F, contract_id: F, nonce: F) -> Self {
        Self {
            cmd_store: QEDCmdStoreWithCache::new(start_checkpoint.to_canonical_u64(), read_store),
            state_tree_store:  KVQSimpleMemoryBackingStore::new(),
            //delta_merkle_proof_cache: Vec::new(),
            start_checkpoint: start_checkpoint,
            write_checkpoint: start_checkpoint+F::ONE,
            current_contract_id: contract_id,
            user_id,
            start_checkpoint_u64: start_checkpoint.to_canonical_u64(),
            write_checkpoint_u64: start_checkpoint.to_canonical_u64()+1,
            user_id_u64: user_id.to_canonical_u64(),
            user_id_u32: user_id.to_canonical_u64() as u32,
            nonce,
        }
    }
    pub fn new_at_head(read_store: R, user_id: F, contract_id: F, nonce: F) -> anyhow::Result<Self> {
        let start_checkpoint = read_store.resolve_get_latest_l2_block_state()?;

        Ok(Self::new_at(read_store, F::from_noncanonical_u64(start_checkpoint.checkpoint_id), user_id, contract_id, nonce))
    }
    pub fn clear(&mut self) {
        self.cmd_store.clear_cache_mut();
        self.state_tree_store.clear();
        self.write_checkpoint = self.start_checkpoint + F::ONE;
        self.write_checkpoint_u64 = self.start_checkpoint_u64 + 1;
    }
}

type GF = GoldilocksField;
impl<R: QEDReadCommandProcessorSync<GoldilocksField>> QEDLocalProvingSessionStore<GoldilocksField, R> {

    pub fn set_contract_state_slot(&mut self, contract: GF, slot: GF, value: QHashOut<GF>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self.cmd_store.resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData{contract_id: contract.to_canonical_u64()})?.state_tree_height.to_canonical_u64() as u8;
        let id = UserContractStateTreeId::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract.to_canonical_u64() as u32,
            height: state_tree_height,
            leaf_id: slot.to_canonical_u64(),
        }))?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        let dmp = id.set_leaf_ucs(&mut self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64(), value)?;
        /*let cache_value = QEDLocalStateSet{
            contract,
            slot,
            contract_state_transition_proof: dmp.clone(),
        };
        self.delta_merkle_proof_cache.push(cache_value);*/

        Ok(dmp)
       // self.state_tree_store.map.insert((self.user_id_u64, contract, slot), value);
    }
    pub fn get_contract_state_slot(&mut self, contract: GF, slot: GF) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        let state_tree_height = self.cmd_store.resolve_get_contract_leaf_mut(&QSRCmdGetContractLeafData{contract_id: contract.to_canonical_u64()})?.state_tree_height.to_canonical_u64() as u8;
        let id = UserContractStateTreeId::new(self.user_id_u64, contract.to_canonical_u64() as u32, state_tree_height);
        let base_mp = self.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof{
            checkpoint_id: self.start_checkpoint_u64,
            user_id: self.user_id_u64,
            contract_id: contract.to_canonical_u64() as u32,
            height: state_tree_height,
            leaf_id: slot.to_canonical_u64(),
        }))?;
        id.injest_merkle_proof_ucs(&mut self.state_tree_store, self.start_checkpoint_u64, &base_mp)?;
        id.get_leaf_ucs(&self.state_tree_store, self.write_checkpoint_u64, slot.to_canonical_u64())
       // self.state_tree_store.map.insert((self.user_id_u64, contract, slot), value);
    }
}

