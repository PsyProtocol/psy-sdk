/// KZG Commitment Scheme implementation for membership proofs
/// Reference implementations:
/// - https://github.com/CleanPegasus/kzg-commitment
/// - https://github.com/rust-ethereum/rust-kzg

pub mod commitment;
pub mod proof;
pub mod setup;
pub mod verifier;

#[cfg(test)]
mod tests;

pub use commitment::{KZGCommitment, KZGCommitmentTarget, CircuitBuilderKZG};
pub use proof::{KZGProof, KZGProofTarget};
pub use setup::{KZGParams, KZGSetup};
pub use verifier::KZGVerifier;