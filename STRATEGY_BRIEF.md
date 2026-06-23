# STRATEGY BRIEF — Web Mode-A (per-tx MetaMask `eth_sign` → proven in-circuit)

_Repositioning / strategic realignment, synthesized from the 8-role PsyVerse Flow strategy pass + the execution outcome. Branch: `feat/mode-a-external-sig`. Last increment: `fa5778c8` (WASM external-sig methods). Status: SHIPPED, build GREEN, not pushed. Date: 2026-06-23._

_Sibling brief: `STRATEGY_BRIEF_MOBILE_CORE_PARITY.md` (the iOS/Android core-parity run — separate product, software-held keys, NOT MetaMask). This brief is the Web wallet. Both ride the same `psy_wallet_core_ffi`._

This document is the post-increment realignment. It synthesizes the 8 strategy sections (Mission, Positioning, Conviction, FlowState/DX, Innovation, Leverage, Narrative, Risk) and the execution outcome into: (1) what changed and the recommended strategic adjustment, (2) the single highest-impact next move, (3) the consolidated brief for the next agent.

---

## 0. WHAT WE NOW KNOW (the increment that just landed)

The two Mode-A external-signature primitives are now **callable from JavaScript, by name, with correct types**, in the exact bundle the bridge imports:

- `get_sig_hash(pk_hash, call_data_json) -> Promise<string>` — runs `start_session` + `prove_contract_call`, returns the 32-byte sighash hex MetaMask must `eth_sign`, leaves the session primed.
- `exec_contract_call_with_external_signature(pk_hash, call_data_json, signature_hex) -> Promise<string>` — parses the 97-byte `pubkey(33)‖r(32)‖s(32)`, runs the SEC1 `0x02/0x03` preflight, reuses the primed session (does NOT re-run `start_session`), rebinds the sighash as a stale-nonce guard, registers the external secp user, asserts pk_hash match, then runs the UNCHANGED `sign_and_submit`.

Confirmed reachable: `psy_prover.d.ts` exposes both as typed methods (lines 117/134) and as wasm exports (lines 204/208); `psy_prover_bg.wasm` (7,313,834 bytes) and `wasm-binary.ts` carry both export symbols. `cargo check --target wasm32-unknown-unknown -p psy_rust_sdk` is GREEN (warnings only, all pre-existing). Diff confined to `wasm/mod.rs` + the two generated `.d.ts` typings — **zero prover/circuit/FFI change**.

**The single most strategically important learning:** the feared wasm32 hard blocker did NOT materialize. `register_external_secp_user` / `prove_secp_sign` compile and link for `wasm32-unknown-unknown`. The Risk section's #1 "most-likely-to-kill" scenario is now retired *in the actual web target*, not just native. That removes the last architectural unknown standing between Route A and a shippable MetaMask-native web wallet.

---

## 1. RECOMMENDED STRATEGIC ADJUSTMENT

The strategy does not pivot. It **graduates**. Three concrete adjustments, in priority order:

**A. Move from "prove the capability exists" to "prove the capability works end-to-end in a browser."** Every section before this increment hedged on "staging-verified at the FFI/native layer." That hedge is now smaller but not gone: the methods *compile and export* to wasm32, but the full `get_sig_hash → eth_sign → exec_with_external_sig` round-trip has not been executed *in a browser against staging*. The risk center of gravity has shifted from "does the circuit accept eth_sign" (retired) to "does in-browser proving complete without hanging/OOM, and does the two-call session contract survive a real MetaMask popup delay." Reorient all remaining effort toward closing that specific gap.

**B. Lock the positioning wedge into the code surface NOW, while it costs nothing.** The Innovation section's highest-leverage, zero-cost move: the WASM method is already signer-neutral (`exec_contract_call_with_external_signature`, not `..._with_metamask`). KEEP IT THAT WAY. This is not cosmetic — it is the architectural position ("authorization is a swappable in-circuit ECDSA verifier, not a hardwired key scheme") that lets the same proven path later serve Ledger/Trezor, MPC custody, and secp256k1 passkeys with pure JS adapters. Do not let the UI-wiring increment rename or narrow the input shape to MetaMask. The wedge is "Bring your secp256k1 signer; keep your keys" — MetaMask is just the first instance.

**C. Treat the two-call session contract as the load-bearing fragility, and freeze it as a golden vector.** The whole design rests on a byte-exact invariant: `get_sig_hash` output == the bytes the wallet signs, the session must not advance between the two calls, and `signature_hex = pubkey(33)‖r(32)‖s(32) = 97 bytes`. A WASM-vs-FFI-vs-JS mismatch surfaces only as an opaque in-circuit `VirtualTarget set twice` after tens of seconds of proving — undebuggable downstream. Adjust the plan to ship a committed golden fixture BEFORE the UI wiring, so any mismatch is a 10ms assertion, not a 10-minute proving failure. This is the cheapest insurance against the bug most likely to eat the next agent.

What we explicitly do NOT change: no circuit work, no prover edits, no FFI changes, no second custody model bleeding in from the mobile (software-held-key) track. Web = key never leaves MetaMask; that is non-negotiable and is now structurally true in shipped code.

---

## 2. THE SINGLE HIGHEST-IMPACT NEXT MOVE

**Wire the bridge MetaMask round-trip for ONE flow — Send — end-to-end against staging, gated behind a committed golden interop vector.**

