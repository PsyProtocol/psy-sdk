//UPSCFCStandardStateDelta<F>

use kvq::traits::KVQSerializable;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use psy_core::data::qhashout::QHashOut;
use psy_crypto::{
    common::witnesses::qrecursion::header::AttestTreeAwareProofInTreeInput,
    hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    dpn::cfc_context_input::DapenCFCUserTransactionInputContext,
    qdata::{checkpoint::QEDCheckpointLeafCompactWithStateRoots, contract_inclusion::QEDContractFunctionInclusionProof},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct UPSCFCStandardStateDeltaInput<F: RichField> {
    pub cfc_transaction_input_context: DapenCFCUserTransactionInputContext<F>,
    pub user_contract_tree_update_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub deferred_tx_debt_pivot_proof: MerkleProofCore<QHashOut<F>>,
    pub inline_tx_debt_pivot_proof: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for UPSCFCStandardStateDeltaInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct UPSVerifyCFCStandardStepInput<F: RichField> {
    pub checkpoint_state: QEDCheckpointLeafCompactWithStateRoots<F>,
    pub verify_cfc_proof_input: AttestTreeAwareProofInTreeInput<F>,
    pub cfc_inclusion_proof: QEDContractFunctionInclusionProof<F>,

    pub process_cfc_state_delta_input: UPSCFCStandardStateDeltaInput<F>,
}

impl<F: RichField> KVQSerializable for UPSVerifyCFCStandardStepInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct UPSVerifyPopDeferredTxStepInput<F: RichField> {
    pub standard_cfc_verify_input: UPSVerifyCFCStandardStepInput<F>,
    pub ups_pop_deferred_tx_proof: DeltaMerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for UPSVerifyPopDeferredTxStepInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
