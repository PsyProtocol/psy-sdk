use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisPrecompileConfig {
    pub precompiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContractConfig {
    pub contracts: HashMap<String, HashMap<String, GenesisContractUserState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContractUserState {
    pub slots: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub precompiles: Vec<String>,
    pub contracts: HashMap<String, HashMap<String, GenesisContractUserState>>,
}

impl GenesisConfig {
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let config: GenesisConfig = serde_json::from_str(json_str)?;
        Ok(config)
    }

    pub fn get_precompile_paths(&self) -> &[String] {
        &self.precompiles
    }

    pub fn get_contract_user_state(&self, contract_id: u64, user_id: u64) -> Option<&GenesisContractUserState> {
        self.contracts
            .get(&contract_id.to_string())
            .and_then(|users| users.get(&user_id.to_string()))
    }

    pub fn get_all_contracts(&self) -> &HashMap<String, HashMap<String, GenesisContractUserState>> {
        &self.contracts
    }

    pub fn get_genesis_contract_ids(&self) -> Vec<u64> {
        self.contracts
            .keys()
            .filter_map(|k| k.parse::<u64>().ok())
            .collect()
    }
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            precompiles: vec!["qed_precompiles".to_string()],
            contracts: HashMap::new(),
        }
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
            "precompiles": ["qed_precompiles"],
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

        let config = GenesisConfig::from_json(json).unwrap();
        assert_eq!(config.precompiles.len(), 1);
        assert_eq!(config.precompiles[0], "qed_precompiles");

        let user_state = config.get_contract_user_state(1, 0).unwrap();
        assert_eq!(user_state.slots.len(), 2);
    }

}
