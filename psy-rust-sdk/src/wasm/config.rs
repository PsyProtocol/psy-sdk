// ================================
// Config functionality (from psy_config)
// ================================

use wasm_bindgen::prelude::*;

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
            "token_contract_state_tree_height": psy_config::network_constants::TOKEN_CONTRACT_STATE_TREE_HEIGHT,
            "token_contract_id": psy_config::network_constants::TOKEN_CONTRACT_ID,
            "users_per_realm": psy_config::network_constants::USERS_PER_REALM,
            "default_user_state_tree_root_u64": psy_config::network_constants::DEFAULT_USER_STATE_TREE_ROOT_U64
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(constants_json.to_string())
    }
}
