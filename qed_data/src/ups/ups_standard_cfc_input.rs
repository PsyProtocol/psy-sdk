//UPSCFCStandardStateDelta<F>



use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{common::witnesses::qrecursion::header::AttestTreeAwareProofInTreeInput, hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, traits::{hasher::FieldQHasher, qhashable::QFieldHashable}}};
use serde::{Deserialize, Serialize};

use crate::{dpn::cfc_context_input::DapenCFCUserTransactionInputContext, qdata::{checkpoint::QEDCheckpointLeafCompactWithStateRoots, contract_inclusion::QEDContractFunctionInclusionProof, user::QEDUserLeaf}};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct UPSCFCStandardStateDeltaInput<F: RichField> {
    pub cfc_transaction_input_context: DapenCFCUserTransactionInputContext<F>,
    pub user_contract_tree_update_proof: DeltaMerkleProofCore<QHashOut<F>>,
    pub deferred_tx_debt_pivot_proof: MerkleProofCore<QHashOut<F>>,
}

impl<F: RichField> KVQSerializable for UPSCFCStandardStateDeltaInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
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
