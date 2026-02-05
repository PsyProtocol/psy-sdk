// WASM-specific bindings and exports
use wasm_bindgen::prelude::*;

// Import the console.log function from the web console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro to easily call console.log
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Initialize panic hook for better error messages in WASM
#[wasm_bindgen(start)]
pub fn main() {
    // Initialize panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Initialize wasm-logger for log crate compatibility
    wasm_logger::init(wasm_logger::Config::default());

    // Initialize tracing subscriber for WASM
    wasm_tracing::set_as_global_default();

    // Log initialization success
    tracing::info!("PSY Rust SDK WASM module initialized successfully with psy_prover support");
}

// Optional manual initialization function
#[wasm_bindgen]
pub fn init_logging() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    wasm_tracing::set_as_global_default();
    tracing::info!("PSY Rust SDK logging initialized manually");
}

// ================================
// Config functionality (from psy_config)
// ================================

#[wasm_bindgen]
pub struct WasmPsyConfig {
    inner: psy_config::PsyConfigGoldilocks,
}

#[wasm_bindgen]
impl WasmPsyConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<WasmPsyConfig, JsValue> {
        console_error_panic_hook::set_once();

        let inner = psy_config::PsyConfigGoldilocks::from_json(json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(WasmPsyConfig { inner })
    }

    #[wasm_bindgen(js_name = useNetwork)]
    pub fn use_network(&mut self, network_name: &str) -> Result<(), JsValue> {
        self.inner
            .use_network(network_name)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getCurrentNetwork)]
    pub fn get_current_network(&self) -> Result<String, JsValue> {
        let network = self
            .inner
            .get_current_network()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_json::to_string(network).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create using builder pattern (for more complex configurations)
    #[wasm_bindgen(js_name = builder)]
    pub fn builder() -> WasmPsyConfigBuilder {
        WasmPsyConfigBuilder::new()
    }

    #[wasm_bindgen(js_name = getNetworkJson)]
    pub fn get_network_json(&self, network_name: &str) -> Result<String, JsValue> {
        let network = self
            .inner
            .get_network(network_name)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_json::to_string(network).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = listNetworks)]
    pub fn list_networks(&self) -> Vec<String> {
        self.inner.list_networks().into_iter().cloned().collect()
    }

    #[wasm_bindgen(js_name = currentNetworkName)]
    pub fn current_network_name(&self) -> String {
        self.inner.current_network_name().to_string()
    }
}

/// WASM Builder for flexible configuration in browser/JS environments
#[wasm_bindgen]
pub struct WasmPsyConfigBuilder {
    inner: psy_config::PsyConfigBuilderGoldilocks,
}

#[wasm_bindgen]
impl WasmPsyConfigBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmPsyConfigBuilder {
        WasmPsyConfigBuilder {
            inner: psy_config::PsyConfigBuilderGoldilocks::new(),
        }
    }

    /// Set configuration from JSON string
    #[wasm_bindgen(js_name = json)]
    pub fn json(mut self, json: &str) -> WasmPsyConfigBuilder {
        self.inner = self.inner.json(json);
        self
    }

    /// Set initial network to use
    #[wasm_bindgen(js_name = network)]
    pub fn network(mut self, network: &str) -> WasmPsyConfigBuilder {
        self.inner = self.inner.network(network);
        self
    }

    /// Build the configuration
    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<WasmPsyConfig, JsValue> {
        let inner = self
            .inner
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(WasmPsyConfig { inner })
    }
}

// Export constants for WASM
#[wasm_bindgen]
pub struct WasmConstants;

#[wasm_bindgen]
impl WasmConstants {
    #[wasm_bindgen(getter)]
    pub fn global_user_tree_height() -> u8 {
        psy_config::network_constants::GLOBAL_USER_TREE_HEIGHT
    }

    #[wasm_bindgen(getter)]
    pub fn coordinator_user_tree_height() -> u8 {
        psy_config::network_constants::COORDINATOR_USER_TREE_HEIGHT
    }

    #[wasm_bindgen(getter)]
    pub fn realm_user_tree_height() -> u8 {
        psy_config::network_constants::REALM_USER_TREE_HEIGHT
    }

    #[wasm_bindgen(getter)]
    pub fn group_realm_height() -> u8 {
        psy_config::network_constants::GROUP_REALM_HEIGHT
    }

    #[wasm_bindgen(getter)]
    pub fn users_per_realm() -> u64 {
        psy_config::network_constants::USERS_PER_REALM
    }

    #[wasm_bindgen(getter)]
    pub fn native_currency_decimal() -> u8 {
        psy_config::network_constants::NATIVE_CURRENCY_DECIMAL
    }

