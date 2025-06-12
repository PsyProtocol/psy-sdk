//! QED User Prover WebAssembly Module
//!
//! This module provides a WebAssembly interface for the QED user-side zero-knowledge proof prover.
//! It implements all RPC methods from the Rust local prover for browser environments.

// #[cfg(not(target_arch = "wasm32"))]
// compile_error!("This crate is designed to work only with WebAssembly targets. Please use --target wasm32-unknown-unknown");

mod error;
mod session;
mod types;
mod utils;
mod worker;
mod provider;

mod request;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::Promise;
use web_sys::console;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;

// QED Core imports
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::data::qhashout::QHashOut;

// WASM-specific imports
use console_error_panic_hook;
use wasm_logger;
use getrandom;

// Re-export types and utilities
pub use error::{WasmError, WasmResult};
pub use types::*;
pub use utils::*;

// Removed duplicate import - already imported above
use session::WasmWalletSession;
use worker::ProverWorkerStore;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

// Global state for the WASM module
static mut PROVER_INSTANCE: Option<QEDUserProverWasm> = None;
static INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Main QED User Prover WASM instance
#[wasm_bindgen]
pub struct QEDUserProverWasm {
    store: Arc<Mutex<ProverWorkerStore>>,
    wallet_session: Arc<RwLock<WasmWalletSession>>,
    is_initialized: bool,
}

#[wasm_bindgen]
impl QEDUserProverWasm {
    /// Create a new QED User Prover WASM instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();
        utils::init_logger();
        
