# STRATEGY BRIEF — psy-wallet-core-ffi (Mobile Core Parity)

_Repositioning / strategic realignment, synthesized from the 8-role PsyVerse Flow strategy pass + the execution outcome. Branch: `feat/mobile-core-parity`. Date: 2026-06-23._

---

## Executive Summary

**What this is.** `psy-wallet-core-ffi` is a uniffi-exported FFI wrapper over the *exact same* `psy_prover::session::WalletSession` the shipping browser-extension wallet runs. iOS (SwiftUI) and Android (Jetpack Compose) become thin native skins over one shared Rust ZK core — not three divergent reimplementations. The wedge is specific and nearly uncontested: **hardware-custodied ZK key (Secure Enclave / Keystore) + native on-device Plonky2 proving + browser-extension feature parity, on one core.** The only true neighbors are Zcash mobile wallets; everyone else either doesn't do private payments or offloads proving to a server (reintroducing the metadata leak).

**What changed this cycle (the strategic update).** The method-surface gap is now **closed at the compiler level**. The execution chain added the 7 remaining wallet-critical methods as faithful 1:1 ports of their `psy-rust-sdk/src/wasm/mod.rs` twins. The crate now exports **15 methods** and `cargo check -p psy_wallet_core_ffi` is **green with zero warnings** (also green under `--tests`; the `a_mode` test passes — external MetaMask `eth_sign` accepted by the secp256k1 circuit with zero circuit changes).

Exported surface (15):
`new`, `register_user`, `add_user`, `get_zk_public_key_json`, `exec_contract_call_json`, `sign_and_submit`, `exec_claim_with_external_proof_json`, `prove_private_note_inclusion_json`, `get_random_keypair_json`, `prove_contract_call_json`, `prove_contract_calls_json`, `start_session`, `deploy_contract_json`, `add_external_proof_json`, `get_contract_state_slot` (net-new scalar-slot read; documented `elements[0]` semantics).

Intentionally skipped (host-trivial / out of objective, documented in parity matrix): `get_deploy_contract_cmd_json`, `ping`, `get_result`.

**The strategic consequence.** With the surface compiler-complete, the program's center of gravity **moves off "port methods" and onto the one unknown `cargo check` cannot see: on-device proving cost.** The riskiest *compatibility* bet already cleared (`register_user` proved + accepted on live staging, real `pkHash`, no `VirtualTarget`, ~5.7s steady-state). The riskiest *feasibility* bet — peak RSS and wall-time on a real mid-tier phone — has **zero device evidence**. Every public claim of the wedge ("your phone proves it") is gated on that single measurement.

---

## 1. Recommended Strategic Adjustment

**Pivot the program from "complete the method surface" (DONE) to "de-risk on-device proving before any UI is built."**

The 8 strategy roles converge on one realignment, and the execution outcome makes it actionable:

1. **The port phase is over; stop treating it as the work.** Leverage, Conviction, and Mission all framed "finish the surface" as the multiplier. That multiplier has been collected — 15 methods, green, faithful. Continuing to add marginal methods (the 3 skipped are host-trivial) is low-leverage. The surface is now a *contract* for the UI teams, not an open work item.

2. **The make-or-break has shifted from compatibility to physics.** Positioning, Risk, and Narrative independently flag the same thing: the wedge ("native on-device proving, key never leaves the enclave") is **silently invalidated** if Plonky2's working set OOM-kills a foreground app or proving takes 20-40s on a 2-4x-slower mobile CPU. We have only an M-series Mac datapoint. iOS jetsam will SIGKILL on RAM spikes; Android low-RAM devices OOM mid-proof; backgrounding freezes a 30s proof mid-submit. **A wallet that crashes while "sending money" is dead on arrival.** This must be measured on a real device *before* SwiftUI/Compose work forks, because the result dictates architecture (on-device-only vs. delegated-proving fallback via the existing prove-proxy:9999) and the public message itself.

3. **Resolve the key-custody honesty gap in parallel (it is co-blocking).** Every method takes `private_key_str: String`. Across uniffi that key lands in a non-zeroized, pageable Swift/Kotlin `String` — which contradicts the "key lives in Secure Enclave" promise. Decide the trust boundary explicitly now: either (a) the enclave holds a non-exportable key and signing happens via a host-side callback the core invokes (the spiked secp256k1/`eth_sign` external-signature path is the exact template), or (b) the ZK key is software-held and the enclave only encrypts-the-blob-at-rest — and say so honestly. The UI teams must not build assuming (a) while the FFI only supports (b).

4. **Lock the parity contract so two UI tracks can't drift or stall on this crate.** Promote the parity matrix (wasm method → FFI method → status) into the README as the binding contract, and grow `proving_spike.rs` into a staging smoke gate that runs the full first-run sequence (`new → get_random_keypair → register_user → exec_contract_call`) on every chain bump. "Compiles green" proves nothing about circuit acceptance; "accepted by staging" does. Pin/assert circuit fingerprints at `new()` so a chain redeploy fails fast with "app update required" instead of silently bricking installed binaries.

