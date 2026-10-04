pub use psy_common;
pub use psy_config::network_constants;
pub use psy_crypto;
pub use psy_provider::request;
#[cfg(not(target_arch = "wasm32"))]
pub use psy_provider::{lps, provider, session, wallet};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

/// `initThreadPool(n)` for JS: rayon (and so plonky2's parallel prover) runs on n workers.
#[cfg(all(target_arch = "wasm32", feature = "threads"))]
pub use wasm_bindgen_rayon::init_thread_pool;

/// The DepositInclusion circuit's fingerprint, common and verifier data, embedded.
pub mod deposit_inclusion_artifacts;

/// Private-note checkpoint helpers shared by the WASM wallet flow and the
/// native test suite (`is_checkpoint_observable`, `ensure_expected_private_note_root`).
mod private_note_checkpoint;

#[cfg(feature = "lab-bench")]
pub mod lab_bench;