Concretely, the next increment is: web UI calls `get_sig_hash` → `window.ethereum.request({ method: 'personal_sign' | 'eth_sign' })` over the returned sighash → `exec_contract_call_with_external_signature` with the assembled `pubkey‖r‖s`. Prove the full Connect → Login → Send path lands a confirmed shielded transfer with the key never leaving MetaMask.

Why this and not the full 7-step bridge (Connect→Login→Balance→Faucet→Claim→Send→Activity): Send is the load-bearing hero moment (the Narrative section's "MetaMask popup over a private payment" screenshot), it exercises every part of the two-call contract, and proving it once de-risks all the other flows that reuse the same primitive. Faucet/Claim/Activity are reads or repetitions of the same authorization path.

Two preconditions to bake into that increment, derived from the Risk + FlowState sections:
1. **Golden vector first** (Adjustment C above) — commit a `(pk_hash, call_data_json) → sighash_hex → test-key signature → expected-accept` fixture, asserted identically against the FFI crate, the WASM build, and the new JS adapter. This is the conformance test that catches the sighash mismatch before it becomes a 490s mystery.
2. **In-browser proving must run off the main thread** — secp + contract-call proving is tens of seconds and risks the wasm 32-bit 4GB ceiling. Run it in a worker/offscreen context (mirror the wallet's offscreen-prover, MEMORY tx_metadata #94). A typed, pre-prove stale-nonce error (re-call `get_sig_hash`) handles the per-tx-popup race that is Mode-A's *normal* failure mode, not an edge case.

That increment turns "a typed surface exists" into "a person can make a private Psy payment from their MetaMask" — the actual distribution point of the entire effort.

---

## 3. CONSOLIDATED BRIEF FOR THE NEXT AGENT

**Mission of the next run:** Wire the Send flow end-to-end (`get_sig_hash → eth_sign → exec_contract_call_with_external_signature`) in the bridge web app against staging, key never leaving MetaMask, behind a committed golden interop vector. UI increment — no circuit, no prover, no FFI changes.

**What you already have (callable today):**
- WASM SDK methods in `psy-ts-sdk/packages/psy-sdk/src/local-web-prover/` — `get_sig_hash(pkHash, callDataJson): Promise<string>` and `exec_contract_call_with_external_signature(pkHash, callDataJson, signatureHex): Promise<string>`, typed in `psy_prover.d.ts` (lines 117/134), exported in `psy_prover_bg.wasm` / `wasm-binary.ts`.
- Reference semantics (do not modify): `psy-wallet-core-ffi/src/lib.rs` — `get_sig_hash` @1016, `exec_contract_call_with_external_signature` @1071, SEC1 preflight @1099, nonce-binding invariant @1119–1140, staging spike `a_mode_spike` @~1401 (proves the circuit ACCEPTS a raw `eth_sign` signature).

**Hard invariants — carry verbatim, never "clean up":**
- `signature_hex` MUST be `pubkey(33, leading 0x02/0x03) ‖ r(32) ‖ s(32)` = 97 bytes = 194 hex chars.
- The signed message is the sighash returned by `get_sig_hash`; do NOT let the session advance between the two calls; the call_data must be identical across both calls.
- `exec_...` must NOT re-run `start_session` (re-deriving shifts the nonce → in-circuit ECDSA fails as `VirtualTarget` partition conflict — CLAUDE.md known-issue #1).
- Keep the method signer-neutral. Never rename to MetaMask; never narrow the input to a wallet-specific shape.

**Build / verify discipline (FlowState):**
- Inner loop for any Rust touch: `CARGO_NET_GIT_FETCH_WITH_CLI=true cargo check --target wasm32-unknown-unknown -p psy_rust_sdk` (~1.4s). Never alternate cargo targets/profiles (drops you to 50s). nightly-2025-09-20, gnark patched locally.
- `build-wasm-binary.ts` footgun: `ensureWasmArtifacts()` returns early if `psy_prover_bg.wasm` exists — `rm` it (and the nodejs copy) before rebuild, or you ship a stale binary that reports success.
- The `.wasm`/`.js`/`wasm-binary.ts` are gitignored in `local-web-prover/`; only the `.d.ts` typings are tracked. "Binary not committed" is by design, not a regression.

**Hard-stop signal:** if any step appears to need a `psy_prover`/circuit change, STOP and report. Route A is SDK + UI layer only. Everything in the repo says you won't need one — the spike passed and the wasm32 link held.

**Dominant risk for the next run (own it explicitly):** in-browser proving latency/OOM and the per-tx stale-signature race. Put proving in a worker; surface a typed "re-sign" error before the prove. The golden vector defuses the silent sighash-mismatch class.

**Positioning copy guardrails (Narrative):** "shielded address"/"shielded payment", never "anonymous"; Psy is an independent chain, never L1/L2/rollup — MetaMask is signer/identity only; show the Psy ID as user-facing identity; "Deposit"/"Withdraw", never "bridge" as a verb. The narrative landmine: be crisp that MetaMask only signs a hash and never sees transfer details — blur this and the privacy audience reads it as theater.

**Repo state of record:** branch `feat/mode-a-external-sig` at `fa5778c8`; working tree carries only pre-existing unrelated noise (`Cargo.toml`, `Cargo.lock`, `config.json`, `rust-toolchain.toml`, `pnpm-lock.yaml`) — leave it. Not pushed; never push or touch deploy branches without an explicit ask.
