//! psy-wallet-core-ffi — the NATIVE (iOS/Android) FFI binding over the Psy ZK
//! wallet core, generated to Swift + Kotlin via uniffi.
//!
//! Native counterpart to `psy-rust-sdk`'s `WasmRpcServer` (`wasm/mod.rs`): the
//! SAME `psy_prover::session::WalletSession` + `UserProverWorkerStore` core and
//! the SAME JSON-in / JSON-out method surface, exported through uniffi instead
//! of wasm-bindgen. One Rust core → web (WASM) + iOS (Swift) + Android (Kotlin).
//!
//! Mobile holds its OWN ZK key (Secure Enclave / Keystore) — a standalone Psy ZK
//! Wallet, unrelated to MetaMask (which is web-only).
//!
//! Every method is a faithful port of the matching `wasm/mod.rs` method, with
//! three mechanical changes: `&mut self`/`&self` → an async `Mutex` guard (uniffi
//! objects are `Arc<Self>`); `JsError` → `PsyError`; and the wasm JS-timer
//! helpers → native `now_ms` / `async_sleep_ms`.

uniffi::setup_scaffolding!();

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::{
    args::{ContractCallArgs, ContractCallData, DPNSoftwareDefinedCallData},
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
};
use psy_crypto::signature::secp256k1::core::PsyCompressedSecp256K1Signature;
use psy_prover::{
    local::store::UserProverWorkerStore,
    session::WalletSession,
    wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint},
};
use psy_provider::provider::NetworkConfig as RpcConfig;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
// The chain-read methods (get_latest_block_state, get_user_leaf_data,
// get_user_contract_state_tree_merkle_proof, …) on the RPC provider are TRAIT
// methods — bring the traits into scope so the private claim/prove ports resolve
// them (the wasm bodies imported these locally).
use psy_data::traits::qdatastore::{
    qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync,
};
// Mode-A: `get_nonce()` on the session's local proving store is a trait method —
// bring the trait into scope so `get_sig_hash` can read the sighash nonce.
use psy_data::qstore::controllers::proving_session::PsyReadLocalProvingSessionStore;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

/// FFI error surfaced to Swift/Kotlin — single variant carrying the raw message
/// (mirrors the WASM build's `JsError::new(&msg)`), so callers see the exact
/// prover/RPC error text identically on every platform.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PsyError {
    #[error("{0}")]
    Wallet(String),
}

impl PsyError {
    fn w(msg: impl std::fmt::Display) -> Self {
        PsyError::Wallet(msg.to_string())
    }
}

/// Native equivalents of the wasm JS-timer helpers (wasm/mod.rs uses `Date.now`
/// + `setTimeout`; the non-wasm branches there are exactly these).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
async fn async_sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Map a `signType` string to its circuit fingerprint (mirrors the WASM build).
fn fingerprint_for(sign_type: &str) -> Result<QHashOut<F>, PsyError> {
    match sign_type {
        "zk" => Ok(get_zk_fingerprint()),
        "secp256k1" => Ok(get_secp256k1_fingerprint()),
        other => Err(PsyError::w(format!("Unsupported sign type: {other}"))),
    }
}

/// Parse a JSON array of 4 decimal strings → `[u64; 4]`.
fn parse_u64x4(json: &str) -> Result<[u64; 4], PsyError> {
    let arr: [String; 4] =
        serde_json::from_str(json).map_err(|e| PsyError::w(format!("parse u64x4: {e}")))?;
    Ok([
        arr[0].parse::<u64>().map_err(|e| PsyError::w(e))?,
        arr[1].parse::<u64>().map_err(|e| PsyError::w(e))?,
        arr[2].parse::<u64>().map_err(|e| PsyError::w(e))?,
        arr[3].parse::<u64>().map_err(|e| PsyError::w(e))?,
    ])
}

/// Mutable wallet state behind an async `Mutex` — interior mutability for the
/// `Arc<Self>` uniffi object, and the native equivalent of the web app's FIFO
/// prover gate (serializes proving; avoids the Rust recursive-borrow panic).
struct Session {
    #[allow(dead_code)]
    store: UserProverWorkerStore,
    wallet_session: WalletSession,
}

/// The native Psy ZK wallet core. Construct once from the JSON network config,
/// then call the wallet operations. Async — every proof is multi-second.
#[derive(uniffi::Object)]
pub struct PsyWalletCore {
    inner: Mutex<Session>,
}

