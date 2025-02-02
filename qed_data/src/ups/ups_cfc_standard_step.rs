

use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use serde::{Deserialize, Serialize};



use super::{ups_standard_cfc_input::UPSVerifyCFCStandardStepInput, verify_previous_ups_step::VerifyPreviousUPSStepProofInProofTreeInput};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct UPSCFCStandardTransactionCircuitInput<F: RichField> {
    pub verify_previous_ups_step: VerifyPreviousUPSStepProofInProofTreeInput<F>,
    pub standard_cfc_step: UPSVerifyCFCStandardStepInput<F>,
}



impl<F: RichField> KVQSerializable for UPSCFCStandardTransactionCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

