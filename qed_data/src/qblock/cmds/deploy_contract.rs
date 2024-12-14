use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

use crate::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QBCDeployContract<F: RichField> {
    pub deployer: QHashOut<F>,
    pub code_definition: ContractCodeDefinition,
    pub function_whitelist: Vec<QHashOut<F>>,

}

impl<F: RichField> QBCDeployContract<F> {
    pub fn new(deployer: QHashOut<F>, code_definition: ContractCodeDefinition, function_whitelist: Vec<QHashOut<F>>) -> Self {
        Self {
            deployer,
            code_definition,
            function_whitelist,
        }
    }
}


impl<F: RichField> KVQSerializable for QBCDeployContract<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
