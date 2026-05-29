// WASM-specific bindings and exports
use plonky2::field::types::Field;
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

fn parse_int_string(value: &str) -> Result<u64, JsError> {
    let normalized = value.strip_prefix("n:").unwrap_or(value);
    normalized
        .parse::<u64>()
        .map_err(|e| JsError::new(&e.to_string()))
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
    session::{
        ClaimBatchItem, PrivateTransferClaim, ShieldDepositClaim, WalletKeyPair, WalletSession,
    },
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

    #[wasm_bindgen]
    pub async fn exec_claim_batch_json(
        &mut self,
        pk_hash: &str,
        claims_json: &str,
    ) -> Result<String, JsError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_crypto::hash::merkle::core::MerkleProofCore;
        use psy_config::network_constants::{
            GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };
        use psy_crypto::shield_address::{
            derive_deposit_commitment, derive_nullifier_hash, derive_shield_address,
        };
        use psy_data::privacy::shield_deposit_claim::ShieldDepositClaimInput;
        use psy_dpn_circuit::circuits::privacy::shield_deposit_claim::ShieldDepositClaimCircuit;
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;

        const NOTE_TREE_HEIGHT: usize = 20;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        #[derive(serde::Deserialize)]
        #[serde(tag = "type", content = "data", rename_all = "snake_case")]
        enum WalletClaimBatchItem {
            Public(ContractCallArgs),
            PrivateTransfer {
                contract_id: String,
                claim: PrivateTransferClaimInput,
            },
            ClaimShieldDeposit(ShieldDepositClaimRaw),
        }

        #[derive(serde::Deserialize)]
        struct PrivateTransferClaimInput {
            note_proof_bincode_b64: String,
            nullifier: [String; 4],
            owner: [String; 4],
            amount: String,
            user_tree_root: [String; 4],
            checkpoint_id: String,
            note_root_slot: String,
            random0: String,
            random1: String,
            #[serde(default)]
            shield_address: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct ShieldDepositClaimRaw {
            nullifier: [String; 4],
            note_secret_hash: [String; 4],
            token_address_u32x8: [String; 8],
            l2_token_contract_id: [String; 8],
            amount_u32x8: [String; 8],
            source_chain_index: String,
            deposit_index: String,
            deposit_root: [String; 4],
            deposit_siblings: Vec<[String; 4]>,
            random0: String,
            random1: String,
            contract_id: String,
        }

        let items: Vec<WalletClaimBatchItem> = serde_json::from_str(claims_json)
            .map_err(|e| JsError::new(&format!("Parse claim batch JSON error: {}", e)))?;

        if items.is_empty() {
            return Err(JsError::new("No claims to execute"));
        }

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );

        let parse_u64_arr = |arr: [String; 4]| -> Result<[u64; 4], JsError> {
            Ok([
                arr[0].parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?,
                arr[1].parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?,
                arr[2].parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?,
                arr[3].parse().map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?,
            ])
        };
        let parse_u32_arr = |arr: [String; 8]| -> Result<[u32; 8], JsError> {
            Ok([
                parse_int_string(&arr[0])? as u32,
                parse_int_string(&arr[1])? as u32,
                parse_int_string(&arr[2])? as u32,
                parse_int_string(&arr[3])? as u32,
                parse_int_string(&arr[4])? as u32,
                parse_int_string(&arr[5])? as u32,
                parse_int_string(&arr[6])? as u32,
                parse_int_string(&arr[7])? as u32,
            ])
        };
        let qhash_from_u64_arr = |arr: [u64; 4]| -> QHashOut<F> {
            QHashOut::from_values(arr[0], arr[1], arr[2], arr[3])
        };

        let mut claims: Vec<ClaimBatchItem> = Vec::new();

        for item in items {
            match item {
                WalletClaimBatchItem::Public(call) => {
                    claims.push(ClaimBatchItem::Public(call));
                }
                WalletClaimBatchItem::PrivateTransfer { contract_id, claim: input } => {
                    let proof_bytes = BASE64.decode(input.note_proof_bincode_b64.as_bytes())
                        .map_err(|e| JsError::new(&format!("base64 decode: {}", e)))?;
                    let proof: ProofWithPublicInputs<F, C, D> =
                        match ProofWithPublicInputs::<F, C, D>::from_bytes(
                            proof_bytes.clone(),
                            circuit.get_common_circuit_data_ref(),
                        ) {
                            Ok(p) => p,
                            Err(native_err) => bincode::deserialize(&proof_bytes).map_err(|bin_err| {
                                JsError::new(&format!(
                                    "proof deserialize: native={} ; bincode={}",
                                    native_err, bin_err
                                ))
                            })?,
                        };
                    let fingerprint = circuit.get_fingerprint();
                    let verifier_data = circuit.get_verifier_config_ref().clone();

                    let nullifier = parse_u64_arr(input.nullifier)?;
                    let owner = parse_u64_arr(input.owner)?;
                    let amount: u64 = input.amount.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let user_tree_root = parse_u64_arr(input.user_tree_root)?;
                    let checkpoint_id: u64 = input.checkpoint_id.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let note_root_slot: u64 = input.note_root_slot.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let random0: u64 = input.random0.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let random1: u64 = input.random1.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let contract_id: u64 = contract_id.parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;

                    let claim = PrivateTransferClaim {
                        nullifier,
                        owner,
                        amount,
                        user_tree_root,
                        checkpoint_id,
                        note_root_slot,
                        random0,
                        random1,
                        note_proof_fingerprint: fingerprint,
                        note_proof: proof,
                        note_verifier_data: verifier_data.into(),
                    };

                    claims.push(ClaimBatchItem::PrivateTransfer { contract_id, claim });
                }
                WalletClaimBatchItem::ClaimShieldDeposit(input) => {
                    let nullifier_secret = parse_u64_arr(input.nullifier)?;
                    let note_secret_hash = parse_u64_arr(input.note_secret_hash)?;
                    let token_address = parse_u32_arr(input.token_address_u32x8)?;
                    let l2_token_contract_id = parse_u32_arr(input.l2_token_contract_id)?;
                    let amount = parse_u32_arr(input.amount_u32x8)?;
                    let source_chain_index: u32 = input.source_chain_index
                        .parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let deposit_index: u64 = input.deposit_index
                        .parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let deposit_root = qhash_from_u64_arr(parse_u64_arr(input.deposit_root)?);
                    let deposit_siblings: Vec<QHashOut<F>> = input.deposit_siblings
                        .into_iter()
                        .map(|sibling| parse_u64_arr(sibling).map(|arr| qhash_from_u64_arr(arr)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let random0: u64 = input.random0
                        .parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let random1: u64 = input.random1
                        .parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
                    let contract_id: u64 = input.contract_id
                        .parse()
                        .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;

                    let provider = self.wallet_session.st_provider.clone();
                    let user_ids = provider
                        .get_user_ids_for_public_key(pk_hash)
                        .await
                        .map_err(|e| JsError::new(&format!("get_user_ids_for_public_key: {}", e)))?;
                    let user_id = *user_ids
                        .first()
                        .ok_or_else(|| JsError::new("No user ID found for public key"))?;

                    let shield_address = derive_shield_address(user_id, random0, random1);
                    let nullifier_hash = derive_nullifier_hash(nullifier_secret);
                    let deposit_leaf = derive_deposit_commitment(
                        shield_address,
                        token_address,
                        l2_token_contract_id,
                        amount,
                        source_chain_index,
                        note_secret_hash,
                    );

                    let circuit = ShieldDepositClaimCircuit::<C, D>::new();
                    let claim_input = ShieldDepositClaimInput::<F> {
                        nullifier_secret: std::array::from_fn(|i| <F as plonky2::field::types::Field>::from_canonical_u64(nullifier_secret[i])),
                        note_secret_hash: std::array::from_fn(|i| <F as plonky2::field::types::Field>::from_canonical_u64(note_secret_hash[i])),
                        r0: <F as plonky2::field::types::Field>::from_canonical_u64(random0),
                        r1: <F as plonky2::field::types::Field>::from_canonical_u64(random1),
                        user_id,
                        deposit_index,
                        token_address,
                        l2_token_contract_id,
                        amount,
                        source_chain_index,
                        deposit_root,
                        deposit_proof: MerkleProofCore {
                            root: deposit_root,
                            value: deposit_leaf,
                            index: deposit_index,
                            siblings: deposit_siblings,
                        },
                    };
                    let proof = circuit
                        .prove(&claim_input)
                        .map_err(|e| JsError::new(&format!("shield claim prove: {}", e)))?;
                    let proof_fingerprint = circuit.get_fingerprint();
                    let verifier_data = circuit.get_verifier_config_ref().clone();

                    claims.push(ClaimBatchItem::ShieldDeposit(ShieldDepositClaim {
                        contract_id,
                        l2_token_contract_id,
                        nullifier_hash,
                        shield_address,
                        token_address,
                        amount,
                        source_chain_index,
                        deposit_root,
                        r0: random0,
                        r1: random1,
                        proof_fingerprint,
                        proof,
                        verifier_data: verifier_data.into(),
                    }));
                }
            }
        }

        let tx_hash = self
            .wallet_session
            .claim_batch(pk_hash, claims)
            .await
            .map_err(|e| JsError::new(&format!("claim_batch error: {}", e)))?;
        Ok(tx_hash.to_string())
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

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof_bytes = BASE64
            .decode(note_proof_bincode_b64.as_bytes())
            .map_err(|e| JsError::new(&format!("base64 decode error: {}", e)))?;
        // Compatibility: accept both modern plonky2 native proof bytes and legacy
        // bincode-serialized proofs to avoid cross-version serialization breakage.
        let proof: ProofWithPublicInputs<F, C, D> =
            match ProofWithPublicInputs::<F, C, D>::from_bytes(
                proof_bytes.clone(),
                circuit.get_common_circuit_data_ref(),
            ) {
                Ok(p) => p,
                Err(native_err) => bincode::deserialize(&proof_bytes).map_err(|bin_err| {
                    JsError::new(&format!(
                        "proof deserialize error: native={} ; bincode={}",
                        native_err, bin_err
                    ))
                })?,
            };
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

    /// Atomic private_claim: start_session → add_external_proof → prove → sign_and_submit.
    ///
    /// This replaces the broken two-step flow (psy_addExternalProof then sendTransaction)
    /// where sendTransaction's internal start_session call would reset the session tree,
    /// losing the injected external proof.
    ///
    /// Inputs (all u64 values as decimal strings to avoid JS precision loss):
    ///   pk_hash                 - receiver's ZK public key (hex QHashOut)
    ///   note_proof_bincode_b64  - base64-encoded PrivateNoteInclusion proof bytes
    ///   nullifier_json          - JSON array of 4 decimal strings
    ///   owner_json              - JSON array of 4 decimal strings
    ///   amount                  - decimal string
    ///   user_tree_root_json     - JSON array of 4 decimal strings
    ///   checkpoint_id           - decimal string
    ///   note_root_slot          - decimal string
    ///   contract_id             - decimal string
    ///   random0                 - decimal string
    ///   random1                 - decimal string
    ///
    /// Returns the transaction hash string.
    #[wasm_bindgen]
    pub async fn exec_claim_with_external_proof_json(
        &mut self,
        pk_hash: &str,
        note_proof_bincode_b64: &str,
        nullifier_json: &str,
        owner_json: &str,
        amount: &str,
        user_tree_root_json: &str,
        checkpoint_id: &str,
        note_root_slot: &str,
        contract_id: &str,
        random0: &str,
        random1: &str,
    ) -> Result<String, JsError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use plonky2::field::types::PrimeField64;
        use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;
        use psy_config::network_constants::{
            GLOBAL_USER_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };

        const NOTE_TREE_HEIGHT: usize = 20;
        type C = PoseidonGoldilocksConfig;
        const D: usize = 2;

        // Helper: parse JSON array of 4 decimal strings → [u64; 4]
        let parse_u64x4 = |json: &str| -> Result<[u64; 4], JsError> {
            let arr: [String; 4] = serde_json::from_str(json)
                .map_err(|e| JsError::new(&format!("parse u64x4: {}", e)))?;
            Ok([
                parse_int_string(&arr[0])?,
                parse_int_string(&arr[1])?,
                parse_int_string(&arr[2])?,
                parse_int_string(&arr[3])?,
            ])
        };

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("parse pk_hash: {}", e)))?;
        let nullifier = parse_u64x4(nullifier_json)?;
        let owner = parse_u64x4(owner_json)?;
        let amount_val: u64 = amount.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let user_tree_root = parse_u64x4(user_tree_root_json)?;
        let checkpoint_id_val: u64 = checkpoint_id.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let note_root_slot_val: u64 = note_root_slot.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let contract_id_val: u64 = contract_id.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let random0_val: u64 = random0.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let random1_val: u64 = random1.parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;

        // Decode and deserialize the proof
        let proof_bytes = BASE64.decode(note_proof_bincode_b64.as_bytes())
            .map_err(|e| JsError::new(&format!("base64 decode: {}", e)))?;
        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof: ProofWithPublicInputs<F, C, D> =
            match ProofWithPublicInputs::<F, C, D>::from_bytes(
                proof_bytes.clone(),
                circuit.get_common_circuit_data_ref(),
            ) {
                Ok(p) => p,
                Err(native_err) => bincode::deserialize(&proof_bytes).map_err(|bin_err| {
                    JsError::new(&format!(
                        "proof deserialize: native={} ; bincode={}",
                        native_err, bin_err
                    ))
                })?,
            };
        let fingerprint = circuit.get_fingerprint();
        let verifier_data = circuit.get_verifier_config_ref().clone();

        // Step 1: reset session (must happen before add_external_proof)
        self.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("start_session: {}", e)))?;

        // Step 2: inject external proof AFTER reset → get leaf_index + siblings
        let (leaf_index, siblings) = self
            .wallet_session
            .add_external_proof_with_siblings(pk_hash, fingerprint, proof, verifier_data)
            .await
            .map_err(|e| JsError::new(&format!("add_external_proof: {}", e)))?;

        // Step 3: build private_claim inputs
        // [nullifier(4), owner(4), amount, user_tree_root(4), checkpoint_id,
        //  note_root_slot, random0, random1, ...siblings.flat(), leaf_index]
        let mut inputs: Vec<u64> = Vec::new();
        inputs.extend_from_slice(&nullifier);
        inputs.extend_from_slice(&owner);
        inputs.push(amount_val);
        inputs.extend_from_slice(&user_tree_root);
        inputs.push(checkpoint_id_val);
        inputs.push(note_root_slot_val);
        inputs.push(random0_val);
        inputs.push(random1_val);
        for s in &siblings {
            inputs.extend_from_slice(s);
        }
        inputs.push(leaf_index);

        let contract_call = ContractCallArgs {
            contract_id: contract_id_val,
            method_name: "private_claim".to_string(),
            inputs,
        };

        // Step 4: prove (session already has the external proof — no reset)
        self.wallet_session
            .prove_contract_call(pk_hash, vec![contract_call])
            .await
            .map_err(|e| JsError::new(&format!("prove_contract_call: {}", e)))?;

        // Baseline receiver nonce before submit; wait for it to change after submit.
        let provider = self.wallet_session.st_provider.clone();
        let user_ids = provider
            .get_user_ids_for_public_key(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("get_user_ids_for_public_key: {}", e)))?;
        let receiver_user_id = *user_ids
            .first()
            .ok_or_else(|| JsError::new("No user ID found for public key"))?;

        let baseline_snapshot = provider
            .get_latest_block_state()
            .await
            .map_err(|e| JsError::new(&format!("get_latest_block_state: {}", e)))?;
        let checkpoint_before = baseline_snapshot.checkpoint_id;
        let baseline_nonce = provider
            .get_user_leaf_data(checkpoint_before, receiver_user_id)
            .await
            .map_err(|e| JsError::new(&format!("get_user_leaf_data(baseline): {}", e)))?
            .nonce
            .to_canonical_u64();

        // Step 5: sign and submit
        let tx_hash = self
            .wallet_session
            .sign_and_submit(pk_hash, DPNSoftwareDefinedCallData::default())
            .await
            .map_err(|e| JsError::new(&format!("sign_and_submit: {}", e)))?;

        // Step 6: wait nonce change on observable checkpoint stream.
        let wait_deadline_ms = now_ms().saturating_add(180_000);
        let mut latest_coordinator_seen = checkpoint_before;
        let mut latest_realm_seen = checkpoint_before;
        let mut latest_observable_seen = checkpoint_before;
        let mut last_error = String::new();
        let mut next_checkpoint = checkpoint_before.saturating_add(1);
        let mut nonce_changed = false;

        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);

            let latest_realm = match provider.get_realm_latest_block_state().await {
                Ok(realm_state) => {
                    latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
                    realm_state.checkpoint_id
                }
                Err(_) => latest_coordinator,
            };

            let latest_observable = latest_realm.min(latest_coordinator);
            latest_observable_seen = latest_observable_seen.max(latest_observable);

            while next_checkpoint <= latest_observable {
                let nonce = match provider
                    .get_user_leaf_data(next_checkpoint, receiver_user_id)
                    .await
                {
                    Ok(leaf) => leaf.nonce.to_canonical_u64(),
                    Err(e) => {
                        last_error = format!(
                            "get_user_leaf_data failed at checkpoint {}: {}",
                            next_checkpoint, e
                        );
                        break;
                    }
                };

                if nonce != baseline_nonce {
                    nonce_changed = true;
                    break;
                }
                last_error = format!(
                    "nonce unchanged at checkpoint {}: nonce={} baseline={}",
                    next_checkpoint, nonce, baseline_nonce
                );
                next_checkpoint = next_checkpoint.saturating_add(1);
            }

            if nonce_changed {
                break;
            }
            async_sleep_ms(1000).await;
        }

        if !nonce_changed {
            return Err(JsError::new(&format!(
                "[wallet-wasm-v2] timeout waiting private_claim nonce change (checkpoint_before={}, baseline_nonce={}, latestCoordinator={}, latestRealm={}, latestObservable={}, lastError={})",
                checkpoint_before,
                baseline_nonce,
                latest_coordinator_seen,
                latest_realm_seen,
                latest_observable_seen,
                last_error
            )));
        }

        Ok(tx_hash.to_string())
    }

    /// Atomic shield claim_deposit:
    /// build ShieldDepositClaim proof -> start_session -> add_external_proof -> prove -> sign_and_submit.
    ///
    /// Inputs:
    ///   pk_hash                    - receiver's ZK public key (hex QHashOut)
    ///   nullifier_json             - JSON array of 4 decimal strings
    ///   note_secret_hash_json      - JSON array of 4 decimal strings
    ///   token_address_u32x8_json   - JSON array of 8 decimal strings (bytes32 BE words)
    ///   l2_token_contract_id_json  - JSON array of 8 decimal strings (bytes32 BE words)
    ///   amount_u32x8_json          - JSON array of 8 decimal strings (bytes32 BE words)
    ///   source_chain_index         - decimal string
    ///   deposit_index              - decimal string
    ///   deposit_root_json          - JSON array of 4 decimal strings (QHashOut limbs)
    ///   deposit_siblings_json      - JSON array of arrays of 4 decimal strings
    ///   random0                    - decimal string
    ///   random1                    - decimal string
    ///   contract_id                - decimal string
    ///
    /// Returns the transaction hash string.
    #[wasm_bindgen]
    pub async fn exec_shield_claim_deposit_json(
        &mut self,
        pk_hash: &str,
        nullifier_json: &str,
        note_secret_hash_json: &str,
        token_address_u32x8_json: &str,
        l2_token_contract_id_json: &str,
        amount_u32x8_json: &str,
        source_chain_index: &str,
        deposit_index: &str,
        deposit_root_json: &str,
        deposit_siblings_json: &str,
        random0: &str,
        random1: &str,
        contract_id: &str,
    ) -> Result<String, JsError> {
        use plonky2::field::types::PrimeField64;
        use psy_data::privacy::shield_deposit_claim::ShieldDepositClaimInput;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_crypto::hash::merkle::core::MerkleProofCore;
        use psy_crypto::shield_address::{derive_deposit_commitment, derive_nullifier_hash, derive_shield_address};
        use psy_dpn_circuit::circuits::privacy::shield_deposit_claim::ShieldDepositClaimCircuit;

        let parse_u64x4 = |json: &str| -> Result<[u64; 4], JsError> {
            let arr: [String; 4] = serde_json::from_str(json)
                .map_err(|e| JsError::new(&format!("parse u64x4: {}", e)))?;
            Ok([
                parse_int_string(&arr[0])?,
                parse_int_string(&arr[1])?,
                parse_int_string(&arr[2])?,
                parse_int_string(&arr[3])?,
            ])
        };

        let parse_u32x8 = |json: &str| -> Result<[u32; 8], JsError> {
            let arr: [String; 8] = serde_json::from_str(json)
                .map_err(|e| JsError::new(&format!("parse u32x8: {}", e)))?;
            Ok([
                parse_int_string(&arr[0])? as u32,
                parse_int_string(&arr[1])? as u32,
                parse_int_string(&arr[2])? as u32,
                parse_int_string(&arr[3])? as u32,
                parse_int_string(&arr[4])? as u32,
                parse_int_string(&arr[5])? as u32,
                parse_int_string(&arr[6])? as u32,
                parse_int_string(&arr[7])? as u32,
            ])
        };

        let parse_qhash = |json: &str| -> Result<QHashOut<F>, JsError> {
            let limbs = parse_u64x4(json)?;
            Ok(QHashOut::from_values(limbs[0], limbs[1], limbs[2], limbs[3]))
        };

        let parse_qhash_vec = |json: &str| -> Result<Vec<QHashOut<F>>, JsError> {
            let arr: Vec<[String; 4]> = serde_json::from_str(json)
                .map_err(|e| JsError::new(&format!("parse qhash vec: {}", e)))?;
            arr.into_iter()
                .map(|item| {
                    Ok(QHashOut::from_values(
                        parse_int_string(&item[0])?,
                        parse_int_string(&item[1])?,
                        parse_int_string(&item[2])?,
                        parse_int_string(&item[3])?,
                    ))
                })
                .collect()
        };

        let qhash_to_internal_u32x8 = |hash: QHashOut<F>| -> [u32; 8] {
            [
                (hash.0.elements[0].to_canonical_u64() & 0xffff_ffff) as u32,
                (hash.0.elements[0].to_canonical_u64() >> 32) as u32,
                (hash.0.elements[1].to_canonical_u64() & 0xffff_ffff) as u32,
                (hash.0.elements[1].to_canonical_u64() >> 32) as u32,
                (hash.0.elements[2].to_canonical_u64() & 0xffff_ffff) as u32,
                (hash.0.elements[2].to_canonical_u64() >> 32) as u32,
                (hash.0.elements[3].to_canonical_u64() & 0xffff_ffff) as u32,
                (hash.0.elements[3].to_canonical_u64() >> 32) as u32,
            ]
        };

        let qhash_to_u64x4 = |hash: QHashOut<F>| -> [u64; 4] {
            [
                hash.0.elements[0].to_canonical_u64(),
                hash.0.elements[1].to_canonical_u64(),
                hash.0.elements[2].to_canonical_u64(),
                hash.0.elements[3].to_canonical_u64(),
            ]
        };

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("parse pk_hash: {}", e)))?;
        let nullifier_secret = parse_u64x4(nullifier_json)?;
        let note_secret_hash = parse_u64x4(note_secret_hash_json)?;
        let token_address = parse_u32x8(token_address_u32x8_json)?;
        let l2_token_contract_id = parse_u32x8(l2_token_contract_id_json)?;
        let amount = parse_u32x8(amount_u32x8_json)?;
        let source_chain_index_val: u32 = source_chain_index
            .parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let deposit_index_val: u64 = deposit_index
            .parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let deposit_root = parse_qhash(deposit_root_json)?;
        let deposit_siblings = parse_qhash_vec(deposit_siblings_json)?;
        let random0_val: u64 = random0
            .parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let random1_val: u64 = random1
            .parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;
        let contract_id_val: u64 = contract_id
            .parse()
            .map_err(|e: std::num::ParseIntError| JsError::new(&e.to_string()))?;

        let provider = self.wallet_session.st_provider.clone();
        let user_ids = provider
            .get_user_ids_for_public_key(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("get_user_ids_for_public_key: {}", e)))?;
        let user_id = *user_ids
            .first()
            .ok_or_else(|| JsError::new("No user ID found for public key"))?;

        let shield_address = derive_shield_address(user_id, random0_val, random1_val);
        let nullifier_hash = derive_nullifier_hash(nullifier_secret);
        let deposit_leaf = derive_deposit_commitment(
            shield_address,
            token_address,
            l2_token_contract_id,
            amount,
            source_chain_index_val,
            note_secret_hash,
        );

        let circuit = ShieldDepositClaimCircuit::<C, D>::new();
        let input = ShieldDepositClaimInput::<F> {
            nullifier_secret: std::array::from_fn(|i| <F as plonky2::field::types::Field>::from_canonical_u64(nullifier_secret[i])),
            note_secret_hash: std::array::from_fn(|i| <F as plonky2::field::types::Field>::from_canonical_u64(note_secret_hash[i])),
            r0: <F as plonky2::field::types::Field>::from_canonical_u64(random0_val),
            r1: <F as plonky2::field::types::Field>::from_canonical_u64(random1_val),
            user_id,
            deposit_index: deposit_index_val,
            token_address,
            l2_token_contract_id,
            amount,
            source_chain_index: source_chain_index_val,
            deposit_root,
            deposit_proof: MerkleProofCore {
                root: deposit_root,
                value: deposit_leaf,
                index: deposit_index_val,
                siblings: deposit_siblings,
            },
        };

        let proof = circuit
            .prove(&input)
            .map_err(|e| JsError::new(&format!("shield claim prove: {}", e)))?;
        let fingerprint = circuit.get_fingerprint();
        let verifier_data = circuit.get_verifier_config_ref().clone();

        self.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("start_session: {}", e)))?;

        let (proof_index, proof_siblings) = self
            .wallet_session
            .add_external_proof_with_siblings(pk_hash, fingerprint, proof, verifier_data)
            .await
            .map_err(|e| JsError::new(&format!("add_external_proof: {}", e)))?;

        let mut contract_inputs = Vec::with_capacity(100);
        contract_inputs.extend_from_slice(&qhash_to_u64x4(nullifier_hash));
        contract_inputs.extend_from_slice(&qhash_to_u64x4(shield_address));
        contract_inputs.extend(token_address.iter().map(|&v| v as u64));
        contract_inputs.extend(amount.iter().map(|&v| v as u64));
        contract_inputs.push(source_chain_index_val as u64);
        contract_inputs.extend(qhash_to_internal_u32x8(deposit_root).iter().map(|&v| v as u64));
        contract_inputs.push(random0_val);
        contract_inputs.push(random1_val);
        for sibling in &proof_siblings {
            contract_inputs.extend_from_slice(sibling);
        }
        contract_inputs.push(proof_index);

        let checkpoint_before = provider
            .get_coordinator_latest_block_state()
            .await
            .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
            .checkpoint_id;

        self.wallet_session
            .prove_contract_call(
                pk_hash,
                vec![ContractCallArgs {
                    contract_id: contract_id_val,
                    method_name: "claim_deposit".to_string(),
                    inputs: contract_inputs,
                }],
            )
            .await
            .map_err(|e| JsError::new(&format!("prove_contract_call: {}", e)))?;

        let tx_hash = self
            .wallet_session
            .sign_and_submit(pk_hash, DPNSoftwareDefinedCallData::default())
            .await
            .map_err(|e| JsError::new(&format!("sign_and_submit: {}", e)))?;

        provider
            .wait_for_endcap_inclusion(user_id, tx_hash, checkpoint_before, Some(180), 1)
            .await
            .map_err(|e| JsError::new(&format!("wait_for_endcap_inclusion: {}", e)))?;

        Ok(tx_hash.to_string())
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
                parse_int_string(&arr[0])?,
                parse_int_string(&arr[1])?,
                parse_int_string(&arr[2])?,
                parse_int_string(&arr[3])?,
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

        let latest_coordinator_before = provider
            .get_coordinator_latest_block_state()
            .await
            .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
            .checkpoint_id;
        let latest_realm_before = provider
            .get_realm_latest_block_state()
            .await
            .map_err(|e| JsError::new(&format!("get_realm_latest_block_state: {}", e)))?
            .checkpoint_id;
        let latest_observable_before = latest_coordinator_before.min(latest_realm_before);
        let checkpoint_before = if checkpoint_before_raw == 0 {
            latest_observable_before
        } else if checkpoint_before_raw > latest_observable_before {
            return Err(JsError::new(&format!(
                "checkpoint_id {} is ahead of latest observable checkpoint {} (coordinator={}, realm={})",
                checkpoint_before_raw,
                latest_observable_before,
                latest_coordinator_before,
                latest_realm_before
            )));
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

        // Wait state update after checkpoint_before and prove against the first
        // checkpoint where note_count or note_root slot changes.
        //
        // NOTE:
        // We intentionally do not require nonce bump here. Some transaction
        // paths may not mutate nonce in a way that's observable at this stage,
        // while note slots are the actual source of truth for note inclusion.
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
        let mut latest_observable_seen = checkpoint_before;

        let mut checkpoint_after: Option<u64> = None;
        let mut last_error = String::new();
        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| JsError::new(&format!("get_coordinator_latest_block_state: {}", e)))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);

            let latest_realm = match provider.get_realm_latest_block_state().await {
                Ok(realm_state) => {
                    latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
                    Some(realm_state.checkpoint_id)
                }
                Err(_) => None,
            };

            let latest_observable = latest_realm
                .map(|realm_checkpoint| realm_checkpoint.min(latest_coordinator))
                .unwrap_or(latest_coordinator);
            latest_observable_seen = latest_observable_seen.max(latest_observable);

            if latest_observable > checkpoint_before {
                let note_count = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        latest_observable,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_count_slot,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(e) => {
                        last_error = format!(
                            "note_count proof rpc failed at checkpoint {}: {}",
                            latest_observable,
                            e
                        );
                        async_sleep_ms(1000).await;
                        continue;
                    }
                };
                let note_root = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        latest_observable,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_root_slot_val,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(e) => {
                        last_error = format!(
                            "note_root proof rpc failed at checkpoint {}: {}",
                            latest_observable,
                            e
                        );
                        async_sleep_ms(1000).await;
                        continue;
                    }
                };

                if note_count != baseline_note_count || note_root != baseline_note_root {
                    checkpoint_after = Some(latest_observable);
                    break;
                }
                last_error = format!(
                    "slots unchanged at checkpoint {}: note_count={} note_root={} baseline_nonce={}",
                    latest_observable, note_count, note_root, baseline_nonce
                );
            }
            if checkpoint_after.is_some() {
                break;
            }
            async_sleep_ms(1000).await;
        }
        let checkpoint_after = checkpoint_after.ok_or_else(|| {
            JsError::new(&format!(
                "[wallet-wasm-v2] timeout waiting note slots change (checkpoint_before={}, baseline_nonce={}, latestCoordinator={}, latestRealm={}, latestObservable={}, lastError={})",
                checkpoint_before,
                baseline_nonce,
                latest_coordinator_seen,
                latest_realm_seen,
                latest_observable_seen,
                last_error
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
            amount: <F as plonky2::field::types::Field>::from_canonical_u64(amount_val),
            randomness: note_secret_hash,
            note_membership_proof,
            note_root_slot_proof,
            contract_proof,
            user_tree_proof,
            checkpoint_id: <F as plonky2::field::types::Field>::from_canonical_u64(checkpoint_after),
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

        // Encode proof as plonky2 native bytes + base64 for stable cross-version decoding.
        let proof_bytes = proof.to_bytes();
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
        sdk_key_fingerprint: Option<String>,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let fingerprint = match sign_type {
            "zk" => psy_prover::wallet::memory_wallet::get_zk_fingerprint(),
            "secp256k1" => psy_prover::wallet::memory_wallet::get_secp256k1_fingerprint(),
            "sdk-key" => {
                let fingerprint = sdk_key_fingerprint.ok_or_else(|| {
                    JsError::new("SDK key fingerprint is required for sdk-key sign type")
                })?;
                QHashOut::<F>::from_str(&fingerprint).map_err(|e| {
                    JsError::new(&format!("Parse SDK key fingerprint error: {}", e))
                })?
            }
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
        sdk_key_fingerprint: Option<String>,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let fingerprint = match sign_type {
            "zk" => psy_prover::wallet::memory_wallet::get_zk_fingerprint(),
            "secp256k1" => psy_prover::wallet::memory_wallet::get_secp256k1_fingerprint(),
            "sdk-key" => {
                let fingerprint = sdk_key_fingerprint.ok_or_else(|| {
                    JsError::new("SDK key fingerprint is required for sdk-key sign type")
                })?;
                QHashOut::<F>::from_str(&fingerprint).map_err(|e| {
                    JsError::new(&format!("Parse SDK key fingerprint error: {}", e))
                })?
            }
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
    pub async fn register_sdk_key_circuit(
        &mut self,
        allowed_contract_ids: &[u64],
        allowed_method_ids: &[u64],
        expected_tx_count: u64,
    ) -> Result<String, JsError> {
        let fingerprint = self
            .wallet_session
            .register_sdk_key_circuit(allowed_contract_ids, allowed_method_ids, expected_tx_count)
            .await
            .map_err(|e| JsError::new(&format!("Register sd key circuit error: {}", e)))?;
        Ok(fingerprint.to_string())
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
