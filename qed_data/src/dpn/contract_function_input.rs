use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::{HashOut, RichField};
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::QFeltSized};
use qed_crypto::hash::traits::{hasher::FieldHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};

use crate::qdata::contract;


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNContractFunctionInputPreCallInfo<F: RichField> {
    pub user_contract_tree_root: QHashOut<F>,
    pub contract_start_state_root: QHashOut<F>,

    pub contract_id: F,
    pub method_id: F,
    pub balance: F,
    pub event_index: F,
}

impl<F: RichField> KVQSerializable for DPNContractFunctionInputPreCallInfo<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


impl<F: RichField> QFieldHashable<F> for DPNContractFunctionInputPreCallInfo<F> {
    fn qfhash<H: FieldHasher<QHashOut<F>, F>>(&self) -> QHashOut<F> {
        let state_combo = H::hash_many(&[
            self.user_contract_tree_root.0.elements[0],
            self.user_contract_tree_root.0.elements[1],
            self.user_contract_tree_root.0.elements[2],
            self.user_contract_tree_root.0.elements[3],

            self.contract_start_state_root.0.elements[0],
            self.contract_start_state_root.0.elements[1],
            self.contract_start_state_root.0.elements[2],
            self.contract_start_state_root.0.elements[3],
        ]);

        let final_hash =  H::hash_many(&[
            state_combo.0.elements[0],
            state_combo.0.elements[1],
            state_combo.0.elements[2],
            state_combo.0.elements[3],

            self.contract_id,
            self.method_id,
            self.balance,
            self.event_index,
        ]);

        final_hash
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNContractFunctionInput<F: RichField> {
    pre_call_info: DPNContractFunctionInputPreCallInfo<F>,
    end_contract_state_root: QHashOut<F>,
    
    inputs: Vec<F>,
    outputs: Vec<F>,
}
