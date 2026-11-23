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

/// The wallet's accessor for the DepositInclusion circuit (cached per worker), with its
/// fingerprint. Returns ms.
#[cfg(target_arch = "wasm32")]
pub fn deposit_inclusion_cached() -> f64 {
    let t = now_ms();
    let fingerprint = crate::wasm::deposit_inclusion_circuit().get_fingerprint();
    core::hint::black_box(fingerprint);
    now_ms() - t
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
    pub fn lab_deposit_inclusion_cached() -> f64 {
        super::deposit_inclusion_cached()
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

// ---------------------------------------------------------------------------------------------
// Replay of captured prove-proxy requests through the local circuit manager
// (wallet-local-prove-lab-20261123). Each call does what the proxy's user-role handler does
// for that RPC method, without the network, so its time is the cost of proving on this device.
// ---------------------------------------------------------------------------------------------

use std::collections::HashMap;

use plonky2::plonk::proof::ProofWithPublicInputs;
use psy_data::{qdata::contract::ContractCodeDefinition, qstore::controllers::session_info::SessionCircuitInfoStore};
use psy_provider::request::{RequestParams, RequestParamsV2};
use psy_ups_circuit::circuit_manager::core::PsyUPSStepCircuitManager;
use psy_vm::ups::circuit_manager::UPSCircuitManager;

struct Replay {
    manager: PsyUPSStepCircuitManager<C, D>,
    circuit_info: SessionCircuitInfoStore<F>,
    contract_codes: HashMap<u64, ContractCodeDefinition>,
}

thread_local! {
    static REPLAY: RefCell<Option<Replay>> = const { RefCell::new(None) };
}

/// The manager's trait methods are `async` but never wait on anything when the manager is
/// local, so one poll completes them.
fn poll_once<T>(future: impl std::future::Future<Output = T>) -> anyhow::Result<T> {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            raw_waker()
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(&waker)) {
        Poll::Ready(value) => Ok(value),
        Poll::Pending => anyhow::bail!("local circuit manager call did not complete in one poll"),
    }
}

/// Builds the fixed circuits (UPS start, CFC transaction, proof-tree aggregation, End Cap) the
/// way `WalletSession::new` would without a prove proxy. Returns ms.
pub fn replay_init() -> anyhow::Result<f64> {
    plonky2::util::stage_log::set_enabled(true);
    let t = now_ms();
    let manager = PsyUPSStepCircuitManager::<C, D>::new_with_config(psy_config::network_constants::PSY_NETWORK_MAGIC);
    let mut circuit_info = SessionCircuitInfoStore::<F>::new();
    poll_once(manager.register_info(&mut circuit_info))?;
    REPLAY.with(|slot| *slot.borrow_mut() = Some(Replay { manager, circuit_info, contract_codes: HashMap::new() }));
    Ok(now_ms() - t)
}

/// Keeps a contract's code so later `psy_prove_contract_call` replays can build its function
/// circuits. `code_json` is `params.contract_code` of a captured `psy_resolve_contract_function_*`.
pub fn replay_add_contract_code(contract_id: u64, code_json: &str) -> anyhow::Result<usize> {
    let code: ContractCodeDefinition = serde_json::from_str(code_json)?;
    let functions = code.functions.len();
    REPLAY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let replay = slot.as_mut().ok_or_else(|| anyhow::anyhow!("replay_init was not called"))?;
        replay.contract_codes.insert(contract_id, code);
        Ok(functions)
    })
}

/// Fingerprints of the fixed circuits, to compare one build of the manager with another
/// (native against wasm, or either against the proxy that produced the captured requests).
pub fn replay_fingerprints() -> anyhow::Result<String> {
    REPLAY.with(|slot| {
        let slot = slot.borrow();
        let replay = slot.as_ref().ok_or_else(|| anyhow::anyhow!("replay_init was not called"))?;
        let m = &replay.manager;
        let set = &m.proof_tree_agg_circuits.circuit_set;
        Ok(serde_json::json!({
            "ups_start": m.ups_start.get_fingerprint().to_string(),
            "ups_start_register_user": m.ups_start_register_user.get_fingerprint().to_string(),
            "ups_cfc_standard_tx": m.ups_cfc_standard_tx.get_fingerprint().to_string(),
            "ups_cfc_deferred_tx": m.ups_cfc_deferred_tx.get_fingerprint().to_string(),
            "ups_end_cap": m.ups_end_cap.get_fingerprint().to_string(),
            "ups_circuit_whitelist_root": m.ups_circuit_whitelist_root.to_string(),
            "single_leaf": set.single_leaf_circuit.get_fingerprint().to_string(),
            "two_leaf": set.two_leaf_circuit.get_fingerprint().to_string(),
            "two_agg": set.two_agg_circuit.get_fingerprint().to_string(),
            "left_leaf_right_agg": set.left_leaf_right_agg_circuit.get_fingerprint().to_string(),
            "left_agg_right_leaf": set.left_agg_right_leaf_circuit.get_fingerprint().to_string(),
        })
        .to_string())
    })
}

