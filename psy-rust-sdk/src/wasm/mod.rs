//! WebAssembly bindings for the Rust SDK.
//!
//! Keep this module as the public surface only. Implementation details live in
//! focused submodules so changes to configuration, runtime support, and wallet
//! RPC bindings do not accumulate in one file.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => ($crate::wasm::log(&format_args!($($t)*).to_string()))
}

mod config;
mod runtime;
mod server;

pub use config::{WasmConstants, WasmPsyConfig, WasmPsyConfigBuilder};
pub use runtime::{init_logging, main};
pub use server::WasmRpcServer;