        Self {
            store: Arc::new(Mutex::new(ProverWorkerStore::new())),
            wallet_session: Arc::new(RwLock::new(WasmWalletSession::new())),
            is_initialized: false,
        }
    }

    /// Initialize the prover with RPC configuration
    #[wasm_bindgen]
    pub async fn initialize(&mut self, rpc_config_json: &str) -> Result<(), JsValue> {
        let rpc_config: RpcConfig = serde_json::from_str(rpc_config_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse RPC config: {}", e)))?;

        let wallet_session = WasmWalletSession::from_config(&rpc_config)
            .map_err(|e| JsValue::from_str(&format!("Failed to create wallet session: {}", e)))?;

        *self.wallet_session.write().unwrap() = wallet_session;
        self.is_initialized = true;
        
        console::log_1(&"QED User Prover WASM initialized successfully".into());
        Ok(())
    }

    /// Check if the prover is initialized
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    // ========== Local Proving Operations ==========

    /// Start a new proving session
    #[wasm_bindgen]
    pub async fn start_session(&self) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.start_session()
            .map_err(|e| JsValue::from_str(&format!("Failed to start session: {}", e)))?;
        
        Ok("Session started successfully".to_string())
    }

    /// Prove a single contract call
    #[wasm_bindgen]
    pub async fn prove_contract_call(&self, contract_call_args_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let contract_call_args: ContractCallArgs = serde_json::from_str(contract_call_args_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse contract call args: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.prove_contract_call(contract_call_args)
            .map_err(|e| JsValue::from_str(&format!("Failed to prove contract call: {}", e)))?;
        
        Ok("Contract call proved successfully".to_string())
    }

    /// Prove multiple contract calls
    #[wasm_bindgen]
    pub async fn prove_contract_calls(&self, contract_call_args_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_call_args_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse contract call args: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.prove_contract_calls(contract_call_args)
            .map_err(|e| JsValue::from_str(&format!("Failed to prove contract calls: {}", e)))?;
        
        Ok("Contract calls proved successfully".to_string())
    }

    /// Sign and submit the current session
    #[wasm_bindgen]
    pub async fn sign_and_submit(&self) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.sign_and_submit()
            .map_err(|e| JsValue::from_str(&format!("Failed to sign and submit: {}", e)))?;
        
        Ok("Session signed and submitted successfully".to_string())
    }

    // ========== User Operations ==========

    /// Register a new user with private key
    #[wasm_bindgen]
    pub async fn register_user(&self, private_key_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let private_key: QHashOut<F> = serde_json::from_str(private_key_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse private key: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let public_key = session.register_user(private_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to register user: {}", e)))?;
        
        serde_json::to_string(&public_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize public key: {}", e)))
    }

    /// Add an existing user with private key
    #[wasm_bindgen]
    pub async fn add_user(&self, private_key_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let private_key: QHashOut<F> = serde_json::from_str(private_key_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse private key: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let public_key = session.add_user(private_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to add user: {}", e)))?;
        
        serde_json::to_string(&public_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize public key: {}", e)))
    }

    /// Switch to a different user
    #[wasm_bindgen]
    pub async fn switch_user(&self, pk_hash_json: &str) -> Result<(), JsValue> {
        self.ensure_initialized()?;
        
        let pk_hash: QHashOut<F> = serde_json::from_str(pk_hash_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse public key hash: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.switch_user(pk_hash)
            .map_err(|e| JsValue::from_str(&format!("Failed to switch user: {}", e)))
    }

    /// Get ZK public key for a private key
    #[wasm_bindgen]
    pub async fn get_zk_public_key(&self, private_key_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let private_key: QHashOut<F> = serde_json::from_str(private_key_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse private key: {}", e)))?;
        
        let session = self.wallet_session.read()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let zk_public_key = session.get_zk_public_key(private_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to get ZK public key: {}", e)))?;
        
        serde_json::to_string(&zk_public_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize ZK public key: {}", e)))
    }

    /// Generate a random keypair
    #[wasm_bindgen]
    pub async fn get_random_keypair(&self) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let session = self.wallet_session.read()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let keypair = session.get_random_keypair()
            .map_err(|e| JsValue::from_str(&format!("Failed to generate random keypair: {}", e)))?;
        
        serde_json::to_string(&keypair)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize keypair: {}", e)))
    }

    // ========== Contract Deployment ==========

    /// Deploy a contract with circuit definitions
    #[wasm_bindgen]
    pub async fn deploy_contract(&self, circuit_defs_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse circuit definitions: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        session.deploy_contract(circuit_defs)
            .map_err(|e| JsValue::from_str(&format!("Failed to deploy contract: {}", e)))?;
        
        Ok("Contract deployed successfully".to_string())
    }

    /// Get deploy contract command
    #[wasm_bindgen]
    pub async fn get_deploy_contract_cmd(&self, circuit_defs_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse circuit definitions: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let deploy_cmd = session.get_deploy_contract_cmd(circuit_defs)
            .map_err(|e| JsValue::from_str(&format!("Failed to get deploy contract command: {}", e)))?;
        
        serde_json::to_string(&deploy_cmd)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize deploy command: {}", e)))
    }

    // ========== Signing and Submission ==========

    /// Get signature hash for a network
    #[wasm_bindgen]
    pub async fn get_sighash(&self, network_magic: u64) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let session = self.wallet_session.read()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let sighash = session.get_sig_hash(network_magic)
            .map_err(|e| JsValue::from_str(&format!("Failed to get signature hash: {}", e)))?;
        
        serde_json::to_string(&sighash)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize signature hash: {}", e)))
    }

    /// Get ZK signature for a signature hash
    #[wasm_bindgen]
    pub async fn get_zk_signature(&self, sighash_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let sighash: QHashOut<F> = serde_json::from_str(sighash_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse signature hash: {}", e)))?;
        
        let session = self.wallet_session.read()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let zk_signature = session.get_zk_signature(sighash)
            .map_err(|e| JsValue::from_str(&format!("Failed to get ZK signature: {}", e)))?;
        
        serde_json::to_string(&zk_signature)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize ZK signature: {}", e)))
    }

    /// Get end cap proof for a signature proof
    #[wasm_bindgen]
    pub async fn get_end_cap_proof(&self, signature_proof_json: &str) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let signature_proof: types::ProofWithPublicInputs = serde_json::from_str(signature_proof_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse signature proof: {}", e)))?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let end_cap_proof = session.get_end_cap_proof(signature_proof)
            .map_err(|e| JsValue::from_str(&format!("Failed to get end cap proof: {}", e)))?;
        
        serde_json::to_string(&end_cap_proof)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize end cap proof: {}", e)))
    }

    /// Get user end cap input
    #[wasm_bindgen]
    pub async fn get_user_ec_input(&self) -> Result<String, JsValue> {
        self.ensure_initialized()?;
        
        let mut session = self.wallet_session.write()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire session lock: {}", e)))?;
        
        let user_ec_input = session.get_user_ec_input()
            .map_err(|e| JsValue::from_str(&format!("Failed to get user EC input: {}", e)))?;
        
        serde_json::to_string(&user_ec_input)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize user EC input: {}", e)))
    }

    // ========== Utility Methods ==========

    /// Ping method for testing connectivity
    #[wasm_bindgen]
    pub async fn ping(&self, message: &str) -> Result<String, JsValue> {
        Ok(message.chars().rev().collect::<String>())
    }

    /// Get result by ID
    #[wasm_bindgen]
    pub async fn get_result(&self, id_json: &str) -> Result<String, JsValue> {
        let id: Hash256 = serde_json::from_str(id_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse ID: {}", e)))?;
        
        let mut store = self.store.lock().unwrap();
        let result = store.get_result_and_clear(&id);
        
        match result {
            Some(data) => {
                let u8_bytes = U8Bytes(data);
                serde_json::to_string(&u8_bytes)
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
            }
            None => Err(JsValue::from_str("Result not found"))
        }
    }

    // ========== Private Helper Methods ==========

    fn ensure_initialized(&self) -> Result<(), JsValue> {
        if !self.is_initialized {
            return Err(JsValue::from_str("Prover not initialized. Call initialize() first."));
        }
        Ok(())
    }
}

// Global functions for WASM module initialization
#[wasm_bindgen(start)]
pub fn main() {
    utils::set_panic_hook();
    utils::init_logger();
    console::log_1(&"QED User Prover WASM module loaded".into());
}

/// Get the global prover instance (singleton pattern)
#[wasm_bindgen]
pub fn get_prover_instance() -> QEDUserProverWasm {
    unsafe {
        INIT_ONCE.call_once(|| {
            PROVER_INSTANCE = Some(QEDUserProverWasm::new());
        });
        
        // Clone the instance (this is safe because we're using Arc internally)
        PROVER_INSTANCE.as_ref().unwrap().clone()
    }
}

// Implement Clone for QEDUserProverWasm to support singleton pattern
impl Clone for QEDUserProverWasm {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            wallet_session: Arc::clone(&self.wallet_session),
            is_initialized: self.is_initialized,
        }
    }
}