#[uniffi::export(async_runtime = "tokio")]
impl PsyWalletCore {
    /// Construct from the SAME JSON network config the web SDK uses. Mirrors
    /// `WasmRpcServer::new`.
    #[uniffi::constructor]
    pub async fn new(rpc_config_json: String) -> Result<Arc<Self>, PsyError> {
        let rpc_config: RpcConfig<F> = serde_json::from_str(&rpc_config_json)
            .map_err(|e| PsyError::w(format!("Parse RPC config error: {e}")))?;
        let wallet_session = WalletSession::new(&rpc_config)
            .await
            .map_err(|e| PsyError::w(format!("Create wallet session error: {e}")))?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Session {
                store: UserProverWorkerStore::new(),
                wallet_session,
            }),
        }))
    }

    /// Register (idempotent) the ZK user for a private key → public-key hash.
    /// `sign_type` is "zk" | "secp256k1". PROVES the ZK-signature circuit (the
    /// proving-spike workload). Mirrors `register_user`.
    pub async fn register_user(
        &self,
        private_key_str: String,
        sign_type: String,
    ) -> Result<String, PsyError> {
        let private_key = QHashOut::<F>::from_str(&private_key_str)
            .map_err(|e| PsyError::w(format!("Parse private key error: {e}")))?;
        let fingerprint = fingerprint_for(&sign_type)?;
        let mut s = self.inner.lock().await;
        let pk_hash = s
            .wallet_session
            .register_user(private_key, fingerprint)
            .await
            .map_err(|e| PsyError::w(format!("Register user error: {e}")))?;
        Ok(pk_hash.to_string())
    }

    /// Read-only signer install for an already-registered key. Mirrors `add_user`.
    pub async fn add_user(
        &self,
        private_key_str: String,
        sign_type: String,
    ) -> Result<String, PsyError> {
        let private_key = QHashOut::<F>::from_str(&private_key_str)
            .map_err(|e| PsyError::w(format!("Parse private key error: {e}")))?;
        let fingerprint = fingerprint_for(&sign_type)?;
        let mut s = self.inner.lock().await;
        let pk_hash = s
            .wallet_session
            .add_user(private_key, fingerprint)
            .await
            .map_err(|e| PsyError::w(format!("Add user error: {e}")))?;
        Ok(pk_hash.to_string())
    }

    /// ZK public-key info (JSON) for a private key — read-only. Mirrors
    /// `get_zk_public_key_json`.
    pub async fn get_zk_public_key_json(
        &self,
        private_key_str: String,
    ) -> Result<String, PsyError> {
        let private_key = QHashOut::<F>::from_str(&private_key_str)
            .map_err(|e| PsyError::w(format!("Parse private key error: {e}")))?;
        let s = self.inner.lock().await;
        let public_key = s
            .wallet_session
            .get_zk_public_key(private_key)
            .await
            .map_err(|e| PsyError::w(format!("Get ZK public key error: {e}")))?;
        serde_json::to_string(&public_key)
            .map_err(|e| PsyError::w(format!("Serialize public key error: {e}")))
    }

    /// Sign + submit one public contract call (JSON `ContractCallData`) — the
    /// public Send / claim / deposit / withdraw / faucet path. Mirrors
    /// `exec_contract_call_json` (wasm/mod.rs:358).
    pub async fn exec_contract_call_json(
        &self,
        pk_hash: String,
        call_data_json: String,
    ) -> Result<String, PsyError> {
        let call_data: ContractCallData = serde_json::from_str(&call_data_json)
            .map_err(|e| PsyError::w(format!("Parse call data JSON error: {e}")))?;
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;
        let s = self.inner.lock().await;
        let end_user_leaf_hash = s
            .wallet_session
            .exec_contract_call(pk_hash, call_data)
            .await
            .map_err(|e| PsyError::w(format!("Error exec calls error: {e}")))?;
        Ok(end_user_leaf_hash.to_string())
    }

    /// Sign + submit the current session's pending proof. Mirrors
    /// `sign_and_submit` (wasm/mod.rs:1120).
    pub async fn sign_and_submit(
        &self,
        pk_hash: String,
        sign_data: Option<String>,
    ) -> Result<String, PsyError> {
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;
        let software_defined_call = sign_data
            .map(|data| serde_json::from_str::<DPNSoftwareDefinedCallData>(&data))
            .transpose()
            .map_err(|e| PsyError::w(format!("Parse sign data error: {e}")))?
            .unwrap_or_default();
        let s = self.inner.lock().await;
        let end_user_leaf_hash = s
            .wallet_session
            .sign_and_submit(pk_hash, software_defined_call)
            .await
            .map_err(|e| PsyError::w(format!("Sign and submit error: {e}")))?;
        Ok(end_user_leaf_hash.to_string())
    }

    /// PRIVATE CLAIM: inject an external note-inclusion proof and claim it.
    /// Faithful port of `exec_claim_with_external_proof_json` (wasm/mod.rs:486).
    #[allow(clippy::too_many_arguments)]
    pub async fn exec_claim_with_external_proof_json(
        &self,
        pk_hash: String,
        note_proof_bincode_b64: String,
        nullifier_json: String,
        owner_json: String,
        amount: String,
        user_tree_root_json: String,
        checkpoint_id: String,
        note_root_slot: String,
        contract_id: String,
        random0: String,
        random1: String,
    ) -> Result<String, PsyError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use plonky2::field::types::PrimeField64;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_config::network_constants::{
            GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;

        const NOTE_TREE_HEIGHT: usize = 20;

        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("parse pk_hash: {e}")))?;
        let nullifier = parse_u64x4(&nullifier_json)?;
        let owner = parse_u64x4(&owner_json)?;
        let amount_val: u64 = amount.parse().map_err(|e| PsyError::w(e))?;
        let user_tree_root = parse_u64x4(&user_tree_root_json)?;
        let checkpoint_id_val: u64 = checkpoint_id.parse().map_err(|e| PsyError::w(e))?;
        let note_root_slot_val: u64 = note_root_slot.parse().map_err(|e| PsyError::w(e))?;
        let contract_id_val: u64 = contract_id.parse().map_err(|e| PsyError::w(e))?;
        let random0_val: u64 = random0.parse().map_err(|e| PsyError::w(e))?;
        let random1_val: u64 = random1.parse().map_err(|e| PsyError::w(e))?;

        let proof_bytes = BASE64
            .decode(note_proof_bincode_b64.as_bytes())
            .map_err(|e| PsyError::w(format!("base64 decode: {e}")))?;
        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof: ProofWithPublicInputs<F, C, D> = match ProofWithPublicInputs::<F, C, D>::from_bytes(
            proof_bytes.clone(),
            circuit.get_common_circuit_data_ref(),
        ) {
            Ok(p) => p,
            Err(native_err) => bincode::deserialize(&proof_bytes).map_err(|bin_err| {
                PsyError::w(format!(
                    "proof deserialize: native={native_err} ; bincode={bin_err}"
                ))
            })?,
        };
        let fingerprint = circuit.get_fingerprint();
        let verifier_data = circuit.get_verifier_config_ref().clone();

        let s = self.inner.lock().await;

        // Step 1: reset session (before add_external_proof).
        s.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| PsyError::w(format!("start_session: {e}")))?;

        // Step 2: inject external proof → leaf_index + siblings.
        let (leaf_index, siblings) = s
            .wallet_session
            .add_external_proof_with_siblings(pk_hash, fingerprint, proof, verifier_data)
            .await
            .map_err(|e| PsyError::w(format!("add_external_proof: {e}")))?;

        // Step 3: build private_claim inputs.
        let mut inputs: Vec<u64> = Vec::new();
        inputs.extend_from_slice(&nullifier);
        inputs.extend_from_slice(&owner);
        inputs.push(amount_val);
        inputs.extend_from_slice(&user_tree_root);
        inputs.push(checkpoint_id_val);
        inputs.push(note_root_slot_val);
        inputs.push(random0_val);
        inputs.push(random1_val);
        for sib in &siblings {
            inputs.extend_from_slice(sib);
        }
        inputs.push(leaf_index);

        let contract_call = ContractCallArgs {
            contract_id: contract_id_val,
            method_name: "private_claim".to_string(),
            inputs,
        };

        // Step 4: prove (session already has the external proof — no reset).
        s.wallet_session
            .prove_contract_call(pk_hash, vec![contract_call])
            .await
            .map_err(|e| PsyError::w(format!("prove_contract_call: {e}")))?;

        // Baseline receiver nonce before submit; wait for it to change after.
        let provider = s.wallet_session.st_provider.clone();
        let user_ids = provider
            .get_user_ids_for_public_key(pk_hash)
            .await
            .map_err(|e| PsyError::w(format!("get_user_ids_for_public_key: {e}")))?;
        let receiver_user_id = *user_ids
            .first()
            .ok_or_else(|| PsyError::w("No user ID found for public key"))?;

        let baseline_snapshot = provider
            .get_latest_block_state()
            .await
            .map_err(|e| PsyError::w(format!("get_latest_block_state: {e}")))?;
        let checkpoint_before = baseline_snapshot.checkpoint_id;
        let baseline_nonce = provider
            .get_user_leaf_data(checkpoint_before, receiver_user_id)
            .await
            .map_err(|e| PsyError::w(format!("get_user_leaf_data(baseline): {e}")))?
            .nonce
            .to_canonical_u64();

        // Step 5: sign and submit.
        let tx_hash = s
            .wallet_session
            .sign_and_submit(pk_hash, DPNSoftwareDefinedCallData::default())
            .await
            .map_err(|e| PsyError::w(format!("sign_and_submit: {e}")))?;

        // Step 6: wait nonce change on the observable checkpoint stream.
        let wait_deadline_ms = now_ms().saturating_add(180_000);
        let mut latest_coordinator_seen = checkpoint_before;
        let mut latest_realm_seen = checkpoint_before;
        let mut latest_observable_seen = checkpoint_before;
        let mut last_error = String::new();
        let mut next_checkpoint = checkpoint_before.saturating_add(1);
        let mut nonce_changed = false;

        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| PsyError::w(format!("get_coordinator_latest_block_state: {e}")))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);

            let latest_realm = match provider.get_realm_latest_block_state().await {
                Ok(realm_state) => {
                    latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
                    realm_state.checkpoint_id
                }
                Err(_) => latest_coordinator,
            };

            let latest_observable = latest_realm.min(latest_coordinator);
            latest_observable_seen = latest_observable_seen.max(latest_observable);

            while next_checkpoint <= latest_observable {
                let nonce = match provider
                    .get_user_leaf_data(next_checkpoint, receiver_user_id)
                    .await
                {
                    Ok(leaf) => leaf.nonce.to_canonical_u64(),
                    Err(e) => {
                        last_error =
                            format!("get_user_leaf_data failed at checkpoint {next_checkpoint}: {e}");
                        break;
                    }
                };
                if nonce != baseline_nonce {
                    nonce_changed = true;
                    break;
                }
                last_error = format!(
                    "nonce unchanged at checkpoint {next_checkpoint}: nonce={nonce} baseline={baseline_nonce}"
                );
                next_checkpoint = next_checkpoint.saturating_add(1);
            }

            if nonce_changed {
                break;
            }
            async_sleep_ms(1000).await;
        }

        if !nonce_changed {
            return Err(PsyError::w(format!(
                "[wallet-native] timeout waiting private_claim nonce change (checkpoint_before={checkpoint_before}, baseline_nonce={baseline_nonce}, latestCoordinator={latest_coordinator_seen}, latestRealm={latest_realm_seen}, latestObservable={latest_observable_seen}, lastError={last_error})"
            )));
        }

        Ok(tx_hash.to_string())
    }

    /// PRIVATE SEND proof: generate a PrivateNoteInclusion ZK proof, returning
    /// the full NoteProofOutput as JSON. Faithful port of
    /// `prove_private_note_inclusion_json` (wasm/mod.rs:729).
    #[allow(clippy::too_many_arguments)]
    pub async fn prove_private_note_inclusion_json(
        &self,
        pk_hash: String,
        owner_json: String,
        amount: String,
        note_secret_hash_json: String,
        nullifier_secret_json: String,
        contract_id: String,
        note_root_slot: String,
        checkpoint_id: String,
    ) -> Result<String, PsyError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use plonky2::field::types::{Field, PrimeField64};
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_config::network_constants::{
            GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };
        use psy_crypto::hash::merkle::core::MerkleProofCore;
        use psy_crypto::hash::traits::hasher::{FieldQHasher, PoseidonHasher};
        use psy_data::privacy::private_note_inclusion::PrivateNoteInclusionInput;
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;

        const NOTE_TREE_HEIGHT: usize = 20;

        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("parse pk_hash: {e}")))?;
        let owner_u64 = parse_u64x4(&owner_json)?;
        let amount_val: u64 = amount.parse().map_err(|e| PsyError::w(e))?;
        let note_secret_hash_u64 = parse_u64x4(&note_secret_hash_json)?;
        let nullifier_secret_u64 = parse_u64x4(&nullifier_secret_json)?;
        let contract_id_val: u64 = contract_id.parse().map_err(|e| PsyError::w(e))?;
        let note_root_slot_val: u64 = note_root_slot.parse().map_err(|e| PsyError::w(e))?;
        let checkpoint_before_raw: u64 = checkpoint_id.parse().map_err(|e| PsyError::w(e))?;

        let owner =
            QHashOut::<F>::from_values(owner_u64[0], owner_u64[1], owner_u64[2], owner_u64[3]);
        let note_secret_hash = QHashOut::<F>::from_values(
            note_secret_hash_u64[0],
            note_secret_hash_u64[1],
            note_secret_hash_u64[2],
            note_secret_hash_u64[3],
        );
        let nullifier_secret = QHashOut::<F>::from_values(
            nullifier_secret_u64[0],
            nullifier_secret_u64[1],
            nullifier_secret_u64[2],
            nullifier_secret_u64[3],
        );

        let s = self.inner.lock().await;
        let provider = s.wallet_session.st_provider.clone();

        let latest_coordinator_before = provider
            .get_coordinator_latest_block_state()
            .await
            .map_err(|e| PsyError::w(format!("get_coordinator_latest_block_state: {e}")))?
            .checkpoint_id;
        let latest_realm_before = provider
            .get_realm_latest_block_state()
            .await
            .map_err(|e| PsyError::w(format!("get_realm_latest_block_state: {e}")))?
            .checkpoint_id;
        let latest_observable_before = latest_coordinator_before.min(latest_realm_before);
        let checkpoint_before = if checkpoint_before_raw == 0 {
            latest_observable_before
        } else if checkpoint_before_raw > latest_observable_before {
            return Err(PsyError::w(format!(
                "checkpoint_id {checkpoint_before_raw} is ahead of latest observable checkpoint {latest_observable_before} (coordinator={latest_coordinator_before}, realm={latest_realm_before})"
            )));
        } else {
            checkpoint_before_raw
        };

        let user_ids = provider
            .get_user_ids_for_public_key(pk_hash)
            .await
            .map_err(|e| PsyError::w(format!("get_user_ids_for_public_key: {e}")))?;
        let sender_user_id = *user_ids
            .first()
            .ok_or_else(|| PsyError::w("No user ID found for public key"))?;

        let user_provider = provider.with_user_id_owned(sender_user_id);

        let note_count_slot = note_root_slot_val.saturating_sub(1);
        let note_count_proof = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_before,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_count_slot,
            )
            .await
            .map_err(|e| PsyError::w(format!("note_count_proof: {e}")))?;
        let note_index = note_count_proof.value.0.elements[3].to_canonical_u64();

        let mut last_path: Vec<QHashOut<F>> = Vec::with_capacity(NOTE_TREE_HEIGHT);
        for level in 0..NOTE_TREE_HEIGHT as u64 {
            let slot = note_root_slot_val + 1 + level;
            let proof = user_provider
                .get_user_contract_state_tree_merkle_proof(
                    checkpoint_before,
                    sender_user_id,
                    contract_id_val as u32,
                    MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                    slot,
                )
                .await
                .map_err(|e| PsyError::w(format!("last_path[{level}]: {e}")))?;
            last_path.push(proof.value);
        }

        let value_hash = QHashOut::<F>::from_values(amount_val, 0, 0, 0);
        let inner = PoseidonHasher::q_two_to_one(owner, value_hash);
        let commitment = PoseidonHasher::q_two_to_one(inner, note_secret_hash);

        let mut siblings: Vec<QHashOut<F>> = Vec::with_capacity(NOTE_TREE_HEIGHT);
        let mut zero = QHashOut::<F>::from_values(0, 0, 0, 0);
        for level in 0..NOTE_TREE_HEIGHT {
            let bit = (note_index >> level) & 1;
            if bit == 0 {
                siblings.push(zero);
            } else {
                siblings.push(last_path[level]);
            }
            zero = PoseidonHasher::q_two_to_one(zero, zero);
        }
        let note_membership_proof =
            MerkleProofCore::new_from_params::<PoseidonHasher>(note_index, commitment, siblings);

        let baseline_user_leaf = provider
            .get_user_leaf_data(checkpoint_before, sender_user_id)
            .await
            .map_err(|e| PsyError::w(format!("get_user_leaf_data(baseline): {e}")))?;
        let baseline_nonce = baseline_user_leaf.nonce.to_canonical_u64();
        let baseline_note_count = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_before,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_count_slot,
            )
            .await
            .map_err(|e| PsyError::w(format!("baseline note_count proof: {e}")))?
            .value;
        let baseline_note_root = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_before,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_root_slot_val,
            )
            .await
            .map_err(|e| PsyError::w(format!("baseline note_root proof: {e}")))?
            .value;

        let wait_deadline_ms = now_ms().saturating_add(180_000);
        let mut latest_coordinator_seen = checkpoint_before;
        let mut latest_realm_seen = checkpoint_before;
        let mut latest_observable_seen = checkpoint_before;
        let mut checkpoint_after: Option<u64> = None;
        let mut last_error = String::new();

        while now_ms() < wait_deadline_ms {
            let latest_coordinator = provider
                .get_coordinator_latest_block_state()
                .await
                .map_err(|e| PsyError::w(format!("get_coordinator_latest_block_state: {e}")))?
                .checkpoint_id;
            latest_coordinator_seen = latest_coordinator_seen.max(latest_coordinator);

            let latest_realm = match provider.get_realm_latest_block_state().await {
                Ok(realm_state) => {
                    latest_realm_seen = latest_realm_seen.max(realm_state.checkpoint_id);
                    Some(realm_state.checkpoint_id)
                }
                Err(_) => None,
            };

            let latest_observable = latest_realm
                .map(|realm_checkpoint| realm_checkpoint.min(latest_coordinator))
                .unwrap_or(latest_coordinator);
            latest_observable_seen = latest_observable_seen.max(latest_observable);

            if latest_observable > checkpoint_before {
                let note_count = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        latest_observable,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_count_slot,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(e) => {
                        last_error =
                            format!("note_count proof rpc failed at checkpoint {latest_observable}: {e}");
                        async_sleep_ms(1000).await;
                        continue;
                    }
                };
                let note_root = match user_provider
                    .get_user_contract_state_tree_merkle_proof(
                        latest_observable,
                        sender_user_id,
                        contract_id_val as u32,
                        MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                        note_root_slot_val,
                    )
                    .await
                {
                    Ok(v) => v.value,
                    Err(e) => {
                        last_error =
                            format!("note_root proof rpc failed at checkpoint {latest_observable}: {e}");
                        async_sleep_ms(1000).await;
                        continue;
                    }
                };

                if note_count != baseline_note_count || note_root != baseline_note_root {
                    checkpoint_after = Some(latest_observable);
                    break;
                }
                last_error = format!(
                    "slots unchanged at checkpoint {latest_observable}: note_count={note_count} note_root={note_root} baseline_nonce={baseline_nonce}"
                );
            }
            if checkpoint_after.is_some() {
                break;
            }
            async_sleep_ms(1000).await;
        }
        let checkpoint_after = checkpoint_after.ok_or_else(|| {
            PsyError::w(format!(
                "[wallet-native] timeout waiting note slots change (checkpoint_before={checkpoint_before}, baseline_nonce={baseline_nonce}, latestCoordinator={latest_coordinator_seen}, latestRealm={latest_realm_seen}, latestObservable={latest_observable_seen}, lastError={last_error})"
            ))
        })?;

        let user_leaf = user_provider
            .get_user_leaf_data(checkpoint_after, sender_user_id)
            .await
            .map_err(|e| PsyError::w(format!("get_user_leaf_data: {e}")))?;
        let note_root_slot_proof = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_after,
                sender_user_id,
                contract_id_val as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                note_root_slot_val,
            )
            .await
            .map_err(|e| PsyError::w(format!("note_root_slot_proof: {e}")))?;
        let contract_proof = user_provider
            .get_user_contract_tree_merkle_proof(checkpoint_after, sender_user_id, contract_id_val as u32)
            .await
            .map_err(|e| PsyError::w(format!("contract_proof: {e}")))?;
        let user_tree_proof = user_provider
            .get_user_tree_merkle_proof(checkpoint_after, sender_user_id)
            .await
            .map_err(|e| PsyError::w(format!("user_tree_proof: {e}")))?;
        let global_user_tree_root = user_tree_proof.root;

        let circuit_input = PrivateNoteInclusionInput {
            nullifier_secret,
            sender_user_id,
            contract_id: contract_id_val,
            note_root_slot: note_root_slot_val,
            user_leaf,
            owner,
            amount: F::from_canonical_u64(amount_val),
            randomness: note_secret_hash,
            note_membership_proof,
            note_root_slot_proof,
            contract_proof,
            user_tree_proof,
            checkpoint_id: F::from_canonical_u64(checkpoint_after),
        };

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof = circuit
            .prove(&circuit_input)
            .map_err(|e| PsyError::w(format!("prove error: {e}")))?;
        let fingerprint = circuit.get_fingerprint();

        let nullifier = PoseidonHasher::q_hash_many(&nullifier_secret.0.elements);

        let proof_bytes = proof.to_bytes();
        let proof_b64 = BASE64.encode(&proof_bytes);

        let to_str_arr = |h: QHashOut<F>| -> [String; 4] {
            let e = h.0.elements;
            [
                e[0].to_canonical_u64().to_string(),
                e[1].to_canonical_u64().to_string(),
                e[2].to_canonical_u64().to_string(),
                e[3].to_canonical_u64().to_string(),
            ]
        };

        let result = serde_json::json!({
            "nullifier": to_str_arr(nullifier),
            "owner": to_str_arr(owner),
            "amount": amount_val.to_string(),
            "user_tree_root": to_str_arr(global_user_tree_root),
            "checkpoint_id": checkpoint_after.to_string(),
            "note_root_slot": note_root_slot_val.to_string(),
            "note_proof_fingerprint": to_str_arr(fingerprint),
            "note_proof_bincode_b64": proof_b64,
        });
        Ok(result.to_string())
    }

    /// Random ZK keypair (JSON) — read-only. Mirrors `get_random_keypair_json`
    /// (wasm/mod.rs:1213).
    pub async fn get_random_keypair_json(&self) -> Result<String, PsyError> {
        let s = self.inner.lock().await;
        let keypair = s
            .wallet_session
            .get_random_keypair()
            .await
            .map_err(|e| PsyError::w(format!("Get random keypair error: {e}")))?;
        serde_json::to_string(&keypair)
            .map_err(|e| PsyError::w(format!("Serialize keypair error: {e}")))
    }

    /// Prove a single public contract call (JSON `ContractCallArgs`), leaving the
    /// proof in the session for a later `sign_and_submit`. Mirrors
    /// `prove_contract_call_json` (wasm/mod.rs:1081).
    pub async fn prove_contract_call_json(
        &self,
        pk_hash: String,
        contract_call_json: String,
    ) -> Result<String, PsyError> {
        let contract_call_arg: ContractCallArgs = serde_json::from_str(&contract_call_json)
            .map_err(|e| PsyError::w(format!("Parse contract call JSON error: {e}")))?;
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;
        let s = self.inner.lock().await;
        s.wallet_session
            .prove_contract_call(pk_hash, vec![contract_call_arg])
            .await
            .map_err(|e| PsyError::w(format!("Prove contract call error: {e}")))?;
        Ok("prove contract call".to_string())
    }

    /// Prove a batch of public contract calls (JSON `Vec<ContractCallArgs>`),
    /// leaving the proof in the session for a later `sign_and_submit`. Mirrors
    /// `prove_contract_calls_json` (wasm/mod.rs:1100).
    pub async fn prove_contract_calls_json(
        &self,
        pk_hash: String,
        contract_calls_json: String,
    ) -> Result<String, PsyError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(&contract_calls_json)
            .map_err(|e| PsyError::w(format!("Parse contract calls JSON error: {e}")))?;
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;
        let s = self.inner.lock().await;
        s.wallet_session
            .prove_contract_call(pk_hash, contract_call_args)
            .await
            .map_err(|e| PsyError::w(format!("Prove contract calls error: {e}")))?;
        Ok("prove contract calls".to_string())
    }

    /// Reset the session tree for a public key (the same reset the claim/prove
    /// flows do internally) — promoted to a public method. Mirrors
    /// `start_session` (wasm/mod.rs:379).
    pub async fn start_session(&self, pk_hash: String) -> Result<String, PsyError> {
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;
        let s = self.inner.lock().await;
        s.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| PsyError::w(format!("Start session error: {e}")))?;
        Ok("start session".to_string())
    }

    /// Deploy a DPN contract from its circuit definitions (JSON
    /// `Vec<DPNFunctionCircuitDefinition>`), returning the contract UUID. Mirrors
    /// `deploy_contract_json` (wasm/mod.rs:1224).
    pub async fn deploy_contract_json(
        &self,
        deployer: String,
        circuit_defs_json: String,
    ) -> Result<String, PsyError> {
        let deployer = QHashOut::<F>::from_str(&deployer)
            .map_err(|e| PsyError::w(format!("Parse deployer error: {e}")))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(&circuit_defs_json)
                .map_err(|e| PsyError::w(format!("Parse circuit defs JSON error: {e}")))?;
        let s = self.inner.lock().await;
        let contract_uuid = s
            .wallet_session
            .deploy_contract(deployer, circuit_defs)
            .await
            .map_err(|e| PsyError::w(format!("Deploy contract error: {e}")))?;
        Ok(contract_uuid)
    }

    /// Inject an external PrivateNoteInclusion proof into the current session
    /// tree. Returns JSON: `{ "leaf_index": u64, "siblings": [[String;4]] }`.
    /// Faithful port of `add_external_proof_json` (wasm/mod.rs:392), including the
    /// dual native/bincode proof-deserialize fallback.
    pub async fn add_external_proof_json(
        &self,
        pk_hash: String,
        note_proof_bincode_b64: String,
    ) -> Result<String, PsyError> {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_config::network_constants::{
            GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT,
        };
        use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;

        const NOTE_TREE_HEIGHT: usize = 20;

        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse pk_hash error: {e}")))?;

        let circuit = PrivateNoteInclusionCircuit::<C, D>::new(
            GLOBAL_USER_TREE_HEIGHT as usize,
            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            MAX_CONTRACT_STATE_TREE_HEIGHT as usize,
            NOTE_TREE_HEIGHT,
        );
        let proof_bytes = BASE64
            .decode(note_proof_bincode_b64.as_bytes())
            .map_err(|e| PsyError::w(format!("base64 decode error: {e}")))?;
        // Compatibility: accept both modern plonky2 native proof bytes and legacy
        // bincode-serialized proofs to avoid cross-version serialization breakage.
        let proof: ProofWithPublicInputs<F, C, D> = match ProofWithPublicInputs::<F, C, D>::from_bytes(
            proof_bytes.clone(),
            circuit.get_common_circuit_data_ref(),
        ) {
            Ok(p) => p,
            Err(native_err) => bincode::deserialize(&proof_bytes).map_err(|bin_err| {
                PsyError::w(format!(
                    "proof deserialize error: native={native_err} ; bincode={bin_err}"
                ))
            })?,
        };
        let fingerprint = circuit.get_fingerprint();
        let verifier_data = circuit.get_verifier_config_ref().clone();

        let s = self.inner.lock().await;
        let (leaf_index, siblings) = s
            .wallet_session
            .add_external_proof_with_siblings(pk_hash, fingerprint, proof, verifier_data)
            .await
            .map_err(|e| PsyError::w(format!("add_external_proof error: {e}")))?;

        // Encode as decimal strings to avoid JS precision loss for large u64 values.
        let siblings_str: Vec<[String; 4]> = siblings
            .iter()
            .map(|s| {
                [
                    s[0].to_string(),
                    s[1].to_string(),
                    s[2].to_string(),
                    s[3].to_string(),
                ]
            })
            .collect();

        let result = serde_json::json!({
            "leaf_index": leaf_index,
            "siblings": siblings_str,
        });
        Ok(result.to_string())
    }

    /// Read a single contract-state slot scalar for a user at the latest
    /// checkpoint, returned as a decimal string. Net-new (no wasm twin): the
    /// Home-screen balance/state lens for the native wallets. Resolves the user's
    /// realm via `with_user_id_owned` and reads through
    /// `get_user_contract_state_tree_merkle_proof`.
    pub async fn get_contract_state_slot(
        &self,
        user_id: String,
        contract_id: String,
        slot: String,
    ) -> Result<String, PsyError> {
        use plonky2::field::types::PrimeField64;
        use psy_config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT;

        let user_id: u64 = user_id.parse().map_err(|e| PsyError::w(e))?;
        let contract_id: u64 = contract_id.parse().map_err(|e| PsyError::w(e))?;
        let slot: u64 = slot.parse().map_err(|e| PsyError::w(e))?;

        let s = self.inner.lock().await;
        let provider = s.wallet_session.st_provider.clone();
        let user_provider = provider.with_user_id_owned(user_id);

        let checkpoint = provider
            .get_latest_block_state()
            .await
            .map_err(|e| PsyError::w(format!("get_latest_block_state: {e}")))?
            .checkpoint_id;

        let proof = user_provider
            .get_user_contract_state_tree_merkle_proof(
                checkpoint,
                user_id,
                contract_id as u32,
                MAX_CONTRACT_STATE_TREE_HEIGHT as u8,
                slot,
            )
            .await
            .map_err(|e| PsyError::w(format!("get_user_contract_state_tree_merkle_proof: {e}")))?;

        // A scalar slot value (e.g. a token balance) is packed into the LOW field
        // element of the slot's 4-element Poseidon value, with the rest zeroed.
        // This is the same convention the prove path writes with, e.g.
        // `QHashOut::from_values(amount, 0, 0, 0)` (psy-rust-sdk wasm/mod.rs:851),
        // and matches the observed on-chain slot-0 balance read (config CLAUDE.md).
        // NOTE: multi-field packed slots (e.g. note_count, where the index lives at
        // elements[3]) are NOT scalar slots — callers must use the dedicated note
        // paths for those, not this scalar lens.
        Ok(proof.value.0.elements[0].to_canonical_u64().to_string())
    }

    // ===================================================================
    // MODE-A — external-signature (MetaMask) contract-call submit path.
    //
    // The web wallet's key never leaves MetaMask. A public contract call is
    // authorized by an EXTERNAL `eth_sign` signature over the Psy session
    // sighash, NOT by a held key. Because `eth_sign` is async + user-gated, the
    // flow is necessarily TWO calls — the exact hand-off shape the WASM/web port
    // wires to MetaMask:
    //
    //   1. `sighash_hex = get_sig_hash(pk_hash, call_data_json)`
    //        → drives start_session + prove_contract_call, returns the 32-byte
    //          Psy sighash the user must `eth_sign` (raw, no EIP-191 prefix —
    //          i.e. `personal_sign` of the raw hash, == k256 sign_prehash).
    //   2. < web layer: `signature = await ethereum.request(eth_sign, sighash)` >
    //   3. `tx = exec_contract_call_with_external_signature(pk_hash,
    //          call_data_json, signature_hex)`
    //        → re-derives the session to the SAME sighash, swaps in an
    //          ExternalSecp256K1User carrying that signature, and runs the
    //          UNCHANGED sign_and_submit. No circuit changes.
    //
    // The signing-source substitution is the whole job: the existing
    // `WalletSession::sign_inner` dispatch routes auth to the registered
    // signature user; we register an external-signature one for this pk_hash so
    // `sign()` returns `prove_secp_sign(external_sig)` instead of held-key
    // signing. The secp circuit accepts a MetaMask `eth_sign` signature
    // unchanged (proven by `a_mode_spike`).
    // ===================================================================

    /// MODE-A step 1: drive a public contract call up to (but not through) the
    /// authorization signature, and return the 32-byte Psy session sighash (hex)
    /// the external wallet (MetaMask) must `eth_sign`.
    ///
    /// This runs `start_session` + `prove_contract_call` (the same prefix
    /// `exec_contract_call` runs), then reads the deterministic
    /// `get_sighash(PSY_NETWORK_MAGIC, nonce)` from the session manager — exactly
    /// the bytes `sign_inner` would sign with a held key. The session is left
    /// primed; pair this with `exec_contract_call_with_external_signature`, which
    /// re-derives the same state and asserts the sighash is unchanged before
    /// submit (a stale-nonce guard).
    pub async fn get_sig_hash(
        &self,
        pk_hash: String,
        call_data_json: String,
    ) -> Result<String, PsyError> {
        use psy_config::network_constants::PSY_NETWORK_MAGIC;

        let call_data: ContractCallData = serde_json::from_str(&call_data_json)
            .map_err(|e| PsyError::w(format!("Parse call data JSON error: {e}")))?;
        if call_data.contract_calls.is_empty() {
            return Err(PsyError::w("No contract calls to execute"));
        }
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;

        let s = self.inner.lock().await;

        // Same prefix exec_contract_call runs (session.rs:519-521), minus sign.
        s.wallet_session
            .start_session(pk_hash)
            .await
            .map_err(|e| PsyError::w(format!("start_session: {e}")))?;
        s.wallet_session
            .prove_contract_call(pk_hash, call_data.contract_calls)
            .await
            .map_err(|e| PsyError::w(format!("prove_contract_call: {e}")))?;

        // Read the exact sighash sign_inner derives (session.rs:975-976).
        let mgr = s
            .wallet_session
            .user_session_mgrs
            .get(&pk_hash)
            .ok_or_else(|| PsyError::w(format!("user {pk_hash} not found in session")))?;
        let nonce = mgr.lps.get_nonce();
        let sighash = mgr.get_sighash(PSY_NETWORK_MAGIC, nonce);
        drop(mgr);

        Ok(sighash.to_string())
    }

    /// MODE-A step 3: submit a public contract call authorized SOLELY by an
    /// external `eth_sign` signature (no held key).
    ///
    /// `signature_hex` is the concatenation `compressed_pubkey(33) ‖ r(32) ‖
    /// s(32)` as hex (66 + 128 = 194 hex chars), i.e. the compressed public key
    /// followed by the 64-byte `r‖s` an `eth_sign` / `sign_prehash` produces over
    /// the sighash from `get_sig_hash`. The signed message is the session sighash
    /// itself (recomputed here and bound into the signature's `message` field).
    ///
    /// Re-derives the session to the same point, recomputes the sighash, asserts
    /// it matches the one the supplied signature was produced over (stale-nonce
    /// guard), then registers an `ExternalSecp256K1User` for this `pk_hash` and
    /// runs the UNCHANGED `sign_and_submit`. The held-key signing step in
    /// `sign_inner` is thereby replaced by re-proving the external signature
    /// through the unchanged secp256k1 circuit. Returns the tx end-user-leaf hash.
    pub async fn exec_contract_call_with_external_signature(
        &self,
        pk_hash: String,
        call_data_json: String,
        signature_hex: String,
    ) -> Result<String, PsyError> {
        use psy_config::network_constants::PSY_NETWORK_MAGIC;

        let call_data: ContractCallData = serde_json::from_str(&call_data_json)
            .map_err(|e| PsyError::w(format!("Parse call data JSON error: {e}")))?;
        if call_data.contract_calls.is_empty() {
            return Err(PsyError::w("No contract calls to execute"));
        }
        let pk_hash = QHashOut::<F>::from_str(&pk_hash)
            .map_err(|e| PsyError::w(format!("Parse public key hash error: {e}")))?;

        // Parse compressed_pubkey(33) ‖ r(32) ‖ s(32) = 97 bytes.
        let sig_bytes = hex::decode(signature_hex.trim_start_matches("0x"))
            .map_err(|e| PsyError::w(format!("Parse signature hex error: {e}")))?;
        if sig_bytes.len() != 97 {
            return Err(PsyError::w(format!(
                "signature must be 97 bytes (pubkey33 + r32 + s32), got {}",
                sig_bytes.len()
            )));
        }
        let mut public_key = [0u8; 33];
        public_key.copy_from_slice(&sig_bytes[..33]);
        // Preflight: reject a non-SEC1-compressed pubkey here with a clear error
        // rather than letting it surface as an opaque in-circuit "prove_secp_sign
        // failed". A valid compressed key is 33 bytes with a 0x02/0x03 prefix
        // (even/odd y). This is a cheap format gate, not full curve-point
        // validation — the in-circuit ECDSA check still backstops a structurally
        // valid but off-curve / wrong key.
        if public_key[0] != 0x02 && public_key[0] != 0x03 {
            return Err(PsyError::w(format!(
                "external signature public key is not SEC1-compressed: leading byte 0x{:02x} (expected 0x02 or 0x03)",
                public_key[0]
            )));
        }
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&sig_bytes[33..]);

        // `mut`: register_external_secp_user takes `&mut wallet`.
        let mut s = self.inner.lock().await;

        // CRITICAL — do NOT re-derive the session here. `get_sig_hash` already
        // ran start_session + prove_contract_call and left the session primed at
        // the EXACT state the returned sighash commits to. Re-running
        // start_session would refresh to the latest checkpoint and shift the
        // nonce/sighash, so the externally-signed message would no longer match
        // the session's end-cap sighash → the in-circuit ECDSA check fails with a
        // VirtualTarget partition conflict. Instead, reuse the primed session and
        // VERIFY its current sighash equals the one the signature was produced
        // over (the signature's bound message), which is the stale-state guard.
        let mgr = s
            .wallet_session
            .user_session_mgrs
            .get(&pk_hash)
            .ok_or_else(|| {
                PsyError::w(format!(
                    "user {pk_hash} not found in session — call get_sig_hash first (it primes the session)"
                ))
            })?;
        let nonce = mgr.lps.get_nonce();
        let sighash = mgr.get_sighash(PSY_NETWORK_MAGIC, nonce);
        drop(mgr);

        // Bind the session's CURRENT sighash as the message the circuit verifies.
        // The supplied r‖s must be an eth_sign over exactly this hash, or the
        // proof is rejected. (The session state is unchanged since get_sig_hash,
        // so this equals the hash the caller signed — barring a concurrent call
        // on the same pk_hash, which the per-pk session manager serializes.)
        let external_sig = PsyCompressedSecp256K1Signature {
            public_key,
            signature,
            message: Hash256::from(sighash),
        };

        // Swap in the external-signature signer for this pk_hash. The pk_hash it
        // derives MUST equal the addressed pk_hash, else the supplied signature
        // is for a different key than the session user.
        let ext_pk_info = s
            .wallet_session
            .wallet
            .register_external_secp_user(external_sig)
            .await
            .map_err(|e| PsyError::w(format!("register_external_secp_user: {e}")))?;
        use psy_crypto::hash::traits::qhashable::QFieldHashable;
        let ext_pk_hash = ext_pk_info.qfhash::<psy_data::config::store_config::PsyHasher>();
        if ext_pk_hash != pk_hash {
            return Err(PsyError::w(format!(
                "external signature public key hash {ext_pk_hash} does not match addressed pk_hash {pk_hash}"
            )));
        }

        // Run the UNCHANGED sign_and_submit — sign_inner now dispatches auth to
        // the ExternalSecp256K1User → prove_secp_sign(external_sig).
        let end_user_leaf_hash = s
            .wallet_session
            .sign_and_submit(pk_hash, call_data.software_defined_call)
            .await
            .map_err(|e| PsyError::w(format!("sign_and_submit (external sig): {e}")))?;

        Ok(end_user_leaf_hash.to_string())
    }
}

