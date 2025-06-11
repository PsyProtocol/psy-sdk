//! Utility functions for QED User Prover WASM module

use crate::error::{WasmError, WasmResult};
use crate::types::*;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::console;

/// Serialize a value to JSON string
pub fn serialize_to_json<T: Serialize>(value: &T) -> WasmResult<String> {
    serde_json::to_string(value).map_err(WasmError::from)
}

/// Deserialize a JSON string to a value
pub fn deserialize_from_json<T: for<'de> Deserialize<'de>>(json: &str) -> WasmResult<T> {
    serde_json::from_str(json).map_err(WasmError::from)
}

/// Convert a Rust future to a JavaScript Promise
pub fn future_to_js_promise<F, T>(future: F) -> Promise
where
    F: std::future::Future<Output = WasmResult<T>> + 'static,
    T: Into<JsValue>,
{
    future_to_promise(async move {
        match future.await {
            Ok(value) => Ok(value.into()),
            Err(error) => Err(error.into()),
        }
    })
}

/// Log a message to the browser console
pub fn log(message: &str) {
    console::log_1(&JsValue::from_str(message));
}

/// Log an error to the browser console
pub fn log_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}

/// Log a warning to the browser console
pub fn log_warn(message: &str) {
    console::warn_1(&JsValue::from_str(message));
}

/// Generate a random session ID
pub fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen();
    format!("session_{:016x}", id)
}

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Set panic hook for better error reporting in WASM
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Initialize logger for WASM
pub fn init_logger() {
    #[cfg(feature = "wasm-logger")]
    wasm_logger::init(wasm_logger::Config::default());
}

/// Validate hex string
pub fn is_valid_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Convert hex string to bytes
pub fn hex_to_bytes(hex: &str) -> WasmResult<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Err(WasmError::InvalidInput("Hex string must have even length".to_string()));
    }
    
    hex::decode(hex)
        .map_err(|e| WasmError::InvalidInput(format!("Invalid hex string: {}", e)))
}

/// Convert bytes to hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Validate contract address format
pub fn is_valid_contract_address(address: &str) -> bool {
    address.starts_with("0x") && address.len() == 42 && is_valid_hex(&address[2..])
}

/// Validate function name
pub fn is_valid_function_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Generate random bytes
pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);
    bytes
}


/// Generate a random user ID
pub fn generate_user_id() -> String {
    use js_sys::Math;
    let timestamp = js_sys::Date::now() as u64;
    let random = (Math::random() * 1000000.0) as u64;
    format!("user_{}_{}", timestamp, random)
}

/// Convert plonky2 proof to WASM-compatible format
pub fn convert_plonky2_proof_to_wasm(
    proof: &plonky2::plonk::proof::ProofWithPublicInputs<
        plonky2::field::goldilocks_field::GoldilocksField,
        plonky2::plonk::config::PoseidonGoldilocksConfig,
        2,
    >,
) -> WasmResult<ProofWithPublicInputs> {
    // This is a simplified conversion - in practice, you'd need to properly
    // serialize the plonky2 proof structure
    let wasm_proof = ProofWithPublicInputs {
        proof: Proof {
            wires_cap: vec![],
            plonk_zs_partial_products_cap: vec![],
            quotient_polys_cap: vec![],
            openings: vec![], // Changed to Vec<String>
            opening_proof: vec![], // Changed to Vec<String>
        },
        public_inputs: proof
            .public_inputs
            .iter()
            .map(|input| input.to_string())
            .collect(),
    };
    
    Ok(wasm_proof)
}

/// Validate JSON input
pub fn validate_json_input(input: &str) -> WasmResult<()> {
    if input.trim().is_empty() {
        return Err(WasmError::InvalidInput("Empty input".to_string()));
    }
    
    // Try to parse as JSON to validate format
    serde_json::from_str::<serde_json::Value>(input)
        .map_err(|e| WasmError::InvalidInput(format!("Invalid JSON: {}", e)))?;
    
    Ok(())
}

/// Create an async result placeholder
pub fn create_async_result(id: String) -> AsyncResult {
    AsyncResult {
        id,
        status: "pending".to_string(),
        result: None,
        error: None,
    }
}

/// Update async result with success
pub fn complete_async_result(mut result: AsyncResult, data: String) -> AsyncResult {
    result.status = "completed".to_string();
    result.result = Some(data);
    result
}

/// Update async result with error
pub fn fail_async_result(mut result: AsyncResult, error: String) -> AsyncResult {
    result.status = "failed".to_string();
    result.error = Some(error);
    result
}

// Removed duplicate function definitions - already defined above