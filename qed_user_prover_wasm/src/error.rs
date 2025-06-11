//! Error handling for QED User Prover WASM module

use wasm_bindgen::prelude::*;
use thiserror::Error;

/// Result type for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;

/// Error types for WASM operations
#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Initialization error: {0}")]
    Initialization(String),
    
    #[error("Session error: {0}")]
    Session(String),
    
    #[error("Proving error: {0}")]
    Proving(String),
    
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Lock error: {0}")]
    Lock(String),
    
    #[error("Not initialized")]
    NotInitialized,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<WasmError> for JsValue {
    fn from(error: WasmError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

impl From<serde_json::Error> for WasmError {
    fn from(error: serde_json::Error) -> Self {
        WasmError::Serialization(error.to_string())
    }
}

impl From<anyhow::Error> for WasmError {
    fn from(error: anyhow::Error) -> Self {
        WasmError::Internal(error.to_string())
    }
}

/// Macro for converting lock errors
macro_rules! lock_error {
    ($expr:expr) => {
        $expr.map_err(|e| WasmError::Lock(format!("Lock poisoned: {}", e)))
    };
}

pub(crate) use lock_error;