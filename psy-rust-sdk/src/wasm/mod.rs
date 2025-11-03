// WASM-specific bindings and exports
// This is the unified WASM interface for all PSY functionality
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
    tracing::info!("PSY Rust SDK WASM module initialized successfully");
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

        let inner = psy_config::PsyConfigGoldilocks::from_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(WasmPsyConfig { inner })
    }

    #[wasm_bindgen(js_name = useNetwork)]
    pub fn use_network(&mut self, network_name: &str) -> Result<(), JsValue> {
        self.inner.use_network(network_name).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getCurrentNetwork)]
    pub fn get_current_network(&self) -> Result<String, JsValue> {
        let network = self.inner.get_current_network().map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_json::to_string(network).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create using builder pattern (for more complex configurations)
    #[wasm_bindgen(js_name = builder)]
    pub fn builder() -> WasmPsyConfigBuilder {
        WasmPsyConfigBuilder::new()
    }

    #[wasm_bindgen(js_name = getNetworkJson)]
    pub fn get_network_json(&self, network_name: &str) -> Result<String, JsValue> {
        let network = self.inner.get_network(network_name).map_err(|e| JsValue::from_str(&e.to_string()))?;

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
        let inner = self.inner.build().map_err(|e| JsValue::from_str(&e.to_string()))?;

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
        psy_config::network_constants::CONFIG_PATH.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn coordinator_rpc_url() -> String {
        psy_config::network_constants::COORDINATOR_RPC_URL.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn realm_rpc_urls() -> Vec<String> {
        psy_config::network_constants::REALM_RPC_URLS.iter().map(|s| s.to_string()).collect()
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
// WasmRpcServer (re-implemented for WASM compatibility)
// ================================

use std::str::FromStr;

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use psy_common::data::{base_types::hash256::Hash256, qhashout::QHashOut};
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

// Import types needed for the original implementation
// WASM-compatible types and imports
#[cfg(target_arch = "wasm32")]
use psy_common::args::{ContractCallArgs, SignType};
use psy_common::data::u8bytes::U8Bytes;
use psy_crypto::signature::zk::data::ZKPublicKeyInfo;
use psy_data::{args::JobInfo, guta::end_cap_input::SubmitUserEndCapNonProofInput, qblock::cmds::deploy_contract::QBCDeployContract};
#[cfg(not(target_arch = "wasm32"))]
use psy_prover::local::args::{ContractCallArgs, JobInfo};
// Conditional imports - only available in native
#[cfg(not(target_arch = "wasm32"))]
use psy_prover::local::store::UserProverWorkerStore;
#[cfg(not(target_arch = "wasm32"))]
use psy_prover::session::{WalletKeyPair, WalletSession};
#[cfg(not(target_arch = "wasm32"))]
use psy_provider::provider::NetworkConfig as RpcConfig;

// WASM-compatible implementations
#[cfg(target_arch = "wasm32")]
pub struct UserProverWorkerStore;

#[cfg(target_arch = "wasm32")]
impl UserProverWorkerStore {
    pub fn new() -> Self {
        UserProverWorkerStore
    }

    pub fn get_result(&self, _id: &Hash256) -> Option<Vec<u8>> {
        None // WASM implementation would use different storage
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WalletSession;

#[cfg(target_arch = "wasm32")]
impl WalletSession {
    pub async fn new(_rpc_config: &RpcConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(WalletSession)
    }

    pub async fn exec_contract_call(
        &mut self,
        _pk_hash: QHashOut<F>,
        _contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("WASM exec_contract_call not implemented".into())
    }

    pub async fn get_claim_rewards_call_args(&self, _job_infos: Vec<JobInfo>) -> Result<Vec<ContractCallArgs>, Box<dyn std::error::Error>> {
        Err("WASM get_claim_rewards_call_args not implemented".into())
    }

    pub async fn claim_rewards(&mut self, _pk_hash: QHashOut<F>, _job_infos: Vec<JobInfo>) -> Result<(), Box<dyn std::error::Error>> {
        Err("WASM claim_rewards not implemented".into())
    }

    pub async fn start_session(&self, _pk_hash: QHashOut<F>) -> Result<(), Box<dyn std::error::Error>> {
        Err("WASM start_session not implemented".into())
    }

    pub async fn prove_contract_call(
        &mut self,
        _pk_hash: QHashOut<F>,
        _contract_call_arg: ContractCallArgs,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("WASM prove_contract_call not implemented".into())
    }

    pub async fn prove_contract_calls(
        &mut self,
        _pk_hash: QHashOut<F>,
        _contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("WASM prove_contract_calls not implemented".into())
    }

    pub async fn sign_and_submit(&self, _pk_hash: QHashOut<F>) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM sign_and_submit not implemented".into())
    }

    pub async fn sign_and_submit_with_sign_data(
        &mut self,
        _pk_hash: QHashOut<F>,
        _sign_data: Option<serde_json::Value>,
    ) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM sign_and_submit_with_sign_data not implemented".into())
    }

    pub async fn register_user(&mut self, _private_key: QHashOut<F>) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM register_user not implemented".into())
    }

    pub async fn register_user_with_type(
        &mut self,
        _private_key: QHashOut<F>,
        _sign_type: SignType,
        _fingerprint: Option<QHashOut<F>>,
    ) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM register_user_with_type not implemented".into())
    }

    pub async fn add_user(&mut self, _private_key: QHashOut<F>) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM add_user not implemented".into())
    }

    pub async fn add_user_with_type(
        &mut self,
        _private_key: QHashOut<F>,
        _sign_type: SignType,
        _fingerprint: Option<QHashOut<F>>,
    ) -> Result<QHashOut<F>, Box<dyn std::error::Error>> {
        Err("WASM add_user_with_type not implemented".into())
    }

    pub async fn get_zk_public_key(&self, _private_key: QHashOut<F>) -> Result<ZKPublicKeyInfo<F>, Box<dyn std::error::Error>> {
        Err("WASM get_zk_public_key not implemented".into())
    }

    pub async fn get_random_keypair(&self) -> Result<WalletKeyPair, Box<dyn std::error::Error>> {
        Err("WASM get_random_keypair not implemented".into())
    }

    pub async fn deploy_contract(
        &self,
        _deployer: QHashOut<F>,
        _circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Err("WASM deploy_contract not implemented".into())
    }

    pub fn get_deploy_contract_cmd(
        &self,
        _deployer: QHashOut<F>,
        _circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<F>, Box<dyn std::error::Error>> {
        Err("WASM get_deploy_contract_cmd not implemented".into())
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletKeyPair;

#[cfg(target_arch = "wasm32")]
pub struct RpcConfig;

#[cfg(target_arch = "wasm32")]
impl<'de> serde::Deserialize<'de> for RpcConfig {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(RpcConfig)
    }
}

#[wasm_bindgen]
pub struct WasmRpcServer {
    store: UserProverWorkerStore,
    wallet_session: WalletSession,
}

#[wasm_bindgen]
impl WasmRpcServer {
    #[wasm_bindgen(constructor)]
    pub async fn new(rpc_config_json: &str) -> Result<WasmRpcServer, JsValue> {
        let rpc_config: RpcConfig =
            serde_json::from_str(rpc_config_json).map_err(|e| JsValue::from_str(&format!("Parse RPC config error: {}", e)))?;

        let wallet_session = WalletSession::new(&rpc_config)
            .await
            .map_err(|e| JsValue::from_str(&format!("Create wallet session error: {}", e)))?;

        Ok(WasmRpcServer {
            store: UserProverWorkerStore::new(),
            wallet_session,
        })
    }

    #[wasm_bindgen]
    pub async fn exec_contract_call_json(&mut self, pk_hash: &str, contract_calls_json: &str) -> Result<String, JsValue> {
        let contract_call_args: Vec<ContractCallArgs> =
            serde_json::from_str(contract_calls_json).map_err(|e| JsValue::from_str(&format!("Parse exec calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session
            .exec_contract_call(pk_hash, contract_call_args)
            .await
            .map_err(|e| JsValue::from_str(&format!("Error exec calls error: {}", e)))?;
        Ok("start session".to_string())
    }

    #[wasm_bindgen]
    pub async fn exec_contract_call_with_sign_data_json(
        &mut self,
        _pk_hash: &str,
        _contract_calls_json: &str,
        _sign_data: Option<String>,
    ) -> Result<String, JsValue> {
        Err(JsValue::from_str("exec_contract_call_with_sign_data_json not implemented in WASM"))
    }

    #[wasm_bindgen]
    pub async fn get_claim_rewards_call_args_json(&self, job_infos_json: &str) -> Result<String, JsValue> {
        let job_infos: Vec<JobInfo> =
            serde_json::from_str(job_infos_json).map_err(|e| JsValue::from_str(&format!("Parse job infos JSON error: {}", e)))?;

        let contract_call_args = self
            .wallet_session
            .get_claim_rewards_call_args(job_infos)
            .await
            .map_err(|e| JsValue::from_str(&format!("Error get claim rewards call args error: {}", e)))?;
        Ok(serde_json::to_string(&contract_call_args).map_err(|e| JsValue::from_str(&e.to_string()))?)
    }

    #[wasm_bindgen]
    pub async fn claim_rewards_json(&mut self, pk_hash: &str, job_infos_json: &str) -> Result<String, JsValue> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;

        let job_infos: Vec<JobInfo> =
            serde_json::from_str(job_infos_json).map_err(|e| JsValue::from_str(&format!("Parse job infos JSON error: {}", e)))?;

        self.wallet_session
            .claim_rewards(pk_hash, job_infos)
            .await
            .map_err(|e| JsValue::from_str(&format!("Error exec calls error: {}", e)))?;
        Ok("claim_rewards".to_string())
    }

    #[wasm_bindgen]
    pub async fn start_session(&self, pk_hash: &str) -> Result<String, JsValue> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;
        self.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| JsValue::from_str(&format!("Start session error: {}", e)))?;
        Ok("start session".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_call_json(&mut self, pk_hash: &str, contract_call_json: &str) -> Result<String, JsValue> {
        let contract_call_arg: ContractCallArgs =
            serde_json::from_str(contract_call_json).map_err(|e| JsValue::from_str(&format!("Parse contract call JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session
            .prove_contract_call(pk_hash, contract_call_arg)
            .await
            .map_err(|e| JsValue::from_str(&format!("Prove contract call error: {}", e)))?;
        Ok("prove contract call".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_calls_json(&mut self, pk_hash: &str, contract_calls_json: &str) -> Result<String, JsValue> {
        let contract_call_args: Vec<ContractCallArgs> =
            serde_json::from_str(contract_calls_json).map_err(|e| JsValue::from_str(&format!("Parse contract calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session
            .prove_contract_calls(pk_hash, contract_call_args)
            .await
            .map_err(|e| JsValue::from_str(&format!("Prove contract calls error: {}", e)))?;
        Ok("prove contract calls".to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit(&self, pk_hash: &str) -> Result<String, JsValue> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;
        let end_user_leaf_hash = self
            .wallet_session
            .sign_and_submit(pk_hash)
            .await
            .map_err(|e| JsValue::from_str(&format!("Sign and submit error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit_with_sign_data(&mut self, pk_hash: &str, sign_data: Option<String>) -> Result<String, JsValue> {
        let sign_data = sign_data
            .map(|s| serde_json::from_str(&s).map_err(|e| JsValue::from_str(&format!("Parse sign data error: {}", e))))
            .transpose()?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash).map_err(|e| JsValue::from_str(&format!("Parse public key hash error: {}", e)))?;

        let end_user_leaf_hash = self
            .wallet_session
            .sign_and_submit_with_sign_data(pk_hash, sign_data)
            .await
            .map_err(|e| JsValue::from_str(&format!("Sign and submit with sign data error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn register_user(&mut self, private_key_str: &str) -> Result<String, JsValue> {
        let private_key = QHashOut::<F>::from_str(private_key_str).map_err(|e| JsValue::from_str(&format!("Parse private key error: {}", e)))?;
        let pk_hash = self
            .wallet_session
            .register_user(private_key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Register user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn register_user_with_type(&mut self, private_key: &str, sign_type: &str, fingerprint: Option<String>) -> Result<String, JsValue> {
        let private_key = QHashOut::<F>::from_str(private_key).map_err(|e| JsValue::from_str(&format!("Parse private key error: {}", e)))?;
        let sign_type = SignType::from_str(sign_type, true).map_err(|e| JsValue::from_str(&format!("Parse sign type error: {}", e)))?;

        let fingerprint = fingerprint
            .map(|f| QHashOut::<F>::from_str(&f).map_err(|e| JsValue::from_str(&format!("Parse fingerprint error: {}", e))))
            .transpose()?;

        let pk_hash = self
            .wallet_session
            .register_user_with_type(private_key, sign_type, fingerprint)
            .await
            .map_err(|e| JsValue::from_str(&format!("Register user with type error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user(&mut self, private_key_str: &str) -> Result<String, JsValue> {
        let private_key = QHashOut::<F>::from_str(private_key_str).map_err(|e| JsValue::from_str(&format!("Parse private key error: {}", e)))?;
        let pk_hash = self
            .wallet_session
            .add_user(private_key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Add user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user_with_type(&mut self, private_key_str: &str, sign_type: &str, fingerprint: Option<String>) -> Result<String, JsValue> {
        let private_key = QHashOut::<F>::from_str(private_key_str).map_err(|e| JsValue::from_str(&format!("Parse private key error: {}", e)))?;

        let sign_type = SignType::from_str(sign_type, true).map_err(|e| JsValue::from_str(&format!("Parse sign type error: {}", e)))?;

        let fingerprint = fingerprint
            .map(|f| QHashOut::<F>::from_str(&f).map_err(|e| JsValue::from_str(&format!("Parse fingerprint error: {}", e))))
            .transpose()?;

        let pk_hash = self
            .wallet_session
            .add_user_with_type(private_key, sign_type, fingerprint)
            .await
            .map_err(|e| JsValue::from_str(&format!("Add user with sign type error: {}", e)))?;

        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn get_zk_public_key_json(&self, private_key_str: &str) -> Result<String, JsValue> {
        let private_key = QHashOut::<F>::from_str(private_key_str).map_err(|e| JsValue::from_str(&format!("Parse private key error: {}", e)))?;
        let public_key = self
            .wallet_session
            .get_zk_public_key(private_key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Get ZK public key error: {}", e)))?;
        serde_json::to_string(&public_key).map_err(|e| JsValue::from_str(&format!("Serialize public key error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn get_random_keypair_json(&self) -> Result<String, JsValue> {
        let keypair = self
            .wallet_session
            .get_random_keypair()
            .await
            .map_err(|e| JsValue::from_str(&format!("Get random keypair error: {}", e)))?;
        serde_json::to_string(&keypair).map_err(|e| JsValue::from_str(&format!("Serialize keypair error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn deploy_contract_json(&self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsValue> {
        let deployer = QHashOut::<F>::from_str(deployer).map_err(|e| JsValue::from_str(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(circuit_defs_json).map_err(|e| JsValue::from_str(&format!("Parse circuit defs JSON error: {}", e)))?;

        let contract_uuid = self
            .wallet_session
            .deploy_contract(deployer, circuit_defs)
            .await
            .map_err(|e| JsValue::from_str(&format!("Deploy contract error: {}", e)))?;
        Ok(contract_uuid)
    }

    #[wasm_bindgen]
    pub fn get_deploy_contract_cmd_json(&self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsValue> {
        let deployer = QHashOut::<F>::from_str(deployer).map_err(|e| JsValue::from_str(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(circuit_defs_json).map_err(|e| JsValue::from_str(&format!("Parse circuit defs JSON error: {}", e)))?;

        let cmd = self
            .wallet_session
            .get_deploy_contract_cmd(deployer, circuit_defs)
            .map_err(|e| JsValue::from_str(&format!("Get deploy contract cmd error: {}", e)))?;
        serde_json::to_string(&cmd).map_err(|e| JsValue::from_str(&format!("Serialize deploy contract cmd error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn ping(&self, message: &str) -> String {
        format!("Pong: {}", message)
    }

    #[wasm_bindgen]
    pub fn get_result(&self, id_str: &str) -> Result<Vec<u8>, JsValue> {
        let id = Hash256::from_hex_string(id_str).map_err(|e| JsValue::from_str(&format!("Invalid ID format: {:?}", e)))?;

        match self.store.get_result(&id) {
            Some(proof) => Ok(proof.clone()),
            None => Err(JsValue::from_str(&format!("Proof not found for ID: {}", id_str))),
        }
    }
}