    #[wasm_bindgen(getter)]
    pub fn native_currency() -> String {
        psy_config::network_constants::NATIVE_CURRENCY.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn native_currency_name() -> String {
        psy_config::network_constants::NATIVE_CURRENCY_NAME.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn register_user_fee() -> u64 {
        psy_config::network_constants::REGISTER_USER_FEE
    }

    #[wasm_bindgen(getter)]
    pub fn deploy_contract_fee() -> u64 {
        psy_config::network_constants::DEPLOY_CONTRACT_FEE
    }

    #[wasm_bindgen(getter)]
    pub fn guta_fee() -> u64 {
        psy_config::network_constants::GUTA_FEE
    }

    #[wasm_bindgen(getter)]
    pub fn current_network() -> String {
        psy_config::network_constants::CURRENT_NETWORK.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn config_path() -> String {
        "config.json".to_string() // Default config path
    }

    #[wasm_bindgen(getter)]
    pub fn coordinator_rpc_url() -> String {
        psy_config::network_constants::COORDINATOR_RPC_URL.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn realm_rpc_urls() -> Vec<String> {
        psy_config::network_constants::REALM_RPC_URLS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get all constants as a JSON string for easier JS consumption
    #[wasm_bindgen(js_name = getAllConstants)]
    pub fn get_all_constants() -> Result<String, JsValue> {
        // Read the generated JSON constants file
        let constants_json = serde_json::to_string_pretty(&serde_json::json!({
            "realm_rpc_urls": Self::realm_rpc_urls(),
            "coordinator_rpc_url": Self::coordinator_rpc_url(),
            "psy_network_magic": psy_config::network_constants::PSY_NETWORK_MAGIC,
            "guta_rewards_tree_max_height": psy_common::job::id::GUTA_REWARDS_TREE_MAX_HEIGHT,
            "ups_session_proof_tree_height": psy_config::network_constants::UPS_SESSION_PROOF_TREE_HEIGHT,
            "ups_circuit_whitelist_tree_height": psy_config::network_constants::UPS_CIRCUIT_WHITELIST_TREE_HEIGHT,
            "realm_user_tree_height": psy_config::network_constants::REALM_USER_TREE_HEIGHT,
            "max_contract_state_tree_height": psy_config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT,
            "token_contract_id": psy_config::network_constants::TOKEN_CONTRACT_ID,
            "users_per_realm": psy_config::network_constants::USERS_PER_REALM,
            "default_user_state_tree_root_u64": psy_config::network_constants::DEFAULT_USER_STATE_TREE_ROOT_U64
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(constants_json.to_string())
    }
}

// ================================
// WasmRpcServer (Actual implementation for WASM)
// ================================

use std::str::FromStr;

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::{
    args::{ContractCallArgs, ContractCallData, DPNSoftwareDefinedCallData, SignType},
    data::{base_types::hash256::Hash256, qhashout::QHashOut, u8bytes::U8Bytes},
};
use psy_crypto::signature::zk::data::ZKPublicKeyInfo;
use psy_data::{
    guta::end_cap_input::SubmitUserEndCapNonProofInput,
    qblock::cmds::deploy_contract::{QBCDeployContract, QContractABI},
};
use psy_prover::{
    local::store::UserProverWorkerStore,
    session::{WalletKeyPair, WalletSession},
};
use psy_provider::provider::NetworkConfig as RpcConfig;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

#[wasm_bindgen]
pub struct WasmRpcServer {
    store: UserProverWorkerStore,
    wallet_session: WalletSession,
}

#[wasm_bindgen]
impl WasmRpcServer {
    #[wasm_bindgen(constructor)]
    pub async fn new(rpc_config_json: &str) -> Result<WasmRpcServer, JsError> {
        let rpc_config: RpcConfig<F> = serde_json::from_str(rpc_config_json)
            .map_err(|e| JsError::new(&format!("Parse RPC config error: {}", e)))?;

        let wallet_session = WalletSession::new(&rpc_config)
            .await
            .map_err(|e| JsError::new(&format!("Create wallet session error: {}", e)))?;

        Ok(WasmRpcServer {
            store: UserProverWorkerStore::new(),
            wallet_session,
        })
    }

    #[wasm_bindgen]
    pub async fn exec_contract_call_json(
        &mut self,
        pk_hash: &str,
        call_data_json: &str,
    ) -> Result<String, JsError> {
        let call_data: ContractCallData = serde_json::from_str(call_data_json)
            .map_err(|e| JsError::new(&format!("Parse call data JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let end_user_leaf_hash = self
            .wallet_session
            .exec_contract_call(pk_hash, call_data)
            .await
            .map_err(|e| JsError::new(&format!("Error exec calls error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    // Local proving operations
    #[wasm_bindgen]
    pub async fn start_session(&self, pk_hash: &str) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;
        self.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("Start session error: {}", e)))?;
        Ok("start session".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_call_json(
        &mut self,
        pk_hash: &str,
        contract_call_json: &str,
    ) -> Result<String, JsError> {
        let contract_call_arg: ContractCallArgs = serde_json::from_str(contract_call_json)
            .map_err(|e| JsError::new(&format!("Parse contract call JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session
            .prove_contract_call(pk_hash, vec![contract_call_arg])
            .await
            .map_err(|e| JsError::new(&format!("Prove contract call error: {}", e)))?;
        Ok("prove contract call".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_calls_json(
        &mut self,
        pk_hash: &str,
        contract_calls_json: &str,
    ) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> =
            serde_json::from_str(contract_calls_json)
                .map_err(|e| JsError::new(&format!("Parse contract calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session
            .prove_contract_call(pk_hash, contract_call_args)
            .await
            .map_err(|e| JsError::new(&format!("Prove contract calls error: {}", e)))?;
        Ok("prove contract calls".to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit(
        &self,
        pk_hash: &str,
        sign_data: Option<String>,
    ) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let software_defined_call = sign_data
            .map(|data| serde_json::from_str::<psy_common::args::DPNSoftwareDefinedCallData>(&data))
            .transpose()
            .map_err(|e| JsError::new(&format!("Parse sign data error: {}", e)))?;

        let end_user_leaf_hash = self
            .wallet_session
            .sign_and_submit(pk_hash, software_defined_call)
            .await
            .map_err(|e| JsError::new(&format!("Sign and submit error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    // User operations
    #[wasm_bindgen]
    pub async fn register_user(
        &mut self,
        private_key_str: &str,
        sign_type: &str,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let fingerprint = match sign_type {
            "zk" => psy_prover::wallet::memory_wallet::get_zk_fingerprint(),
            "secp256k1" => psy_prover::wallet::memory_wallet::get_secp256k1_fingerprint(),
            _ => {
                return Err(JsError::new(&format!(
                    "Unsupported sign type: {}",
                    sign_type
                )))
            }
        };

        let pk_hash = self
            .wallet_session
            .register_user(private_key, fingerprint)
            .await
            .map_err(|e| JsError::new(&format!("Register user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user(
        &mut self,
        private_key_str: &str,
        sign_type: &str,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let fingerprint = match sign_type {
            "zk" => psy_prover::wallet::memory_wallet::get_zk_fingerprint(),
            "secp256k1" => psy_prover::wallet::memory_wallet::get_secp256k1_fingerprint(),
            _ => {
                return Err(JsError::new(&format!(
                    "Unsupported sign type: {}",
                    sign_type
                )))
            }
        };

        let pk_hash = self
            .wallet_session
            .add_user(private_key, fingerprint)
            .await
            .map_err(|e| JsError::new(&format!("Add user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn get_zk_public_key_json(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
        let public_key = self
            .wallet_session
            .get_zk_public_key(private_key)
            .await
            .map_err(|e| JsError::new(&format!("Get ZK public key error: {}", e)))?;
        serde_json::to_string(&public_key)
            .map_err(|e| JsError::new(&format!("Serialize public key error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn get_random_keypair_json(&self) -> Result<String, JsError> {
        let keypair = self
            .wallet_session
            .get_random_keypair()
            .await
            .map_err(|e| JsError::new(&format!("Get random keypair error: {}", e)))?;
        serde_json::to_string(&keypair)
            .map_err(|e| JsError::new(&format!("Serialize keypair error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn deploy_contract_json(
        &self,
        deployer: &str,
        circuit_defs_json: &str,
    ) -> Result<String, JsError> {
        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(circuit_defs_json)
                .map_err(|e| JsError::new(&format!("Parse circuit defs JSON error: {}", e)))?;

        let contract_uuid = self
            .wallet_session
            .deploy_contract(deployer, circuit_defs)
            .await
            .map_err(|e| JsError::new(&format!("Deploy contract error: {}", e)))?;
        Ok(contract_uuid)
    }

    #[wasm_bindgen]
    pub fn get_deploy_contract_cmd_json(
        &self,
        deployer: &str,
        circuit_defs_json: &str,
    ) -> Result<String, JsError> {
        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(circuit_defs_json)
                .map_err(|e| JsError::new(&format!("Parse circuit defs JSON error: {}", e)))?;

        let cmd = self
            .wallet_session
            .get_deploy_contract_cmd(deployer, circuit_defs)
            .map_err(|e| JsError::new(&format!("Get deploy contract cmd error: {}", e)))?;
        serde_json::to_string(&cmd)
            .map_err(|e| JsError::new(&format!("Serialize deploy contract cmd error: {}", e)))
    }

    // Test function
    #[wasm_bindgen]
    pub fn ping(&self, message: &str) -> Result<String, JsError> {
        Ok(format!("Pong: {}", message))
    }

    #[wasm_bindgen]
    pub fn get_result(&self, id_str: &str) -> Result<Vec<u8>, JsError> {
        let id = Hash256::from_hex_string(id_str)
            .map_err(|e| JsError::new(&format!("Invalid ID format: {:?}", e)))?;

        match self.store.get_result(&id) {
            Some(proof) => Ok(proof.clone()),
            None => Err(JsError::new(&format!("Proof not found for ID: {}", id_str))),
        }
    }
}
