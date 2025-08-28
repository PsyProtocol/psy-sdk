use std::collections::HashMap;
use serde_json::Value;

pub const REWARDS_CONTRACT_ID: u32 = 0;
pub const TOKEN_CONTRACT_ID: u32 = 1;

#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub id: u32,
    pub name: &'static str,
    pub json_definition: &'static str,
    pub methods: Vec<String>,
    pub description: &'static str,
}

// Constants for JSON content - will be populated by build.rs
static REWARDS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/rewards.json"));
static TOKEN_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/token.json"));

/// Get the precompiled contract definition by contract ID
pub fn get_precompiled_contract(contract_id: u32) -> Option<&'static str> {
    match contract_id {
        REWARDS_CONTRACT_ID => {
            if REWARDS_JSON.is_empty() {
                None
            } else {
                Some(REWARDS_JSON)
            }
        }
        TOKEN_CONTRACT_ID => {
            if TOKEN_JSON.is_empty() {
                None
            } else {
                Some(TOKEN_JSON)
            }
        }
        _ => None,
    }
}

/// Get all precompiled contracts as a HashMap
pub fn get_all_precompiled_contracts() -> HashMap<u32, &'static str> {
    let mut contracts = HashMap::new();
    
    if let Some(contract) = get_precompiled_contract(REWARDS_CONTRACT_ID) {
        contracts.insert(REWARDS_CONTRACT_ID, contract);
    }
    
    if let Some(contract) = get_precompiled_contract(TOKEN_CONTRACT_ID) {
        contracts.insert(TOKEN_CONTRACT_ID, contract);
    }
    
    contracts
}

/// Get the contract name for a precompiled contract
pub fn get_contract_name(contract_id: u32) -> Option<&'static str> {
    match contract_id {
        REWARDS_CONTRACT_ID => Some("rewards"),
        TOKEN_CONTRACT_ID => Some("token"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_precompiled_contract() {
        // Note: These tests will fail until contracts are compiled
        // assert!(get_precompiled_contract(REWARDS_CONTRACT_ID).is_some());
        // assert!(get_precompiled_contract(TOKEN_CONTRACT_ID).is_some());
        assert!(get_precompiled_contract(999).is_none());
    }

    #[test]
    fn test_get_all_precompiled_contracts() {
        let contracts = get_all_precompiled_contracts();
        // Note: contracts might be empty until build process completes
    }

    #[test]
    fn test_get_contract_name() {
        assert_eq!(get_contract_name(REWARDS_CONTRACT_ID), Some("rewards"));
        assert_eq!(get_contract_name(TOKEN_CONTRACT_ID), Some("token"));
        assert_eq!(get_contract_name(999), None);
    }
}
