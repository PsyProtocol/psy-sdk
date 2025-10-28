
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore};
use crate::{dpn::proving_session::DPNProvingSessionSimpleMethodCall, qdata::{checkpoint::{QEDCheckpointLeaf, QEDCheckpointLeafStats, QEDCheckpointGlobalStateRoots, QEDL2BlockState}, contract::{ContractCodeDefinition, QEDContractLeaf}, user::QEDUserLeaf}};
use serde::{Deserialize, Serialize};
use ts_rs::TS;


use super::cmd::{QSRCmdGetCheckpointLeafData, QSRCmdGetContractCodeDefinition, QSRCmdGetContractLeafData, QSRCmdGetL2BlockState, QSRCmdGetUserLeafData, QSRHashCmd, QSRMerkleCmd};



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct QEDReadCommandBatchInput {
    pub get_user_leaf: Vec<QSRCmdGetUserLeafData>,
    pub get_contract_leaf: Vec<QSRCmdGetContractLeafData>,
    pub get_contract_code: Vec<QSRCmdGetContractCodeDefinition>,
    pub get_checkpoint_leaf: Vec<QSRCmdGetCheckpointLeafData>,
    pub get_l2_block_state: Vec<QSRCmdGetL2BlockState>,
    pub get_merkle_proof: Vec<QSRMerkleCmd>,
    pub get_hash: Vec<QSRHashCmd>,
}
impl QEDReadCommandBatchInput {
    pub fn new() -> Self {
        Self {
            get_user_leaf: Vec::new(),
            get_contract_leaf: Vec::new(),
            get_contract_code: Vec::new(),
            get_checkpoint_leaf: Vec::new(),
            get_l2_block_state: Vec::new(),
            get_merkle_proof: Vec::new(),
            get_hash: Vec::new(),
        }
    }
    pub fn push_get_user_leaf(&mut self, checkpoint_id: u64, user_id: u64) -> usize {
        let id = self.get_user_leaf.len();
        self.get_user_leaf.push(QSRCmdGetUserLeafData { checkpoint_id, user_id });
        id
    }
    pub fn push_get_contract_leaf(&mut self, contract_id: u64) -> usize {
        let id = self.get_contract_leaf.len();
        self.get_contract_leaf.push(QSRCmdGetContractLeafData { contract_id });
        id
    }
    pub fn push_get_contract_code(&mut self, contract_id: u64) -> usize {
        let id = self.get_contract_code.len();
        self.get_contract_code.push(QSRCmdGetContractCodeDefinition { contract_id });
        id
    }
    pub fn push_get_checkpoint_leaf(&mut self, checkpoint_id: u64) -> usize {
        let id = self.get_checkpoint_leaf.len();
        self.get_checkpoint_leaf.push(QSRCmdGetCheckpointLeafData { checkpoint_id });
        id
    }
    pub fn push_get_l2_block_state(&mut self, checkpoint_id: u64) -> usize {
        let id = self.get_l2_block_state.len();
        self.get_l2_block_state.push(QSRCmdGetL2BlockState { checkpoint_id });
        id
    }
    pub fn push_get_merkle_proof(&mut self, cmd: QSRMerkleCmd) -> usize {
        let id = self.get_merkle_proof.len();
        self.get_merkle_proof.push(cmd);
        id
    }
    pub fn push_get_hash(&mut self, cmd: QSRHashCmd) -> usize {
        let id = self.get_hash.len();
        self.get_hash.push(cmd);
        id
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDReadCommandBatchOutput<F: RichField> {
    pub get_user_leaf: Vec<QEDUserLeaf<F>>,
    pub get_contract_leaf: Vec<QEDContractLeaf<F>>,
    pub get_contract_code: Vec<ContractCodeDefinition>,
    pub get_checkpoint_leaf: Vec<QEDCheckpointLeaf<F>>,
    pub get_l2_block_state: Vec<QEDL2BlockState>,
    pub get_merkle_proof: Vec<MerkleProofCore<QHashOut<F>>>,
    pub get_hash: Vec<QHashOut<F>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DPNReadOtherUserLeafMerkleProof<F: RichField> {
    pub user_tree_proof: MerkleProofCore<QHashOut<F>>,
    pub user_leaf: QEDUserLeaf<F>,

}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DPNReadOtherUserContractStateLeafMerkleProof<F: RichField> {
    pub user_leaf_witness: DPNReadOtherUserLeafMerkleProof<F>,
    pub contract_state_proof: MerkleProofCore<QHashOut<F>>,
    pub state_slot_proofs: Vec<MerkleProofCore<QHashOut<F>>>,
}



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DPNInvokeDeferredMethodCallWitness<F: RichField> {
    pub call_data: DPNProvingSessionSimpleMethodCall<F>,
    pub insertion_proof: DeltaMerkleProofCore<QHashOut<F>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DPNCheckpointLeafStatsWitness<F: RichField> {
    pub checkpoint_leaf_stats: QEDCheckpointLeafStats<F>,
    pub checkpoint_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub checkpoint_historical_proof: MerkleProofCore<QHashOut<F>>, // Proves this checkpoint existed historically
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DPNClearEntireTreeWitness<F: RichField> {
    pub state_tree_height: u64,
    pub zero_hash: QHashOut<F>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub enum DPNStateCmdWitness<F: RichField> {
    MerkleProof(MerkleProofCore<QHashOut<F>>),
    DeltaMerkleProof(DeltaMerkleProofCore<QHashOut<F>>),
    MerkleProofArray(Vec<MerkleProofCore<QHashOut<F>>>),
    DeltaMerkleProofArray(Vec<DeltaMerkleProofCore<QHashOut<F>>>),
    ReadOtherUserContractState(DPNReadOtherUserContractStateLeafMerkleProof<F>),
    InvokeExternalContractFunctionDeferred(DPNInvokeDeferredMethodCallWitness<F>),
    CheckpointLeafStats(DPNCheckpointLeafStatsWitness<F>),
    ClearEntireTree(DPNClearEntireTreeWitness<F>),
    TargetArray(Vec<F>),
    TargetArray2D(Vec<Vec<F>>),
}

impl<F: RichField> DPNStateCmdWitness<F> {

    pub fn get_merkle_proof_ref(&self) -> &MerkleProofCore<QHashOut<F>> {
        match &self {
            DPNStateCmdWitness::MerkleProof(merkle_proof) => {
                merkle_proof
            },
            _ => panic!("get_merkle_proof_ref expects witnesss type to be MerkleProof, but got {:?}",&self),
        }
    }
    pub fn get_delta_merkle_proof_ref(&self) -> &DeltaMerkleProofCore<QHashOut<F>> {
        match &self {
            DPNStateCmdWitness::DeltaMerkleProof(delta_merkle_proof) => {
                delta_merkle_proof
            },
            _ => panic!("get_delta_merkle_proof_ref expects witnesss type to be DeltaMerkleProof, but got {:?}",&self),
        }
    }
    pub fn get_merkle_proof_array_ref(&self) -> &Vec<MerkleProofCore<QHashOut<F>>> {
        match &self {
            DPNStateCmdWitness::MerkleProofArray(merkle_proofs) => {
                merkle_proofs
            },
            _ => panic!("get_merkle_proof_array_ref expects witnesss type to be MerkleProofArray, but got {:?}",&self),
        }
    }
    pub fn get_delta_merkle_proof_array_ref(&self) -> &Vec<DeltaMerkleProofCore<QHashOut<F>>> {
        match &self {
            DPNStateCmdWitness::DeltaMerkleProofArray(delta_merkle_proofs) => {
                delta_merkle_proofs
            },
            _ => panic!("get_delta_merkle_proof_array_ref expects witnesss type to be DeltaMerkleProofArray, but got {:?}",&self),
        }
    }


    pub fn get_read_other_contract_state_ref(&self) -> &DPNReadOtherUserContractStateLeafMerkleProof<F> {
        match &self {
            DPNStateCmdWitness::ReadOtherUserContractState(w) => {
                w
            },
            _ => panic!("get_read_other_contract_state_ref expects witnesss type to be ReadOtherUserContractState, but got {:?}",&self),
        }
    }


    pub fn get_invoke_external_function_deferred_ref(&self) -> &DPNInvokeDeferredMethodCallWitness<F> {
        match &self {
            DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(w) => {
                w
            },
            _ => panic!("get_invoke_external_function_deferred_ref expects witnesss type to be InvokeExternalContractFunctionDeferred, but got {:?}",&self),
        }
    }
    pub fn get_target_array_ref(&self) -> &Vec<F> {
        match &self {
            DPNStateCmdWitness::TargetArray(w) => {
                w
            },
            _ => panic!("get_target_array_ref expects witnesss type to be TargetArray, but got {:?}",&self),
        }
    }
    pub fn get_target_array_2d_ref(&self) -> &Vec<Vec<F>> {
        match &self {
            DPNStateCmdWitness::TargetArray2D(w) => {
                w
            },
            _ => panic!("get_target_array_2d_ref expects witnesss type to be TargetArray2D, but got {:?}",&self),
        }
    }
    pub fn get_checkpoint_leaf_stats_ref(&self) -> &DPNCheckpointLeafStatsWitness<F> {
        match &self {
            DPNStateCmdWitness::CheckpointLeafStats(stats) => {
                stats
            },
            _ => panic!("get_checkpoint_leaf_stats_ref expects witness type to be CheckpointLeafStats, but got {:?}",&self),
        }
    }
    pub fn get_clear_entire_tree_ref(&self) -> &DPNClearEntireTreeWitness<F> {
        match &self {
            DPNStateCmdWitness::ClearEntireTree(witness) => {
                witness
            },
            _ => panic!("get_clear_entire_tree_ref expects witness type to be ClearEntireTree, but got {:?}",&self),
        }
    }
    pub fn get_merkle_proof(self) -> MerkleProofCore<QHashOut<F>> {
        match self {
            DPNStateCmdWitness::MerkleProof(merkle_proof) => {
                merkle_proof
            },
            _ => panic!("get_merkle_proof expects witnesss type to be MerkleProof, but got {:?}",&self),
        }
    }
    pub fn get_delta_merkle_proof(self) -> DeltaMerkleProofCore<QHashOut<F>> {
        match self {
            DPNStateCmdWitness::DeltaMerkleProof(delta_merkle_proof) => {
                delta_merkle_proof
            },
            _ => panic!("get_delta_merkle_proof expects witnesss type to be DeltaMerkleProof, but got {:?}",&self),
        }
    }
    pub fn get_merkle_proof_array(self) -> Vec<MerkleProofCore<QHashOut<F>>> {
        match self {
            DPNStateCmdWitness::MerkleProofArray(merkle_proofs) => {
                merkle_proofs
            },
            _ => panic!("get_merkle_proof_array expects witnesss type to be MerkleProofArray, but got {:?}",&self),
        }
    }
    pub fn get_delta_merkle_proof_array(self) -> Vec<DeltaMerkleProofCore<QHashOut<F>>> {
        match self {
            DPNStateCmdWitness::DeltaMerkleProofArray(delta_merkle_proofs) => {
                delta_merkle_proofs
            },
            _ => panic!("get_delta_merkle_proof_array expects witnesss type to be DeltaMerkleProofArray, but got {:?}",&self),
        }
    }


    pub fn get_read_other_contract_state(self) -> DPNReadOtherUserContractStateLeafMerkleProof<F> {
        match self {
            DPNStateCmdWitness::ReadOtherUserContractState(w) => {
                w
            },
            _ => panic!("get_read_other_contract_state expects witnesss type to be ReadOtherUserContractState, but got {:?}",&self),
        }
    }


    pub fn get_invoke_external_function_deferred(self) -> DPNInvokeDeferredMethodCallWitness<F> {
        match self {
            DPNStateCmdWitness::InvokeExternalContractFunctionDeferred(w) => {
                w
            },
            _ => panic!("get_invoke_external_function_deferred expects witnesss type to be InvokeExternalContractFunctionDeferred, but got {:?}",&self),
        }
    }
    pub fn get_target_array(self) -> Vec<F> {
        match self {
            DPNStateCmdWitness::TargetArray(w) => {
                w
            },
            _ => panic!("get_target_array expects witnesss type to be TargetArray, but got {:?}",&self),
        }
    }
    pub fn get_target_array_2d(self) -> Vec<Vec<F>> {
        match self {
            DPNStateCmdWitness::TargetArray2D(w) => {
                w
            },
            _ => panic!("get_target_array_2d expects witnesss type to be TargetArray2D, but got {:?}",&self),
        }
    }
    pub fn get_clear_entire_tree(self) -> DPNClearEntireTreeWitness<F> {
        match self {
            DPNStateCmdWitness::ClearEntireTree(witness) => {
                witness
            },
            _ => panic!("get_clear_entire_tree expects witness type to be ClearEntireTree, but got {:?}",&self),
        }
    }
}
impl<F: RichField> QEDReadCommandBatchOutput<F> {
    pub fn new() -> Self {
        Self {
            get_user_leaf: Vec::new(),
            get_contract_leaf: Vec::new(),
            get_contract_code: Vec::new(),
            get_checkpoint_leaf: Vec::new(),
            get_l2_block_state: Vec::new(),
            get_merkle_proof: Vec::new(),
            get_hash: Vec::new(),
        }
    }
    pub fn append(&mut self, other: Self) {
        self.get_user_leaf.extend(other.get_user_leaf);
        self.get_contract_leaf.extend(other.get_contract_leaf);
        self.get_contract_code.extend(other.get_contract_code);
        self.get_checkpoint_leaf.extend(other.get_checkpoint_leaf);
        self.get_l2_block_state.extend(other.get_l2_block_state);
        self.get_merkle_proof.extend(other.get_merkle_proof);
        self.get_hash.extend(other.get_hash);
    }
    pub fn concat(&self, other: &Self) -> Self {
        let mut b = self.clone();
        b.append(other.clone());
        b
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait QEDReadCommandProcessorSync<F: RichField> {
    async fn resolve_batch(&self, input: &QEDReadCommandBatchInput) -> anyhow::Result<QEDReadCommandBatchOutput<F>>;
    async fn resolve_get_hash(&self, input: &QSRHashCmd) -> anyhow::Result<QHashOut<F>>;
    async fn resolve_get_merkle_proof(&self, input: &QSRMerkleCmd) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn resolve_get_user_leaf(&self, input: &QSRCmdGetUserLeafData) -> anyhow::Result<QEDUserLeaf<F>>;
    async fn resolve_get_contract_leaf(&self, input: &QSRCmdGetContractLeafData) -> anyhow::Result<QEDContractLeaf<F>>;
    async fn resolve_get_contract_code(&self, input: &QSRCmdGetContractCodeDefinition) -> anyhow::Result<ContractCodeDefinition>;
    async fn resolve_get_checkpoint_leaf(&self, input: &QSRCmdGetCheckpointLeafData) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    async fn resolve_get_l2_block_state(&self, input: &QSRCmdGetL2BlockState) -> anyhow::Result<QEDL2BlockState>;
    async fn resolve_get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait QEDReadCommandProcessorSyncMut<F: RichField> {
    async fn resolve_batch_mut(&mut self, input: &QEDReadCommandBatchInput) -> anyhow::Result<QEDReadCommandBatchOutput<F>>;
    async fn resolve_get_hash_mut(&mut self, input: &QSRHashCmd) -> anyhow::Result<QHashOut<F>>;
    async fn resolve_get_merkle_proof_mut(&mut self, input: &QSRMerkleCmd) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn resolve_get_user_leaf_mut(&mut self, input: &QSRCmdGetUserLeafData) -> anyhow::Result<QEDUserLeaf<F>>;
    async fn resolve_get_contract_leaf_mut(&mut self, input: &QSRCmdGetContractLeafData) -> anyhow::Result<QEDContractLeaf<F>>;
    async fn resolve_get_contract_code_mut(&mut self, input: &QSRCmdGetContractCodeDefinition) -> anyhow::Result<ContractCodeDefinition>;
    async fn resolve_get_checkpoint_leaf_mut(&mut self, input: &QSRCmdGetCheckpointLeafData) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    async fn resolve_get_l2_block_state_mut(&mut self, input: &QSRCmdGetL2BlockState) -> anyhow::Result<QEDL2BlockState>;
    async fn resolve_get_latest_l2_block_state_mut(&mut self) -> anyhow::Result<QEDL2BlockState>;

}
/*
*/