**What to explicitly NOT do now:** do not write public "your phone proves it" copy; do not add `is_sync`/wasm-timing parity work (no mobile multiplier); do not refactor the SDK "while we're here" (fidelity is the whole value); do not target mainstream consumers (on-device proving UX isn't consumer-ready). First users, in order: existing Psy web-wallet users wanting a phone client (also the beta proving-testers), then Zcash-mobile privacy maximalists, then downstream Psy app developers buying the uniffi bindings.

---

## 2. The Single Highest-Impact Next Move

**Cross-compile `proving_spike.rs` to a real mid-tier Android device and an older iPhone, and record peak RSS + wall-clock for `register → send → claim`. One number on a real phone is worth more than the rest of this brief.**

- **Why this and nothing else first:** it is the only risk with *zero* evidence, it is invisible in the green build, and it is *existential* to the entire premise. If the spike passes on a mid-tier device, the wedge is real and nearly uncontested — green-light the UIs and the "your phone proves it" message. If it fails, the architecture must include delegated proving (prove-proxy fallback) *before* two UIs are built on a false assumption — a far cheaper correction now than after.
- **It is a provisioning problem, not a code problem.** `build-ios.sh` / `build-android.sh` already exist; the blocker is "no Xcode/NDK in this environment." That is an external constraint to hand off, not an engineering unknown.
- **Acceptance criteria for the spike:** wall-time and peak RSS for `register_user`, one public `exec_contract_call_json`, and one `exec_claim_with_external_proof_json`, on (a) a ~6GB Android device and (b) an older iPhone. Set a hard memory budget (e.g. foreground-app-safe; <~120MB if a Safari/keyboard extension is ever in scope). Report `{circuit, wall_ms, peak_rss_bytes}` per action.
- **Bundle the cheap co-blocker decision:** in the same increment, make a written call on key custody (enclave-callback signing vs. software-held + encrypted-at-rest) so the UI teams build against the real trust boundary.

Everything else (durable tx-queue / crash-survivable proving intent, read-only methods off the proving mutex, panic→`catch_unwind`→`PsyError`, App/Play store positioning review, CI image for the bus-factor-1 toolchain) is real and queued — but it is all downstream of knowing whether the phone can prove at all.

---

## 3. Consolidated Brief (the one-page contract)

**Thesis (one sentence):** One Rust core, three wallets — the same `WalletSession` the extension runs, exported via uniffi so iOS + Android share one ZK-proving, key-holding, signing engine; every method finished ships two wallets at once.

**State of play (verified against source, 2026-06-23):**
- 15 uniffi methods exported; faithful ports differing from wasm twins only by the 3 sanctioned transforms (lock-guard for `self`, `PsyError::w` for `JsError`, native timers). `get_contract_state_slot` is the one net-new method (scalar-slot read, `elements[0]`, documented).
- `cargo check -p psy_wallet_core_ffi` green, zero warnings; `--tests` green; `a_mode` test passes (external `eth_sign` accepted, zero circuit changes).
- Chain-source compat confirmed on live staging (`register_user` proved + accepted, real `pkHash`, ~5.7s steady-state — server-side).
- Changes confined to `psy-wallet-core-ffi/`; committed at `76984e31` on `feat/mobile-core-parity`; no push. Pre-existing dirty files (Cargo.lock/toml, config.json, two `*.d.ts`, pnpm-lock, rust-toolchain) left untouched per safety rules.

**Risk register (ranked):**
1. **[EXISTENTIAL] On-device proving time/memory — zero device evidence.** Defuse via the spike (Move #2). Add crash-survivable proving (persist intent before / submitted after) and a delegated-proving fallback policy.
2. **[HIGH] Secret key crosses FFI as plain `String`** — contradicts the enclave promise. Decide enclave-callback vs. software-held; minimally take keys as zeroizable bytes, never log.
3. **[HIGH] Circuit/chain-commit coupling** (the `VirtualTarget` failure class). Assert fingerprints at `new()`; CI gate that rebuilds + proves against the staging chain commit on every bump; treat chain commit as a release coordinate.
4. **[MED-HIGH] App/Play store + regulatory.** Position as self-custody key-management + signing tool, never money transmitter; keep "shielded address" discipline; pre-read store guidelines; have a sideload/TestFlight contingency.
5. **[MED] uniffi async + tokio on a UI-owned main thread.** `catch_unwind`→`PsyError` so a prover panic isn't an app abort; split read-only calls off the single proving `Mutex` so the UI stays responsive during a proof.
6. **[MED] Parity contract not yet binding / [MED] bus-factor-1 toolchain.** Put the parity matrix in the README; capture the build incantation in a reproducible CI image with the cross-compile targets.

**Demo that makes people get it (≤90s, once the spike passes):** split screen — web (MetaMask, per-tx `eth_sign`) vs. native (own key, `signType 'zk'`); kick off a private send on the phone, narrate "the proof is generated *here*, nothing has left the device"; submit; show the explorer settling the transfer with no amounts and no sender↔receiver link. The single image, if only one: phone mid-private-send, "Your payment is being proven on this phone. It hasn't touched a server."

**Don't claim:** a phone proving number (no device data yet — say "desktop baseline ~5.7s, on-device numbers landing next"); "shipping mobile wallet" (the core is ready; the UIs + enclave storage are the per-platform layers still to build); L1/L2 or "bridge" (independent ZK chain; "Deposit"/"Withdraw"; show the Psy ID / shielded address).

**Inner-loop DX note (cheap, optional):** the mandated `cargo check` is ~13s even on a no-op because two external build scripts flap (`psy_config`'s missing `genesis_contracts.json` rerun-if-changed, and gnark's bindgen header mtime bump). Creating that empty stamp file + an env-pinned `check.sh` wrapper collapses the loop to ~1-2s without editing any forbidden tree. Not load-bearing for strategy; quality-of-life for the next increment.

---

_Bottom line: the code is a clean, faithful, green port — that part is genuinely low-risk and now compiler-complete. The remaining risk is entirely in what `cargo check` cannot see: this has never run on a phone, and the key-custody story doesn't yet match the enclave promise. Spend the next increment on a real-device proving + memory measurement and the enclave-callback signing decision. Those two are the only things that can quietly invalidate the entire mobile thesis after two UIs are built on top of it._
