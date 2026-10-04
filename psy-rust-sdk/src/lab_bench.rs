//! Lab benchmarks (feature `lab-bench`, wasm-prover-lab-20261005): the wallet's local proving
//! work, one piece at a time, with the same calls the wallet makes. Built for wasm (called from
//! the harness worker) and natively (called from `examples/lab_native.rs`).

use std::cell::RefCell;

use plonky2::plonk::config::PoseidonGoldilocksConfig;
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use psy_dpn_circuit::circuits::privacy::deposit_inclusion::DepositInclusionCircuit;
use psy_prover::wallet::memory_wallet::PsyWalletLocalCircuits;

type C = PoseidonGoldilocksConfig;
type F = plonky2::field::goldilocks_field::GoldilocksField;
const D: usize = 2;

thread_local! {
    /// The loaded local circuits, kept like the wallet keeps them (for the life of the worker).
    static CIRCUITS: RefCell<Option<PsyWalletLocalCircuits>> = const { RefCell::new(None) };
}

/// Milliseconds from a monotonic clock (performance.now() in a browser worker).
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let global = js_sys::global();
        js_sys::Reflect::get(&global, &"performance".into())
            .ok()
            .and_then(|p| {
                let now = js_sys::Reflect::get(&p, &"now".into()).ok()?.dyn_into::<js_sys::Function>().ok()?;
                now.call0(&p).ok()?.as_f64()
            })
            .unwrap_or_else(js_sys::Date::now)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static T0: OnceLock<Instant> = OnceLock::new();
        T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1e3
    }
}

/// Loads the embedded local circuits bundle (what `WalletSession::new` does first). Returns ms.
pub fn load_local_circuits() -> anyhow::Result<f64> {
    let t = now_ms();
    let circuits = PsyWalletLocalCircuits::from_embedded_bundle()?;
    let ms = now_ms() - t;
    CIRCUITS.with(|c| *c.borrow_mut() = Some(circuits));
    Ok(ms)
}

fn with_circuits<R>(f: impl FnOnce(&PsyWalletLocalCircuits) -> anyhow::Result<R>) -> anyhow::Result<R> {
    if CIRCUITS.with(|c| c.borrow().is_none()) {
        load_local_circuits()?;
    }
    CIRCUITS.with(|c| f(c.borrow().as_ref().expect("loaded")))
}

/// `reps` ZK signature inner proofs (one per wallet transaction) with fixed test values.
/// Returns the time of each proof in ms.
pub fn zk_sign_inner(reps: u32) -> anyhow::Result<Vec<f64>> {
    with_circuits(|circuits| {
        let mut times = Vec::new();
        for i in 0..reps as u64 {
            let private_key = QHashOut::<F>::from_values(7 + i, 11, 13, 17);
            let sig_hash = QHashOut::<F>::from_values(19, 23 + i, 29, 31);
            let t = now_ms();
            let proof = circuits.prove_zk_sign_inner(private_key, sig_hash)?;
            times.push(now_ms() - t);
            // Every proof must verify, whatever the thread count or build options.
            circuits.zk_signature_inner().circuit_data.verify(proof)?;
        }
        Ok(times)
    })
}

/// Builds the two privacy circuits (what the first private-note or shield-deposit claim pays when
/// the circuits load on first use). Returns ms.
pub fn privacy_circuits() -> anyhow::Result<f64> {
    with_circuits(|circuits| {
        let t = now_ms();
        core::hint::black_box(circuits.private_note_inclusion_verifier_data());
        core::hint::black_box(circuits.shield_deposit_claim_verifier_data());
        Ok(now_ms() - t)
    })
}

/// `DepositInclusionCircuit::new()` plus its fingerprint, as trace generation does for every
/// shield-deposit claim item. Returns ms.
pub fn deposit_inclusion_new() -> anyhow::Result<f64> {
    let t = now_ms();
    let circuit = DepositInclusionCircuit::<C, D>::new();
    let fingerprint = circuit.get_fingerprint();
    let ms = now_ms() - t;
    core::hint::black_box(fingerprint);
    Ok(ms)
}

/// Threads rayon would use here.
pub fn rayon_threads() -> usize {
    rayon::current_num_threads()
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use wasm_bindgen::prelude::*;

    fn js(e: anyhow::Error) -> JsError {
        JsError::new(&format!("{e:#}"))
    }

    #[wasm_bindgen]
    pub fn lab_load_local_circuits() -> Result<f64, JsError> {
        super::load_local_circuits().map_err(js)
    }

    #[wasm_bindgen]
    pub fn lab_zk_sign_inner(reps: u32) -> Result<Vec<f64>, JsError> {
        super::zk_sign_inner(reps).map_err(js)
    }

    #[wasm_bindgen]
    pub fn lab_privacy_circuits() -> Result<f64, JsError> {
        super::privacy_circuits().map_err(js)
    }

    #[wasm_bindgen]
    pub fn lab_deposit_inclusion_new() -> Result<f64, JsError> {
        super::deposit_inclusion_new().map_err(js)
    }

    /// Bytes of the wasm linear memory.
    #[wasm_bindgen]
    pub fn lab_memory_bytes() -> f64 {
        (core::arch::wasm32::memory_size(0) * 65536) as f64
    }

    #[wasm_bindgen]
    pub fn lab_rayon_threads() -> usize {
        super::rayon_threads()
    }

    #[wasm_bindgen]
    pub fn lab_now_ms() -> f64 {
        super::now_ms()
    }
}
