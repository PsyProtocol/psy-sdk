#[cfg(not(target_arch = "wasm32"))]
pub mod error;
pub mod memory_wallet;
pub mod simple_sign;
pub mod software_defined_circuit;
