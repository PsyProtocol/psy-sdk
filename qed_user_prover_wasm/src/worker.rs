//! WASM-compatible prover worker store implementation

use crate::error::{WasmError, WasmResult};
use crate::types::*;
use crate::types::Hash256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// WASM-compatible prover worker store
#[derive(Debug, Default)]
pub struct ProverWorkerStore {
    /// Storage for async operation results
    results: HashMap<Hash256, Vec<u8>>,
    /// Storage for session data
    sessions: HashMap<String, SessionData>,
    /// Storage for user data
    users: HashMap<String, UserData>,
    /// Storage for contract data
    contracts: HashMap<String, ContractData>,
    /// Counter for generating unique IDs
    id_counter: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    session_id: String,
    user_id: Option<String>,
    created_at: u64,
    last_activity: u64,
    state: SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SessionState {
    Active,
    Proving,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserData {
    user_id: String,
    public_key: String,
    private_key_hash: String,
    created_at: u64,
    last_login: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContractData {
    contract_address: String,
    contract_code: String,
    deployed_at: u64,
    functions: Vec<String>,
}

impl ProverWorkerStore {
    /// Create a new prover worker store
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            sessions: HashMap::new(),
            users: HashMap::new(),
            contracts: HashMap::new(),
            id_counter: 0,
        }
    }

    /// Generate a unique ID
    fn generate_id(&mut self) -> Hash256 {
        self.id_counter += 1;
        let id_bytes = self.id_counter.to_le_bytes();
        let mut hash_bytes = [0u8; 32];
        hash_bytes[..8].copy_from_slice(&id_bytes);
        Hash256::from_bytes(hash_bytes)
    }

    /// Store a result and return its ID
    pub fn store_result(&mut self, data: Vec<u8>) -> Hash256 {
        let id = self.generate_id();
        self.results.insert(id.clone(), data);
        id
    }

    /// Get and clear a result by ID
    pub fn get_result_and_clear(&mut self, id: &Hash256) -> Option<Vec<u8>> {
        self.results.remove(id)
    }

    /// Get a result by ID without clearing
    pub fn get_result(&self, id: &Hash256) -> Option<&Vec<u8>> {
        self.results.get(id)
    }

    /// Clear a result by ID
    pub fn clear_result(&mut self, id: &Hash256) -> bool {
        self.results.remove(id).is_some()
    }

    /// Store session data
    pub fn store_session(&mut self, session_info: SessionInfo) -> WasmResult<()> {
        let session_data = SessionData {
            session_id: session_info.session_id.clone(),
            user_id: session_info.user_id,
            created_at: session_info.created_at,
            last_activity: session_info.last_activity,
            state: SessionState::Active,
        };
        
        self.sessions.insert(session_info.session_id, session_data);
        Ok(())
    }

    /// Get session data
    pub fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.get(session_id).map(|data| SessionInfo {
            session_id: data.session_id.clone(),
            user_id: data.user_id.clone(),
            created_at: data.created_at,
            last_activity: data.last_activity,
        })
    }

