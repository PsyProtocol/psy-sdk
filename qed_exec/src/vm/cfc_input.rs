use kvq::traits::KVQSerializable;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use qed_core::data::qhashout::QHashOut;
use psy_data::dpn::cfc_context_input::DapenCFCUserTransactionInputContext;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::exec::QEDCmdWithInputAndWitness;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct DapenContractFunctionCircuitInput<F: RichField> {
    pub inputs: Vec<F>,
    pub outputs: Vec<F>,
    pub cmd_witnesses: Vec<QEDCmdWithInputAndWitness<F>>,
    pub session_proof_tree_root: QHashOut<F>,
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
