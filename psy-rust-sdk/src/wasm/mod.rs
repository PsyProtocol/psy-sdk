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

async fn async_sleep_ms(ms: i32) {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{global, Reflect};
        use js_sys::{Function, Promise};
        use wasm_bindgen::{closure::Closure, JsCast, JsValue};

        let promise = Promise::new(&mut |resolve: Function, _reject: Function| {
            let global_obj = global();
            let resolve_for_cb = resolve.clone();
            let cb = Closure::<dyn FnMut()>::once(move || {
                let _ = resolve_for_cb.call0(&JsValue::NULL);
            });
            if let Some(set_timeout) = Reflect::get(&global_obj, &JsValue::from_str("setTimeout"))
                .ok()
                .and_then(|v| v.dyn_into::<Function>().ok())
            {
                let _ = set_timeout.call2(
                    &global_obj,
                    cb.as_ref().unchecked_ref(),
                    &JsValue::from_f64(ms as f64),
                );
            } else {
                let _ = resolve.call0(&JsValue::NULL);
            }
            cb.forget();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
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

    /// Inject an external PrivateNoteInclusion proof into the current session tree.
    /// Returns JSON: { "leaf_index": u64, "siblings": [[u64;4]] }
    #[wasm_bindgen]
    pub async fn add_external_proof_json(
        &self,
        pk_hash: &str,
        note_proof_bincode_b64: &str,
    ) -> Result<String, JsError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;
        use psy_config::network_constants::{
            GLOBAL_USER_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };

        const NOTE_TREE_HEIGHT: usize = 20;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse pk_hash error: {}", e)))?;

        let proof_bytes = BASE64
            .decode(note_proof_bincode_b64.as_bytes())
            .map_err(|e| JsError::new(&format!("base64 decode error: {}", e)))?;
        let proof: ProofWithPublicInputs<F, C, D> = bincode::deserialize(&proof_bytes)
            .map_err(|e| JsError::new(&format!("bincode deserialize error: {}", e)))?;

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let fingerprint = circuit.get_fingerprint();
        let verifier_data = circuit.get_verifier_config_ref().clone();

        let (leaf_index, siblings) = self
            .wallet_session
            .add_external_proof_with_siblings(pk_hash, fingerprint, proof, verifier_data)
            .await
            .map_err(|e| JsError::new(&format!("add_external_proof error: {}", e)))?;

        // Encode as decimal strings to avoid JS precision loss for large u64 values
        let siblings_str: Vec<[String; 4]> = siblings
            .iter()
            .map(|s| {
                [
                    s[0].to_string(),
                    s[1].to_string(),
                    s[2].to_string(),
                    s[3].to_string(),
                ]
            })
            .collect();

        let result = serde_json::json!({
            "leaf_index": leaf_index,
            "siblings": siblings_str,
        });
        Ok(result.to_string())
    }

    /// Generate a PrivateNoteInclusion ZK proof and return the full NoteProofOutput as JSON.
    ///
    /// Inputs (all u64 arrays as JSON arrays of decimal strings to avoid JS precision loss):
    ///   pk_hash            - sender's ZK public key (hex QHashOut)
    ///   owner_json         - receiver's shield address as JSON array of 4 decimal strings
    ///   amount             - transfer amount (u64 as decimal string)
    ///   note_secret_hash_json - randomness used in commitment, JSON array of 4 decimal strings
    ///   nullifier_secret_json - nullifier secret, JSON array of 4 decimal strings
    ///   contract_id        - contract ID (u64 as decimal string)
    ///   note_root_slot     - note root slot index (u64 as decimal string)
    ///   checkpoint_id      - pre-submit checkpoint ID (u64 as decimal string, "0" = latest)
    ///
    /// Returns JSON matching NoteProofOutput.
    #[wasm_bindgen]
    pub async fn prove_private_note_inclusion_json(
        &self,
        pk_hash: &str,
        owner_json: &str,
        amount: &str,
        note_secret_hash_json: &str,
        nullifier_secret_json: &str,
        contract_id: &str,
        note_root_slot: &str,
        checkpoint_id: &str,
    ) -> Result<String, JsError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use plonky2::field::types::{Field, PrimeField64};
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;
        use psy_config::network_constants::{
            GLOBAL_USER_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };
        use psy_crypto::hash::traits::{
            hasher::{FieldQHasher, PoseidonHasher},
            qhashable::QFieldHashable,
        };
        use psy_crypto::hash::merkle::core::MerkleProofCore;
        use psy_data::privacy::private_note_inclusion::PrivateNoteInclusionInput;
        use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
        use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;

        const NOTE_TREE_HEIGHT: usize = 20;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        // Helper: parse a JSON array of 4 decimal strings into [u64;4]
        let parse_u64x4 = |json: &str| -> Result<[u64; 4], JsError> {
            let arr: [String; 4] = serde_json::from_str(json)
                .map_err(|e| JsError::new(&format!("parse array error: {}", e)))?;
            Ok([
                arr[0].parse::<u64>().map_err(|e| JsError::new(&e.to_string()))?,
                arr[1].parse::<u64>().map_err(|e| JsError::new(&e.to_string()))?,
                arr[2].parse::<u64>().map_err(|e| JsError::new(&e.to_string()))?,
                arr[3].parse::<u64>().map_err(|e| JsError::new(&e.to_string()))?,
            ])
        };

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("parse pk_hash: {}", e)))?;
        let owner_u64 = parse_u64x4(owner_json)?;
        let amount_val: u64 = amount.parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let note_secret_hash_u64 = parse_u64x4(note_secret_hash_json)?;
        let nullifier_secret_u64 = parse_u64x4(nullifier_secret_json)?;
        let contract_id_val: u64 = contract_id.parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let note_root_slot_val: u64 = note_root_slot.parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let checkpoint_before_raw: u64 = checkpoint_id.parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;

        let owner = QHashOut::<F>::from_values(owner_u64[0], owner_u64[1], owner_u64[2], owner_u64[3]);
        let note_secret_hash = QHashOut::<F>::from_values(
            note_secret_hash_u64[0], note_secret_hash_u64[1],
            note_secret_hash_u64[2], note_secret_hash_u64[3],
        );
        let nullifier_secret = QHashOut::<F>::from_values(
            nullifier_secret_u64[0], nullifier_secret_u64[1],
            nullifier_secret_u64[2], nullifier_secret_u64[3],
        );

        let provider = self.wallet_session.st_provider.clone();

        // Resolve checkpoint_before (0 = latest).
        let checkpoint_before = if checkpoint_before_raw == 0 {
            provider.get_latest_block_state().await
                .map_err(|e| JsError::new(&format!("get_latest_block_state: {}", e)))?
                .checkpoint_id
        } else {
            checkpoint_before_raw
        };

        // Get sender's user_id from public key
        let user_ids = provider.get_user_ids_for_public_key(pk_hash).await
            .map_err(|e| JsError::new(&format!("get_user_ids_for_public_key: {}", e)))?;
        let sender_user_id = *user_ids.first()
            .ok_or_else(|| JsError::new("No user ID found for public key"))?;

        let user_provider = provider.with_user_id_owned(sender_user_id);

        // Build note membership proof from the pre-submit checkpoint.
        // note_count is at slot note_root_slot - 1, elements[3] is the note index.
        let note_count_slot = note_root_slot_val.saturating_sub(1);
        let note_count_proof = user_provider.get_user_contract_state_tree_merkle_proof(
            checkpoint_before, sender_user_id, contract_id_val as u32,
            MAX_CONTRACT_STATE_TREE_HEIGHT as u8, note_count_slot,
        ).await
            .map_err(|e| JsError::new(&format!("note_count_proof: {}", e)))?;
        let note_index = note_count_proof.value.0.elements[3].to_canonical_u64();

        // Collect last_path (levels 0..NOTE_TREE_HEIGHT) from slots note_root_slot+1..
        let mut last_path: Vec<QHashOut<F>> = Vec::with_capacity(NOTE_TREE_HEIGHT);
        for level in 0..NOTE_TREE_HEIGHT as u64 {
            let slot = note_root_slot_val + 1 + level;
            let proof = user_provider.get_user_contract_state_tree_merkle_proof(
                checkpoint_before, sender_user_id, contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8, slot,
            ).await
                .map_err(|e| JsError::new(&format!("last_path[{}]: {}", level, e)))?;
            last_path.push(proof.value);
        }

        // Build commitment and Merkle siblings
        let value_hash = QHashOut::<F>::from_values(amount_val, 0, 0, 0);
        let inner = PoseidonHasher::q_two_to_one(owner, value_hash);
        let commitment = PoseidonHasher::q_two_to_one(inner, note_secret_hash);

        let mut siblings: Vec<QHashOut<F>> = Vec::with_capacity(NOTE_TREE_HEIGHT);
        let mut zero = QHashOut::<F>::from_values(0, 0, 0, 0);
        for level in 0..NOTE_TREE_HEIGHT {
            let bit = (note_index >> level) & 1;
            if bit == 0 {
                siblings.push(zero);
            } else {
                siblings.push(last_path[level]);
            }
            zero = PoseidonHasher::q_two_to_one(zero, zero);
        }
        let note_membership_proof = MerkleProofCore::new_from_params::<PoseidonHasher>(
            note_index, commitment, siblings,
        );

        // Align with CLI private_transfer:
        // 1) wait user nonce change after checkpoint_before
        // 2) wait note_count / note_root slot value change
        // 3) prove against selected checkpoint_after
        let baseline_user_leaf = provider
            .get_user_leaf_data(checkpoint_before, sender_user_id)
            .await
            .map_err(|e| JsError::new(&format!("get_user_leaf_data(baseline): {}", e)))?;
        let baseline_nonce = baseline_user_leaf.nonce.to_canonical_u64();
        let baseline_note_count = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_before,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_count_slot,
            )
            .await
            .map_err(|e| JsError::new(&format!("baseline note_count proof: {}", e)))?
            .value;
        let baseline_note_root = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_before,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_root_slot_val,
            )
            .await
            .map_err(|e| JsError::new(&format!("baseline note_root proof: {}", e)))?
            .value;

        let wait_deadline_ms = now_ms().saturating_add(180_000);
        let mut latest_coordinator_seen = checkpoint_before;
        let mut latest_realm_seen = checkpoint_before;

        let mut checkpoint_after_nonce: Option<u64> = None;
        let mut next_checkpoint_to_check = checkpoint_before.saturating_add(1);
        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);
            if let Ok(realm_state) = provider.get_realm_latest_block_state().await {
                latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
            }

            while next_checkpoint_to_check <= latest_coordinator {
                let leaf = match provider
                    .get_user_leaf_data(next_checkpoint_to_check, sender_user_id)
                    .await
                {
                    Ok(v) => v,
                    Err(_) => break,
                };
                if leaf.nonce.to_canonical_u64() > baseline_nonce {
                    checkpoint_after_nonce = Some(next_checkpoint_to_check);
                    break;
                }
                next_checkpoint_to_check = next_checkpoint_to_check.saturating_add(1);
            }
            if checkpoint_after_nonce.is_some() {
                break;
            }
            async_sleep_ms(1000).await;
        }
        let checkpoint_after_nonce = checkpoint_after_nonce.ok_or_else(|| {
            JsError::new(&format!(
                "[wallet-wasm-v2] timeout waiting nonce change (checkpoint_before={}, baseline_nonce={}, latestCoordinator={}, latestRealm={})",
                checkpoint_before, baseline_nonce, latest_coordinator_seen, latest_realm_seen
            ))
        })?;

        let mut checkpoint_after: Option<u64> = None;
        let mut next_slot_checkpoint = checkpoint_after_nonce.saturating_sub(1).saturating_add(1);
        let mut last_error = String::new();
        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);
            if let Ok(realm_state) = provider.get_realm_latest_block_state().await {
                latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
            }

            while next_slot_checkpoint <= latest_coordinator {
                let note_count = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        next_slot_checkpoint,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_count_slot,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(_) => break,
                };
                let note_root = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        next_slot_checkpoint,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_root_slot_val,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(_) => break,
                };

                if note_count != baseline_note_count || note_root != baseline_note_root {
                    checkpoint_after = Some(next_slot_checkpoint);
                    break;
                }
                last_error = format!(
                    "slots unchanged at checkpoint {}: note_count={} note_root={}",
                    next_slot_checkpoint, note_count, note_root
                );
                next_slot_checkpoint = next_slot_checkpoint.saturating_add(1);
            }
            if checkpoint_after.is_some() {
                break;
            }
            async_sleep_ms(1000).await;
        }
        let checkpoint_after = checkpoint_after.ok_or_else(|| {
            JsError::new(&format!(
                "[wallet-wasm-v2] timeout waiting note slots change (checkpoint_before={}, nonceCheckpoint={}, latestCoordinator={}, latestRealm={}, lastError={})",
                checkpoint_before, checkpoint_after_nonce, latest_coordinator_seen, latest_realm_seen, last_error
            ))
        })?;

        // Fetch post-submit proofs and leaf at the selected checkpoint.
        let user_leaf = user_provider.get_user_leaf_data(checkpoint_after, sender_user_id).await
            .map_err(|e| JsError::new(&format!("get_user_leaf_data: {}", e)))?;
        let note_root_slot_proof = user_provider.get_user_contract_state_tree_merkle_proof(
            checkpoint_after, sender_user_id, contract_id_val as u32,
            MAX_CONTRACT_STATE_TREE_HEIGHT as u8, note_root_slot_val,
        ).await
            .map_err(|e| JsError::new(&format!("note_root_slot_proof: {}", e)))?;
        // Fetch contract_proof and user_tree_proof
        let contract_proof = user_provider.get_user_contract_tree_merkle_proof(
            checkpoint_after, sender_user_id, contract_id_val as u32,
        ).await
            .map_err(|e| JsError::new(&format!("contract_proof: {}", e)))?;
        let user_tree_proof = user_provider.get_user_tree_merkle_proof(
            checkpoint_after, sender_user_id,
        ).await
            .map_err(|e| JsError::new(&format!("user_tree_proof: {}", e)))?;
        let global_user_tree_root = user_tree_proof.root;

        // Build circuit input and prove
        let circuit_input = PrivateNoteInclusionInput {
            nullifier_secret,
            sender_user_id,
            contract_id: contract_id_val,
            note_root_slot: note_root_slot_val,
            user_leaf,
            owner,
            amount: F::from_canonical_u64(amount_val),
            randomness: note_secret_hash,
            note_membership_proof,
            note_root_slot_proof,
            contract_proof,
            user_tree_proof,
            checkpoint_id: F::from_canonical_u64(checkpoint_after),
        };

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof = circuit.prove(&circuit_input)
            .map_err(|e| JsError::new(&format!("prove error: {}", e)))?;
        let fingerprint = circuit.get_fingerprint();

        // Compute nullifier
        let nullifier = PoseidonHasher::q_hash_many(&nullifier_secret.0.elements);

        // Encode proof as bincode + base64
        let proof_bytes = bincode::serialize(&proof)
            .map_err(|e| JsError::new(&format!("bincode serialize: {}", e)))?;
        let proof_b64 = BASE64.encode(&proof_bytes);

        // Build NoteProofOutput as JSON
        let to_str_arr = |h: QHashOut<F>| -> [String; 4] {
            let e = h.0.elements;
            [
                e[0].to_canonical_u64().to_string(),
                e[1].to_canonical_u64().to_string(),
                e[2].to_canonical_u64().to_string(),
                e[3].to_canonical_u64().to_string(),
            ]
        };

        let result = serde_json::json!({
            "nullifier": to_str_arr(nullifier),
            "owner": to_str_arr(owner),
            "amount": amount_val.to_string(),
            "user_tree_root": to_str_arr(global_user_tree_root),
            "checkpoint_id": checkpoint_after.to_string(),
            "note_root_slot": note_root_slot_val.to_string(),
            "note_proof_fingerprint": to_str_arr(fingerprint),
            "note_proof_bincode_b64": proof_b64,
        });
        Ok(result.to_string())
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
            .map_err(|e| JsError::new(&format!("Parse sign data error: {}", e)))?
            .unwrap_or_default();

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
