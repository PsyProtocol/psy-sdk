use anyhow::ensure;
use kvq::traits::KVQSerializable;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use psy_common::data::qhashout::QHashOut;
use psy_config::network_constants::CONTRACT_FUNCTION_TREE_HEIGHT;
use psy_crypto::hash::{merkle::utils::simple_merkle_tree::SimpleMerkleTree, traits::hasher::{MerkleZeroHasher, MerkleZeroHasherWithMarkedLeaf}};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::qdata::contract::ContractCodeDefinition;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
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
    pub fn into_with_whitelist_root<H: MerkleZeroHasherWithMarkedLeaf<QHashOut<F>>>(self) -> anyhow::Result<QBCDeployContractWithRoot<F>> {
        QBCDeployContractWithRoot::<F>::new::<H>(self.deployer, self.code_definition, self.function_whitelist)
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QBCDeployContractWithRoot<F: RichField> {
    pub deployer: QHashOut<F>,
    pub code_definition: ContractCodeDefinition,
    pub function_whitelist: Vec<QHashOut<F>>,
    pub function_whitelist_root: QHashOut<F>,
}

impl<F: RichField> QBCDeployContractWithRoot<F> {
    pub fn new<H: MerkleZeroHasherWithMarkedLeaf<QHashOut<F>>>(
        deployer: QHashOut<F>,
        code_definition: ContractCodeDefinition,
        function_whitelist: Vec<QHashOut<F>>,
    ) -> anyhow::Result<Self> {
        ensure!(
            function_whitelist.len() == code_definition.functions.len() * 4,
            "function_whitelist must contain four entries per function"
        );
        let zero = QHashOut::from_values(0, 0, 0, 0);
        for i in 0..code_definition.functions.len() {
            let base = i * 4;
            ensure!(function_whitelist[base + 3] == zero, "function whitelist placeholder must be zero");
        }
        let mut whitelist_tree = SimpleMerkleTree::<H, QHashOut<F>>::new(CONTRACT_FUNCTION_TREE_HEIGHT);
        for (i, leaf) in function_whitelist.iter().enumerate() {
            whitelist_tree.set_leaf(i as u64, *leaf);
        }
        let function_whitelist_root = whitelist_tree.get_root();

        Ok(Self {
            deployer,
            code_definition,
            function_whitelist,
            function_whitelist_root,
        })
    }
}

impl<F: RichField> KVQSerializable for QBCDeployContractWithRoot<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QContractMetadata {
    pub contract_id: Option<u64>,
    pub deployer: String,
    pub state_tree_height: u16,
    pub function_count: usize,
    pub function_whitelist_root: String,
    pub functions: Vec<QFunctionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QFunctionMetadata {
    pub method_id: u32,
    pub name: String,
    pub num_inputs: u32,
    pub num_outputs: u32,
}