    /// Update session activity
    pub fn update_session_activity(&mut self, session_id: &str) -> WasmResult<()> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.last_activity = crate::utils::current_timestamp();
            Ok(())
        } else {
            Err(WasmError::Session(format!("Session not found: {}", session_id)))
        }
    }

    /// Set session state
    pub fn set_session_state(&mut self, session_id: &str, state: &str) -> WasmResult<()> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.state = match state {
                "active" => SessionState::Active,
                "proving" => SessionState::Proving,
                "completed" => SessionState::Completed,
                _ => SessionState::Failed(state.to_string()),
            };
            Ok(())
        } else {
            Err(WasmError::Session(format!("Session not found: {}", session_id)))
        }
    }

    /// Remove session
    pub fn remove_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    /// Store user data
    pub fn store_user(&mut self, user_info: UserInfo) -> WasmResult<()> {
        let user_data = UserData {
            user_id: user_info.user_id.clone(),
            public_key: user_info.public_key,
            private_key_hash: String::new(), // Don't store private keys
            created_at: user_info.created_at,
            last_login: crate::utils::current_timestamp(),
        };
        
        self.users.insert(user_info.user_id, user_data);
        Ok(())
    }

    /// Get user data
    pub fn get_user(&self, user_id: &str) -> Option<UserInfo> {
        self.users.get(user_id).map(|data| UserInfo {
            user_id: data.user_id.clone(),
            public_key: data.public_key.clone(),
            created_at: data.created_at,
        })
    }

    /// Update user login time
    pub fn update_user_login(&mut self, user_id: &str) -> WasmResult<()> {
        if let Some(user) = self.users.get_mut(user_id) {
            user.last_login = crate::utils::current_timestamp();
            Ok(())
        } else {
            Err(WasmError::Session(format!("User not found: {}", user_id)))
        }
    }

    /// Remove user
    pub fn remove_user(&mut self, user_id: &str) -> bool {
        self.users.remove(user_id).is_some()
    }

    /// List all users
    pub fn list_users(&self) -> Vec<UserInfo> {
        self.users.values().map(|data| UserInfo {
            user_id: data.user_id.clone(),
            public_key: data.public_key.clone(),
            created_at: data.created_at,
        }).collect()
    }

    /// Store contract data
    pub fn store_contract(&mut self, contract_address: &str, contract_code: &str, functions: Vec<String>) -> WasmResult<()> {
        let contract_data = ContractData {
            contract_address: contract_address.to_string(),
            contract_code: contract_code.to_string(),
            deployed_at: crate::utils::current_timestamp(),
            functions,
        };
        
        self.contracts.insert(contract_address.to_string(), contract_data);
        Ok(())
    }

    /// Get contract data
    pub fn get_contract(&self, contract_address: &str) -> Option<&ContractData> {
        self.contracts.get(contract_address)
    }

    /// Remove contract
    pub fn remove_contract(&mut self, contract_address: &str) -> bool {
        self.contracts.remove(contract_address).is_some()
    }

    /// List all contracts
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            results_count: self.results.len(),
            sessions_count: self.sessions.len(),
            users_count: self.users.len(),
            contracts_count: self.contracts.len(),
            total_memory_usage: self.estimate_memory_usage(),
        }
    }

    /// Estimate memory usage (rough calculation)
    fn estimate_memory_usage(&self) -> usize {
        let results_size: usize = self.results.values().map(|v| v.len()).sum();
        let sessions_size = self.sessions.len() * std::mem::size_of::<SessionData>();
        let users_size = self.users.len() * std::mem::size_of::<UserData>();
        let contracts_size = self.contracts.len() * std::mem::size_of::<ContractData>();
        
        results_size + sessions_size + users_size + contracts_size
    }

    /// Clear all data
    pub fn clear_all(&mut self) {
        self.results.clear();
        self.sessions.clear();
        self.users.clear();
        self.contracts.clear();
        self.id_counter = 0;
    }

    /// Clear expired sessions (older than 24 hours)
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let current_time = crate::utils::current_timestamp();
        let expiry_time = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
        
        let expired_sessions: Vec<String> = self.sessions
            .iter()
            .filter(|(_, session)| current_time - session.last_activity > expiry_time)
            .map(|(id, _)| id.clone())
            .collect();
        
        let count = expired_sessions.len();
        for session_id in expired_sessions {
            self.sessions.remove(&session_id);
        }
        
        count
    }

    /// Clear old results (older than 1 hour)
    pub fn cleanup_old_results(&mut self) -> usize {
        // For simplicity, we'll clear results based on ID counter
        // In a real implementation, you'd track timestamps for results
        let old_threshold = if self.id_counter > 1000 { self.id_counter - 1000 } else { 0 };
        
        let old_results: Vec<Hash256> = self.results
            .keys()
            .filter(|id| {
                // Simple heuristic: assume lower ID counter means older result
                let id_bytes = id.as_bytes();
                let id_counter = u64::from_le_bytes([
                    id_bytes[0], id_bytes[1], id_bytes[2], id_bytes[3],
                    id_bytes[4], id_bytes[5], id_bytes[6], id_bytes[7],
                ]);
                id_counter < old_threshold
            })
            .cloned()
            .collect();
        
        let count = old_results.len();
        for result_id in old_results {
            self.results.remove(&result_id);
        }
        
        count
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct StorageStats {
    results_count: usize,
    sessions_count: usize,
    users_count: usize,
    contracts_count: usize,
    total_memory_usage: usize,
}

// WASM bindings for storage stats
#[wasm_bindgen]
impl StorageStats {
    #[wasm_bindgen(getter)]
    pub fn results_count(&self) -> usize {
        self.results_count
    }

    #[wasm_bindgen(getter)]
    pub fn sessions_count(&self) -> usize {
        self.sessions_count
    }

    #[wasm_bindgen(getter)]
    pub fn users_count(&self) -> usize {
        self.users_count
    }

    #[wasm_bindgen(getter)]
    pub fn contracts_count(&self) -> usize {
        self.contracts_count
    }

    #[wasm_bindgen(getter)]
    pub fn total_memory_usage(&self) -> usize {
        self.total_memory_usage
    }
}