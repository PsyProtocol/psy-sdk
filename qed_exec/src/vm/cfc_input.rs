use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_data::dpn::cfc_context_input::DapenCFCUserTransactionInputContext;
use serde::{Deserialize, Serialize};

use super::exec::QEDCmdWithInputAndWitness;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DapenContractFunctionCircuitInput<F: RichField> {
    pub inputs: Vec<F>,
    pub outputs: Vec<F>,
    pub cmd_witnesses: Vec<QEDCmdWithInputAndWitness<F>>,
    pub tx_input_ctx: DapenCFCUserTransactionInputContext<F>,
}

impl<F: RichField> KVQSerializable for DapenContractFunctionCircuitInput<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
