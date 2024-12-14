use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::{QFeltSized, ToQFelts}};
use qed_crypto::hash::traits::{hasher::FieldHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractLeaf<F: RichField> {
    pub deployer: QHashOut<F>,
    pub function_tree_root: QHashOut<F>,
    pub state_tree_height: F,
}

impl<F: RichField> KVQSerializable for QEDContractLeaf<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> QFeltSized for QEDContractLeaf<F> {
    fn q_felt_size() -> usize {
        9
    }
}
impl<F: RichField> ToQFelts<F> for QEDContractLeaf<F> {
    fn to_qfelts(&self) -> Vec<F> {
        vec![
            self.deployer.0.elements[0],
            self.deployer.0.elements[1],
            self.deployer.0.elements[2],
            self.deployer.0.elements[3],
            self.function_tree_root.0.elements[0],
            self.function_tree_root.0.elements[1],
            self.function_tree_root.0.elements[2],
            self.function_tree_root.0.elements[3],
            self.state_tree_height,
        ]
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 9 {
            panic!("Invalid number of elements for QEDContractLeaf");
        }
        let deployer = QHashOut::from_qfelts(&felts[0..4]);
        let function_tree_root = QHashOut::from_qfelts(&felts[4..8]);
        let state_tree_height = felts[8];
        QEDContractLeaf {
            deployer,
            function_tree_root,
            state_tree_height,
        }
    }
}


impl<F: RichField> QFieldHashable<F> for QEDContractLeaf<F> {
    fn qfhash<H: FieldHasher<QHashOut<F>, F>>(&self) -> QHashOut<F> {
        H::hash_many(&[
            self.deployer.0.elements[0],
            self.deployer.0.elements[1],
            self.deployer.0.elements[2],
            self.deployer.0.elements[3],
            self.function_tree_root.0.elements[0],
            self.function_tree_root.0.elements[1],
            self.function_tree_root.0.elements[2],
            self.function_tree_root.0.elements[3],
            self.state_tree_height,
        ])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContractFunctionCodeDefinition {
    pub call_signature: [u8; 20],
    pub vm_type: u32,
    pub code: Vec<u8>,
}

impl KVQSerializable for ContractFunctionCodeDefinition {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContractCodeDefinition {
    pub state_tree_height: u16,
    pub functions: Vec<ContractFunctionCodeDefinition>,
}

impl KVQSerializable for ContractCodeDefinition {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
