use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use psy_crypto::signature::zk::data::ZKPublicKeyInfo;
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

use crate::qblock::cmds::register_user::QBCRegisterUser;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct ContractConfig<F: RichField> {
    pub name: String,
    pub path: String,
    pub contract_name: String,
    pub method_names: Vec<String>,
    #[serde(default)]
    pub deployer: QHashOut<F>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct GenesisContractConfig<F: RichField> {
    pub contracts: IndexMap<String, IndexMap<String, GenesisUserContractState<F>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct GenesisUserContractState<F: RichField> {
    pub slots: IndexMap<u64, QHashOut<F>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct GenesisConfig<F: RichField> {
    pub precompiles: Vec<ContractConfig<F>>,
    pub contracts: IndexMap<u64, IndexMap<u64, GenesisUserContractState<F>>>,
    #[serde(default)]
    pub users: Vec<QBCRegisterUser<F>>,
}

impl<F: RichField> GenesisConfig<F> {
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let config: GenesisConfig<F> = serde_json::from_str(json_str)?;
        Ok(config)
    }

    pub fn from_path(config_path: &str) -> anyhow::Result<Option<Self>> {
        match std::fs::read_to_string(config_path) {
            Ok(config_content) => {
        let config_value: serde_json::Value = serde_json::from_str(&config_content)?;
        if let Some(genesis_obj) = config_value.get("genesis") {
            let genesis_config = Self::from_json(&serde_json::to_string(genesis_obj)?)?;
                    Ok(Some(genesis_config))
        } else {
                    Ok(None)
                }
            }
            Err(_e) => Ok(None),
        }
    }

    pub fn get_precompile_paths(&self) -> Vec<String> {
        self.precompiles
            .iter()
            .map(|config| config.path.clone())
            .collect()
    }

    pub fn get_precompile_configs(&self) -> &[ContractConfig<F>] {
        &self.precompiles
    }

    pub fn get_contract_user_state(
        &self,
        contract_id: u64,
        user_id: u64,
    ) -> Option<&GenesisUserContractState<F>> {
        self.contracts
            .get(&contract_id)
            .and_then(|users| users.get(&user_id))
    }

    pub fn get_all_contracts(
        &self,
    ) -> &IndexMap<u64, IndexMap<u64, GenesisUserContractState<F>>> {
        &self.contracts
    }

    pub fn get_genesis_contract_ids(&self) -> Vec<u64> {
        self.contracts.keys().cloned().collect()
    }

    pub fn get_genesis_users(&self) -> &[QBCRegisterUser<F>] {
        &self.users
    }

    pub fn get_contract_deployer(&self, contract_index: usize) -> QHashOut<F> {
        self.precompiles.get(contract_index).unwrap().deployer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};

    #[test]
    fn test_genesis_config_parsing() {
        let json = r#"
        {
            "precompiles": [
                {
                    "name": "test",
                    "path": "qed_precompiles",
                    "contract_name": "ContractRef",
                    "method_names": ["test_method"]
                }
            ],
            "contracts": {
                "1": {
                    "0": {
                        "slots": {
                            "0": "0000000000000000000000000000000000000000000000000000000000000000",
                            "1": "0000000000000000000000000000000000000000000000000000000000000001"
                        }
                    }
                }
            }
        }
        "#;

        let config = GenesisConfig::<GoldilocksField>::from_json(json).unwrap();
        assert_eq!(config.precompiles.len(), 1);
        assert_eq!(config.precompiles[0].path, "qed_precompiles");
        assert_eq!(config.precompiles[0].name, "test");

        let user_state = config.get_contract_user_state(1, 0).unwrap();
        assert_eq!(user_state.slots.len(), 2);
    }
}
