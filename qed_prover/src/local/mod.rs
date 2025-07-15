pub mod api;
pub mod store;
pub use store::UserProverWorkerStore;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // WASM-specific modules
        pub mod types;
        pub mod wallet_session;
        
        use wasm_bindgen::prelude::*;
        pub use api::WasmRpcServer;
    } else {
        // Native-only modules
        pub mod args;
        pub mod common;
    }
}