/// `replay_add_contract_code` from a captured `psy_resolve_contract_function_*` file as it is on disk.
pub fn replay_add_contract_code_fixture(fixture_json: &str) -> anyhow::Result<u64> {
    let fixture: serde_json::Value = serde_json::from_str(fixture_json)?;
    let params = &fixture["request"]["params"];
    let contract_id = params["contract_id"].as_u64().ok_or_else(|| anyhow::anyhow!("fixture has no contract_id"))?;
    replay_add_contract_code(contract_id, &params["contract_code"].to_string())?;
    Ok(contract_id)
}

fn proof_from_json(json: &str) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
    Ok(serde_json::from_str(json)?)
}

/// Replays one captured request (`{"method": ..., "params": ...}`). Returns a JSON object with
/// `method`, `decode_ms`, `build_ms` (circuits built because of this request), `prove_ms`,
/// `public_inputs` (count) and, for contract calls, `contract_id`, `fn_id` and `fingerprint`.
pub fn replay(request_json: &str) -> anyhow::Result<String> {
    let t_decode = now_ms();
    let mut value: serde_json::Value = serde_json::from_str(request_json)?;
    // A whole capture file is accepted as it is on disk: field elements are 64-bit JSON numbers,
    // so the text must never pass through a JavaScript JSON round trip.
    if let Some(request) = value.get_mut("request").map(serde_json::Value::take) {
        value = request;
    }
    let method = value.get("method").and_then(|m| m.as_str()).unwrap_or_default().to_string();
    let request = serde_json::json!({ "method": method, "params": value.get_mut("params").map(serde_json::Value::take) });
    let is_v2 = matches!(
        method.as_str(),
        "psy_prove_ups_end_cap"
            | "psy_prove_single_leaf_circuit"
            | "psy_prove_two_leaf_circuit"
            | "psy_prove_two_agg_circuit"
            | "psy_prove_left_leaf_right_agg_circuit"
            | "psy_prove_left_agg_right_leaf_circuit"
    );
    let mut out = serde_json::Map::new();
    out.insert("method".into(), method.clone().into());
    // Drop stage lines left by circuit builds or an earlier failed request.
    plonky2::util::stage_log::take();

    REPLAY.with(|slot| {
        let slot = slot.borrow();
        let replay = slot.as_ref().ok_or_else(|| anyhow::anyhow!("replay_init was not called"))?;
        let manager = &replay.manager;
        let mut build_ms = 0.0;
        let (decode_ms, t_prove, proof) = if is_v2 {
            let request: RequestParamsV2<C, D> = serde_json::from_value(request)?;
            let decode_ms = now_ms() - t_decode;
            let t_prove = now_ms();
            let agg = &manager.proof_tree_agg_circuits;
            let proof = match request {
                RequestParamsV2::UpsEndCap(r) => {
                    let whitelist_proof = agg.circuit_inclusion_proofs.get_inclusion_proof_for_type(r.circuit_type);
                    let verifier_data = replay.circuit_info.get_circuit_info_by_fingerprint(r.fingerprint)?.verifier_data.to_verifier_data::<C, D>();
                    manager.ups_end_cap.prove_base(&r.end_cap_from_proof_tree_input, &whitelist_proof, &r.agg_header, &r.proof, &verifier_data)?
                }
                RequestParamsV2::SingleLeaf(r) => agg.circuit_set.single_leaf_circuit.prove_base(
                    r.agg_circuit_whitelist_root,
                    &r.single_insert_leaf_proof,
                    &r.single_proof,
                    &r.single_verifier_data.to_verifier_data(),
                )?,
                RequestParamsV2::TwoLeaf(r) => agg.circuit_set.two_leaf_circuit.prove_base(
                    r.agg_circuit_whitelist_root,
                    &r.left_insert_leaf_proof,
                    &r.left_proof,
                    &r.left_verifier_data.to_verifier_data(),
                    &r.right_insert_leaf_proof,
                    &r.right_proof,
                    &r.right_verifier_data.to_verifier_data(),
                )?,
                RequestParamsV2::TwoAgg(r) => agg.circuit_set.two_agg_circuit.prove_base(
                    &r.left_agg_whitelist_merkle_proof,
                    &r.left_agg_proof_header,
                    &r.left_proof,
                    &r.left_verifier_data.to_verifier_data(),
                    &r.right_agg_whitelist_merkle_proof,
                    &r.right_agg_proof_header,
                    &r.right_proof,
                    &r.right_verifier_data.to_verifier_data(),
                )?,
                RequestParamsV2::LeftLeafRightAgg(r) => agg.circuit_set.left_leaf_right_agg_circuit.prove_base(
                    &r.left_insert_leaf_proof,
                    &r.left_proof,
                    &r.left_verifier_data.to_verifier_data(),
                    &r.right_agg_whitelist_merkle_proof,
                    &r.right_agg_proof_header,
                    &r.right_proof,
                    &r.right_verifier_data.to_verifier_data(),
                )?,
                RequestParamsV2::LeftAggRightLeaf(r) => agg.circuit_set.left_agg_right_leaf_circuit.prove_base(
                    &r.left_agg_whitelist_merkle_proof,
                    &r.left_agg_proof_header,
                    &r.left_proof,
                    &r.left_verifier_data.to_verifier_data(),
                    &r.right_insert_leaf_proof,
                    &r.right_proof,
                    &r.right_verifier_data.to_verifier_data(),
                )?,
            };
            (decode_ms, t_prove, proof)
        } else {
            let request: RequestParams<F> = serde_json::from_value(request)?;
            let decode_ms = now_ms() - t_decode;
            let mut t_prove = now_ms();
            let proof = match request {
                RequestParams::ProveUpsStart(r) => manager.ups_start.prove_base(&r.input)?,
                RequestParams::ProveUpsStartRegisterUser(r) => manager.ups_start_register_user.prove_base(&r.input)?,
                RequestParams::UpsCfcStandardTx(r) => manager.ups_cfc_standard_tx.prove_base(&r.input)?,
                RequestParams::UpsCfcDeferredTx(r) => manager.ups_cfc_deferred_tx.prove_base(&r.input)?,
                RequestParams::ProveContractCall(r) => {
                    out.insert("contract_id".into(), r.contract_id.into());
                    out.insert("fn_id".into(), r.fn_id.into());
                    if manager.contract_function_registration(r.contract_id, r.fn_id).is_none() {
                        let code = replay
                            .contract_codes
                            .get(&r.contract_id)
                            .ok_or_else(|| anyhow::anyhow!("no contract code for contract {}", r.contract_id))?;
                        let method_id = code
                            .functions
                            .get(r.fn_id as usize)
                            .ok_or_else(|| anyhow::anyhow!("contract {} has no function {}", r.contract_id, r.fn_id))?
                            .method_id;
                        poll_once(manager.resolve_contract_function_by_method_id(r.contract_id, code, method_id))??;
                    }
                    // A cache miss rebuilds the function circuit here, as on the proxy.
                    let circuit = manager.contract_function_circuit(r.contract_id, r.fn_id)?;
                    build_ms = now_ms() - t_prove;
                    t_prove = now_ms();
                    if let Some(registration) = manager.contract_function_registration(r.contract_id, r.fn_id) {
                        out.insert("fingerprint".into(), registration.fingerprint.to_string().into());
                    }
                    circuit.prove_base(&r.input)?
                }
                RequestParams::ZKSignatureMinifierProof(r) => {
                    let inner = proof_from_json(&r.inner_proof)?;
                    poll_once(manager.prove_zk_sign_minifier(inner))??
                }
                RequestParams::PrivateNoteInclusionMinifierProof(r) => {
                    let base = proof_from_json(&r.base_proof)?;
                    let circuit = manager.private_note_inclusion_minifier_circuit();
                    build_ms = now_ms() - t_prove;
                    t_prove = now_ms();
                    circuit.prove_minifier(base)?
                }
                RequestParams::ShieldDepositClaimMinifierProof(r) => {
                    let base = proof_from_json(&r.base_proof)?;
                    let circuit = manager.shield_deposit_claim_minifier_circuit();
                    build_ms = now_ms() - t_prove;
                    t_prove = now_ms();
                    circuit.prove_minifier(base)?
                }
                _ => anyhow::bail!("replay does not handle {}", method),
            };
            (decode_ms, t_prove, proof)
        };
        out.insert("decode_ms".into(), decode_ms.into());
        out.insert("build_ms".into(), build_ms.into());
        out.insert("prove_ms".into(), (now_ms() - t_prove).into());
        out.insert("public_inputs".into(), proof.public_inputs.len().into());
        // One entry per plonky2 proof inside this request (base proof, then minifiers).
        let stages: Vec<serde_json::Value> =
            plonky2::util::stage_log::take().iter().filter_map(|line| serde_json::from_str(line).ok()).collect();
        out.insert("proofs".into(), stages.into());
        let (cached_bytes, budget_bytes, cached_circuits) = manager.contract_prover_cache_usage();
        out.insert("contract_cache_bytes".into(), cached_bytes.into());
        out.insert("contract_cache_budget".into(), budget_bytes.into());
        out.insert("contract_cache_circuits".into(), cached_circuits.into());
        Ok(serde_json::Value::Object(out).to_string())
    })
}

#[cfg(target_arch = "wasm32")]
mod replay_exports {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn lab_replay_init() -> Result<f64, JsError> {
        super::replay_init().map_err(|e| JsError::new(&format!("{e:#}")))
    }

#[wasm_bindgen]
    pub fn lab_replay_add_contract_code_fixture(fixture_json: &str) -> Result<u64, JsError> {
        super::replay_add_contract_code_fixture(fixture_json).map_err(|e| JsError::new(&format!("{e:#}")))
    }

#[wasm_bindgen]
    pub fn lab_replay(request_json: &str) -> Result<String, JsError> {
        super::replay(request_json).map_err(|e| JsError::new(&format!("{e:#}")))
    }

    #[wasm_bindgen]
    pub fn lab_replay_fingerprints() -> Result<String, JsError> {
        super::replay_fingerprints().map_err(|e| JsError::new(&format!("{e:#}")))
    }
}