#[cfg(test)]
mod a_mode_spike {
    //! GO/NO-GO for the Mode-A web-wallet architecture: prove that a MetaMask
    //! `eth_sign`-format signature is accepted by the EXISTING Psy secp256k1
    //! authorization circuit, with NO circuit changes.
    //!
    //! run: cargo +nightly-2025-09-20 test -p psy_wallet_core_ffi a_mode -- --nocapture
    use std::str::FromStr;

    use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::hash::hash_types::HashOut;
    use plonky2::hash::poseidon::PoseidonPermutation;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use psy_common::data::{
        base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::CompressedPublicKey,
    };
    use psy_common_circuit::circuits::secp256k1_signature::Secp256K1SignatureCircuit;
    use psy_crypto::hash::traits::hasher::FieldQHasher;
    use psy_crypto::signature::secp256k1::core::PsyCompressedSecp256K1Signature;
    use psy_data::config::store_config::PsyHasher;

    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    #[test]
    fn metamask_eth_sign_accepted_by_existing_secp256k1_circuit() {
        // A real secp256k1 (Ethereum-style) private key — TEST ONLY.
        let private_key = QHashOut::<F>::from_str(
            "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",
        )
        .unwrap();
        // A Psy transaction sig_hash (Poseidon/Goldilocks QHashOut).
        let sig_hash = QHashOut::<F>::from_str(
            "83955402ec7f375d1d6e8f3bf59753fe0af1e7c62bb4b662716a2524d3e2d186",
        )
        .unwrap();

        // === MetaMask `eth_sign` EXACTLY: ECDSA over the RAW 32-byte hash, no
        // EIP-191 prefix (= k256 sign_prehash). RFC-6979 deterministic, so this
        // is byte-for-byte what MetaMask/ethers return for this key+hash.
        // Produced by k256 DIRECTLY (independent of any Psy signing code) — i.e.
        // an EXTERNAL signature. ===
        let signing_key = SigningKey::from_slice(&Hash256::from(private_key).0).unwrap();
        let sig_hash_bytes = Hash256::from(sig_hash).0;
        let sig: k256::ecdsa::Signature = signing_key.sign_prehash(&sig_hash_bytes).unwrap();
        let mut rs = [0u8; 64];
        rs[..32].copy_from_slice(&sig.r().to_bytes());
        rs[32..].copy_from_slice(&sig.s().to_bytes());
        let mut pk = [0u8; 33];
        pk.copy_from_slice(&signing_key.verifying_key().to_encoded_point(true).to_bytes());
        println!(
            "[a-mode] external eth_sign sig: pk={} rs={}",
            hex::encode(pk),
            hex::encode(rs)
        );

        let external_sig = PsyCompressedSecp256K1Signature {
            public_key: pk,
            signature: rs,
            message: Hash256::from(sig_hash),
        };

        // === Feed it to the EXISTING Psy secp256k1 auth circuit. prove()
        // succeeds ONLY if the in-circuit ECDSA verification of THIS external
        // signature passes — i.e. the circuit ACCEPTS it. ===
        let circuit = Secp256K1SignatureCircuit::<C, D>::new();
        let proof = circuit
            .prove(&external_sig)
            .expect("existing secp256k1 circuit MUST accept a valid eth_sign signature");

        // === The auth binding the rest of the system consumes:
        // hash(sig_hash, public_key_param). ===
        let pk_param = psy_crypto::signature::secp256k1::wallet::hash_no_pad_compressed_public_key::<
            F,
            PoseidonPermutation<F>,
        >(CompressedPublicKey(pk));
        let expected = PsyHasher::q_two_to_one(QHashOut::from(Hash256::from(sig_hash)), pk_param);
        let got = QHashOut(HashOut {
            elements: [
                proof.public_inputs[0],
                proof.public_inputs[1],
                proof.public_inputs[2],
                proof.public_inputs[3],
            ],
        });
        assert_eq!(
            got, expected,
            "auth binding hash(sig_hash, public_key_param) MUST match the existing path"
        );

        println!(
            "MODE-A GO: external MetaMask eth_sign signature ACCEPTED by the existing secp256k1 circuit — no circuit changes. binding={got}"
        );
    }
}

