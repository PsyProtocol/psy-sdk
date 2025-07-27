

use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::common::witnesses::qrecursion::header::AttestProofInTreeInput;
use serde::{Deserialize, Serialize};


use crate::qdata::user_contract_state::UserContractState;

use super::verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeInput;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct UPSEndCapFromProofTreeGadgetInput<F: RichField> {
    pub user_contract_state: UserContractState<F>,
    pub verify_previous_ups_step_input: VerifyPreviousUPSStepProofInProofTreeInput<F>,
    pub verify_zk_signature_proof_input: AttestProofInTreeInput<F>,
    pub user_public_key_param: QHashOut<F>,
    pub nonce: F,
    pub slots_modified: F,
}



impl<F: RichField> KVQSerializable for UPSEndCapFromProofTreeGadgetInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

