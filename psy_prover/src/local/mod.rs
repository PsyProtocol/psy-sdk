pub mod common;
pub mod store;
pub use store::UserProverWorkerStore;

// Shared modules for both native and wasm
pub mod args;

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