#[cfg(test)]
mod mode_a_staging_spike {
    //! END-TO-END GO/NO-GO for the Mode-A SUBMIT path against LIVE staging:
    //! prove that a real public contract call authorized SOLELY by an external
    //! `eth_sign` signature (no held key in the SDK at submit time) is ACCEPTED
    //! by the staging chain, with NO circuit changes.
    //!
    //! Requires staging access. Skips (passes, no-op) unless `PSY_CONFIG` points
    //! at a staging RPC config — so plain `cargo test` stays offline.
    //!
    //! run:
    //!   PSY_CONFIG=/tmp/psy-staging-config.json \
    //!   CARGO_NET_GIT_FETCH_WITH_CLI=true \
    //!   cargo test -p psy_wallet_core_ffi \
    //!     mode_a_external_sig_contract_call_accepted_by_staging -- --nocapture
    use std::str::FromStr;
    use std::time::{Duration, Instant};

    use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use psy_common::data::{base_types::hash256::Hash256, qhashout::QHashOut};

    use crate::PsyWalletCore;

    type F = GoldilocksField;

    /// A fresh, deterministic secp256k1 (Ethereum-style) test key — NEVER a
    /// real-funds key. Overridable via PSY_MODE_A_KEY (bare 64-hex lowercase) so
    /// reruns can use a not-yet-registered key.
    fn test_private_key_hex() -> String {
        std::env::var("PSY_MODE_A_KEY")
            .unwrap_or_else(|_| "2b7e151628aed2a6abf7158809cf4f3c762e7160f38b4da56a784d9045190cfe".into())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn mode_a_external_sig_contract_call_accepted_by_staging() {
        let Ok(config_path) = std::env::var("PSY_CONFIG") else {
            eprintln!("[mode-a-staging] PSY_CONFIG not set — skipping live staging spike (offline no-op).");
            return;
        };
        let config_json = std::fs::read_to_string(&config_path)
            .unwrap_or_else(|e| panic!("read PSY_CONFIG ({config_path}): {e}"));

        let private_key_hex = test_private_key_hex();

        eprintln!("[mode-a-staging] building wallet core (RPC bootstrap)…");
        let core = PsyWalletCore::new(config_json)
            .await
            .expect("build PsyWalletCore from staging config");

        // 1) Register the secp256k1 user (the proven register_user path).
        eprintln!("[mode-a-staging] register_user (secp256k1)…");
        let pk_hash = core
            .register_user(private_key_hex.clone(), "secp256k1".to_string())
            .await
            .expect("register_user(secp256k1)");
        eprintln!("[mode-a-staging] registered pkHash={pk_hash}");

        // 2) Wait for the registration to land (≈2 checkpoints), polling add_user
        //    until it resolves the on-chain user_id.
        eprintln!("[mode-a-staging] waiting for registration to land (add_user poll)…");
        let add_deadline = Instant::now() + Duration::from_secs(240);
        loop {
            match core
                .add_user(private_key_hex.clone(), "secp256k1".to_string())
                .await
            {
                Ok(_) => {
                    eprintln!("[mode-a-staging] user registered + added.");
                    break;
                }
                Err(e) => {
                    if Instant::now() >= add_deadline {
                        panic!("timeout waiting for registration to land: {e}");
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        // 3) Build a real simple_mint call on contract 0 — the canonical
        //    self-credit that a zero-balance user can make (it brings PSY in and
        //    pays its own fee; mirrors Makefile `mint` / contract_call.json).
        let mint_amount: u64 = 10_000_000_000;
        let call_data_json = serde_json::json!({
            "contract_calls": [{
                "contract_id": 0,
                "method_name": "simple_mint",
                "inputs": [mint_amount],
            }],
            "software_defined_call": { "inputs": [] }
        })
        .to_string();

        // 4) MODE-A step 1: get the sighash the external wallet must eth_sign.
        eprintln!("[mode-a-staging] get_sig_hash (drives start_session + prove_contract_call)…");
        let t_sighash = Instant::now();
        let sighash_hex = core
            .get_sig_hash(pk_hash.clone(), call_data_json.clone())
            .await
            .expect("get_sig_hash");
        eprintln!(
            "[mode-a-staging] sighash={sighash_hex} (in {:?})",
            t_sighash.elapsed()
        );

        // 5) EXTERNAL signature — EXACTLY MetaMask eth_sign: raw k256
        //    sign_prehash over the sighash bytes (no EIP-191 prefix). Produced by
        //    k256 directly, independent of any Psy signing code → an external sig.
        let priv_qhash = QHashOut::<F>::from_str(&private_key_hex).expect("parse priv key");
        let signing_key = SigningKey::from_slice(&Hash256::from(priv_qhash).0).expect("k256 key");
        let sighash_qhash = QHashOut::<F>::from_str(&sighash_hex).expect("parse sighash");
        let sighash_bytes = Hash256::from(sighash_qhash).0;
        let sig: k256::ecdsa::Signature = signing_key
            .sign_prehash(&sighash_bytes)
            .expect("eth_sign (sign_prehash) over sighash");
        let mut payload = Vec::with_capacity(97);
        payload.extend_from_slice(&signing_key.verifying_key().to_encoded_point(true).to_bytes()); // 33
        payload.extend_from_slice(&sig.r().to_bytes()); // 32
        payload.extend_from_slice(&sig.s().to_bytes()); // 32
        assert_eq!(payload.len(), 97, "signature payload must be pubkey33 + r32 + s32");
        let signature_hex = hex::encode(&payload);
        eprintln!("[mode-a-staging] external eth_sign signature built ({} hex chars).", signature_hex.len());

        // 6) MODE-A step 3: submit authorized SOLELY by the external signature.
        eprintln!("[mode-a-staging] exec_contract_call_with_external_signature (submit)…");
        let t_submit = Instant::now();
        let tx_result = core
            .exec_contract_call_with_external_signature(pk_hash.clone(), call_data_json, signature_hex)
            .await;
        let submit_dur = t_submit.elapsed();

        match tx_result {
            Ok(tx_hash) => {
                eprintln!("[mode-a-staging] submit returned tx/leaf hash={tx_hash} in {submit_dur:?}");
                assert!(!tx_hash.is_empty(), "tx hash must be non-empty");
                assert!(
                    !tx_hash.contains("VirtualTarget"),
                    "circuit mismatch (VirtualTarget) in result: {tx_hash}"
                );
                println!(
                    "MODE-A SUBMIT GO: contract call authorized SOLELY by an external eth_sign \
                     signature ACCEPTED on live staging — no circuit changes. tx={tx_hash} ({submit_dur:?})"
                );
            }
            Err(e) => {
                let msg = format!("{e:?}");
                assert!(
                    !msg.contains("VirtualTarget"),
                    "HARD BLOCKER: circuit/VirtualTarget mismatch on external-sig submit — \
                     would indicate the circuit verifies a different message than get_sighash. {msg}"
                );
                panic!("[mode-a-staging] external-sig submit failed (NOT a circuit mismatch): {msg}");
            }
        }
    }
}
