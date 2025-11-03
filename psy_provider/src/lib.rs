// Shared modules for both native and wasm
pub mod common;
pub mod request;

// These modules use async/await and are only available for native
#[cfg(not(target_arch = "wasm32"))]
pub mod lps;
#[cfg(not(target_arch = "wasm32"))]
pub mod provider;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;
#[cfg(not(target_arch = "wasm32"))]
pub mod wallet;
