use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use plonky2::hash::hash_types::RichField;
use crate::data::qhashout::QHashOut;

/// Genesis configuration for precompiled contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisPrecompileConfig {
    /// Path to the precompiles crate
    pub precompiles: Vec<String>,
}

/// Genesis configuration for contract initial state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContractConfig {
    /// Contract ID -> User ID -> Slot configuration
    pub contracts: HashMap<String, HashMap<String, GenesisContractUserState>>,
}

/// Initial state for a user's contract storage slots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisContractUserState {
    /// Slot ID -> hex value
    pub slots: HashMap<String, String>,
}

/// Complete genesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Precompiled contracts configuration
    pub precompiles: Vec<String>,
    /// Initial contract states
    pub contracts: HashMap<String, HashMap<String, GenesisContractUserState>>,
}

impl GenesisConfig {
    /// Parse genesis configuration from JSON
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let config: GenesisConfig = serde_json::from_str(json_str)?;
        Ok(config)
    }

    /// Get all precompiled contract paths
    pub fn get_precompile_paths(&self) -> &[String] {
        &self.precompiles
    }

    /// Get contract state for a specific contract ID and user ID
    pub fn get_contract_user_state(&self, contract_id: u64, user_id: u64) -> Option<&GenesisContractUserState> {
        self.contracts
            .get(&contract_id.to_string())
            .and_then(|users| users.get(&user_id.to_string()))
    }

    /// Get all contracts with their initial states
    pub fn get_all_contracts(&self) -> &HashMap<String, HashMap<String, GenesisContractUserState>> {
        &self.contracts
    }

    /// Convert hex string to field element
    pub fn hex_to_field<F: RichField>(hex_str: &str) -> anyhow::Result<F> {
        // Remove 0x prefix if present
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        
        // Parse as u64 (assuming field elements fit in u64)
        let value = u64::from_str_radix(hex_str, 16)?;
        Ok(F::from_canonical_u64(value))
    }

    /// Convert hex string to QHashOut
    pub fn hex_to_qhashout<F: RichField>(hex_str: &str) -> anyhow::Result<QHashOut<F>> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        
        // Ensure the hex string is exactly 64 characters (256 bits)
        if hex_str.len() != 64 {
            anyhow::bail!("Hex string must be exactly 64 characters (256 bits)");
        }
        
        // Split into 4 parts of 16 characters each (64 bits each)
        let mut elements = [F::ZERO; 4];
        for i in 0..4 {
            let start = i * 16;
            let end = start + 16;
            let part = &hex_str[start..end];
            let value = u64::from_str_radix(part, 16)?;
            elements[i] = F::from_canonical_u64(value);
        }
        
        Ok(QHashOut::from_felt_slice(&elements))
    }

    /// Get the contract IDs that should be deployed in genesis
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

    #[test]
    fn test_hex_to_field() {
        type F = GoldilocksField;
        
        let result = GenesisConfig::hex_to_field::<F>("0x1234567890abcdef").unwrap();
        assert_eq!(result, F::from_canonical_u64(0x1234567890abcdef));
        
        let result = GenesisConfig::hex_to_field::<F>("1234567890abcdef").unwrap();
        assert_eq!(result, F::from_canonical_u64(0x1234567890abcdef));
    }

    #[test]
    fn test_hex_to_qhashout() {
        type F = GoldilocksField;
        
        let hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let result = GenesisConfig::hex_to_qhashout::<F>(hex).unwrap();
        
        // Should have the last element as 1
        assert_eq!(result.0.elements[3], F::ONE);
        assert_eq!(result.0.elements[0], F::ZERO);
        assert_eq!(result.0.elements[1], F::ZERO);
        assert_eq!(result.0.elements[2], F::ZERO);
    }
}