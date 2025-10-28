use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::{
    core::{DeltaMerkleProofCore, MerkleProofCore},
    treeprover::{AggStateTrackableInput, AggStateTransition},
};
use serde::{Deserialize, Serialize};

use crate::qdata::{
    checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf},
    contract::QEDContractLeaf,
    user::QEDUserLeaf,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDUserRegistrationCircuitInput<F: RichField> {
    pub user_tree_delta_merkle_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub user_leaf: QEDUserLeaf<F>,
    pub allowed_circuit_hashes_root: QHashOut<F>,
}
impl<F: RichField> AggStateTrackableInput<F> for QEDUserRegistrationCircuitInput<F> {
    fn get_state_transition(&self) -> AggStateTransition<F> {
        AggStateTransition {
            state_transition_start: self.user_tree_delta_merkle_proof.old_root,
            state_transition_end: self.user_tree_delta_merkle_proof.new_root,
        }
    }
}

impl<F: RichField> KVQSerializable for QEDUserRegistrationCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDDeployContractCircuitInput<F: RichField> {
    pub allowed_circuit_hashes_root: QHashOut<F>,
    pub contract_tree_delta_merkle_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub contract_leaf: QEDContractLeaf<F>,
}
impl<F: RichField> AggStateTrackableInput<F> for QEDDeployContractCircuitInput<F> {
    fn get_state_transition(&self) -> AggStateTransition<F> {
        AggStateTransition {
            state_transition_start: self.contract_tree_delta_merkle_proof.old_root,
            state_transition_end: self.contract_tree_delta_merkle_proof.new_root,
        }
    }
}

impl<F: RichField> KVQSerializable for QEDDeployContractCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDCheckpointStateTransitionCircuitInput<F: RichField> {
    pub old_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub old_checkpoint_leaf: QEDCheckpointLeaf<F>,

    pub new_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub new_checkpoint_leaf: QEDCheckpointLeaf<F>,

    pub boundry_user_registration_merkle_proof: MerkleProofCore<QHashOut<F>>,
    pub boundry_user_update_merkle_proof: MerkleProofCore<QHashOut<F>>,

    pub checkpoint_delta_merkle_proof: DeltaMerkleProofCore<QHashOut<F>>,
}
impl<F: RichField> AggStateTrackableInput<F> for QEDCheckpointStateTransitionCircuitInput<F> {
    fn get_state_transition(&self) -> AggStateTransition<F> {
        AggStateTransition {
            state_transition_start: self.checkpoint_delta_merkle_proof.old_root,
            state_transition_end: self.checkpoint_delta_merkle_proof.new_root,
        }
    }
}

impl<F: RichField> KVQSerializable for QEDCheckpointStateTransitionCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDInternalBlockCircuitInputs<F: RichField> {
    pub register_users: Vec<QEDUserRegistrationCircuitInput<F>>,
    pub deploy_contracts: Vec<QEDDeployContractCircuitInput<F>>,
    pub checkpoint_state_transition: QEDCheckpointStateTransitionCircuitInput<F>,
}
