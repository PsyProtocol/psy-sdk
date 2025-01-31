

use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::{common::witnesses::qrecursion::header::AttestTreeAwareProofInTreeInput, hash::{merkle::core::MerkleProofCore, traits::{hasher::{FieldHasher, FieldQHasher}, qhashable::QFieldHashable}}};
use serde::{Deserialize, Serialize};

use crate::qdata::user::QEDUserLeaf;

use super::ups_context_input::UserProvingSessionCurrentState;



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct VerifyPreviousUPSStepProofInProofTreeInput<F: RichField> {
    pub proof_attestation_witness: AttestTreeAwareProofInTreeInput<F>,
    pub previous_step_state: UserProvingSessionCurrentState<F>,
    pub ups_circuit_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
}


impl<F: RichField> KVQSerializable for VerifyPreviousUPSStepProofInProofTreeInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}