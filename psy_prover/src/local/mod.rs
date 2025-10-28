pub mod store;
pub mod common;
pub use store::UserProverWorkerStore;

// Shared modules for both native and wasm
pub mod args;
pub mod request;

// These modules use async/await and are only available for native
// #[cfg(not(target_arch = "wasm32"))]
pub mod provider;
// #[cfg(not(target_arch = "wasm32"))]
pub mod lps;
// #[cfg(feature = "is_sync")]
pub mod simple;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-export everything from native when not in WASM
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

// Re-export everything from wasm when in WASM
#[cfg(target_arch = "wasm32")]
pub use wasm::*;