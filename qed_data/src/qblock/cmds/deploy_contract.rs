use anyhow::ensure;
use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::{config::network_constants::CONTRACT_FUNCTION_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::hash::{merkle::utils::simple_merkle_tree::SimpleMerkleTree, traits::hasher::MerkleZeroHasher};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::qdata::contract::ContractCodeDefinition;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash,TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct QBCDeployContract<F: RichField> {
    pub deployer: QHashOut<F>,
    pub code_definition: ContractCodeDefinition,
    pub function_whitelist: Vec<QHashOut<F>>,
    #[serde(default)]
    pub function_code_hashes: Vec<QHashOut<F>>,

}

impl<F: RichField> QBCDeployContract<F> {
    pub fn new(
        deployer: QHashOut<F>,
        code_definition: ContractCodeDefinition,
        function_whitelist: Vec<QHashOut<F>>,
        function_code_hashes: Vec<QHashOut<F>>,
    ) -> Self {
        Self {
            deployer,
            code_definition,
            function_whitelist,
            function_code_hashes,
        }
    }
    pub fn into_with_whitelist_root<H: MerkleZeroHasher<QHashOut<F>>>(self) -> anyhow::Result<QBCDeployContractWithRoot<F>>{
        QBCDeployContractWithRoot::<F>::new::<H>(
            self.deployer,
            self.code_definition,
            self.function_whitelist,
            self.function_code_hashes,
        )

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
    #[serde(default)]
    pub function_code_hashes: Vec<QHashOut<F>>,
    pub function_code_hash_root: QHashOut<F>,

}

impl<F: RichField> QBCDeployContractWithRoot<F> {
    pub fn new<H: MerkleZeroHasher<QHashOut<F>>>(
        deployer: QHashOut<F>,
        code_definition: ContractCodeDefinition,
        function_whitelist: Vec<QHashOut<F>>,
        function_code_hashes: Vec<QHashOut<F>>,
    ) -> anyhow::Result<Self> {
        ensure!(
            function_code_hashes.len() == code_definition.functions.len(),
            "function_code_hashes length must equal number of functions"
        );
        ensure!(
            function_whitelist.len() == code_definition.functions.len() * 2,
            "function_whitelist must contain two entries per function"
        );
        let mut whitelist_tree = SimpleMerkleTree::<H, QHashOut<F>>::new(CONTRACT_FUNCTION_TREE_HEIGHT);
        for (i, leaf) in function_whitelist.iter().enumerate() {
            whitelist_tree.set_leaf(i as u64, *leaf);
        }
        let function_whitelist_root = whitelist_tree.get_root();

        let mut code_tree = SimpleMerkleTree::<H, QHashOut<F>>::new(CONTRACT_FUNCTION_TREE_HEIGHT);
        for (i, leaf) in function_code_hashes.iter().enumerate() {
            code_tree.set_leaf(i as u64, *leaf);
        }
        let function_code_hash_root = code_tree.get_root();

        Ok(Self {
            deployer,
            code_definition,
            function_whitelist,
            function_whitelist_root,
            function_code_hashes,
            function_code_hash_root,
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
