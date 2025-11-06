// WASM-compatible exports for psy_prover
// This module defines what functionality should be available in WASM
// The actual wasm_bindgen is handled by psy_rust_sdk

// Re-export session and wallet functionality that works in WASM
pub use crate::{session::*, wallet::*};

// Define WASM-compatible functions without wasm_bindgen attributes
// psy_rust_sdk will add the wasm_bindgen attributes

pub fn init_prover_logging() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());
        wasm_tracing::set_as_global_default();
        tracing::info!("PSY Prover logging initialized");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tracing::info!("PSY Prover logging (native mode)");
    }
}
