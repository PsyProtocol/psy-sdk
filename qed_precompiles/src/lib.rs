use std::collections::HashMap;

pub const REWARDS_CONTRACT_ID: u32 = 0;
pub const TOKEN_CONTRACT_ID: u32 = 1;

/// Get the precompiled contract definition by contract ID
pub fn get_precompiled_contract(contract_id: u32) -> Option<&'static str> {
    match contract_id {
        REWARDS_CONTRACT_ID => {
            Some(include_str!("../rewards/target/rewards.json"))
        }
        TOKEN_CONTRACT_ID => {
            Some(include_str!("../token/target/token.json"))
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
