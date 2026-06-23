# STRATEGY BRIEF — gnark Mobile-FFI Gate (native arm64 cross-compile UNBLOCKED)

_Repositioning / strategic realignment, synthesized from the 8-role PsyVerse Flow strategy pass + the execution outcome. Working branch: `feat/mode-a-external-sig` (psy-sdk-fresh). The 4 gate edits + the gate commit physically live in the sibling repo `parth-generic-v1` (branch `feat/bridge-app-unified`, commit `9f9c9135`), reached from psy-sdk-fresh via relative path deps. Date: 2026-06-24._

_Sibling briefs (do NOT clobber): `STRATEGY_BRIEF_MOBILE_CORE_PARITY.md` (the native iOS/Android core-parity run — the 15-method uniffi surface this gate now lets cross-compile) and the prior Web Mode-A brief content (MetaMask per-tx `eth_sign`, separate product). All three ride the same `psy_wallet_core_ffi` core. This brief is the **mobile-core enabling layer**: the dependency-graph gate that turns "the surface is compiler-complete on host" into "the surface links for a phone."_

This document is the post-increment realignment for the gnark gate. It synthesizes the 8 strategy sections (Mission, Positioning, Conviction, FlowState/DX, Innovation, Leverage, Narrative, Risk) and the execution outcome into: (1) what changed and the recommended strategic adjustment, (2) the single highest-impact next move, (3) the consolidated brief for the next agent.

---

## 0. WHAT WE NOW KNOW (the increment that just landed)

**The native mobile core now cross-compiles. The hard blocker is gone — and it was exactly the dependency the wallet never called.**

Before this increment, `cargo build -p psy_wallet_core_ffi --target aarch64-*` failed *before any Plonky2 code ran*: `psy_wallet_core_ffi → psy_prover → psy_plonky2_circuits` hard-linked `gnark-plonky2-wrapper`, a Rust→Go/cgo lib whose `build.rs` compiles a Go archive **host-only**. gnark is the node/prove-proxy L1 Groth16 *wrap*; the wallet's real flow (the ~5.7s staging proof) is pure Plonky2 and never invokes it.

The gate feature-gated gnark behind `gnark-wrap`, kept it in the `default` set of both `psy_plonky2_circuits` and `psy_prover` (so the node stays byte-identical), and let the FFI — which already declared `psy_prover = { default-features = false }` — fall out gnark-free. Four files, zero circuit-logic change, uncommitted-then-committed exactly per the repo-safety contract.

**Verified acceptance gates (all green):**
- **FFI gnark-free:** `cargo tree -p psy_wallet_core_ffi -i gnark-plonky2-wrapper` → not in graph; targeted FFI build reports the gnark patch "not used in the crate graph."
- **Node byte-identical:** node-context resolution shows `psy_prover` = `default,gnark-wrap` and `psy_plonky2_circuits` = `default,default-no-gnark,gnark-wrap,serialize_speedy,std` — the original feature set exactly; gnark still reaches both via `default`.
- **Headline gate — real Android arm64-v8a build:** `cargo ndk -t arm64-v8a build --release -p psy_wallet_core_ffi` → **Finished in 2m19s**. Artifacts: `libpsy_wallet_core_ffi.so` (20MB) + `.a` (136MB), **0 gnark/cgo/Go symbols, 0 gnark strings**. No further host-only blocker (ring/secp256k1-sys/blst) surfaced.

**The single most strategically important learning:** the gnark cgo wall was the *only* hard compile blocker between the shared Rust core and a phone binary. The Risk section's #1 "most-likely-to-kill" scenario (feature unification silently re-arming gnark on the FFI) was defused with an objective `cargo tree` tripwire and did not materialize. **One load-bearing plan deviation proved decisive:** Cargo *silently ignores* `default-features = false` on a `workspace = true` dep when the workspace definition doesn't itself set it — so the dep had to be switched to a direct path (`../../psy_plonky2_circuits`, still inside the allowed file) for the override to bite. Without that, the FFI kept pulling gnark while reporting success. That is the trap any future re-derivation must avoid.

---

## 1. RECOMMENDED STRATEGIC ADJUSTMENT

The strategy does not pivot. It **clears its last compile-time excuse and hands the baton to physics.** Three concrete adjustments, in priority order:

**A. Collapse the gap between the two mobile briefs: the surface is now both compiler-complete (parity brief) AND cross-compilable (this gate).** Until today, `STRATEGY_BRIEF_MOBILE_CORE_PARITY.md` correctly named its #1 next move — "cross-compile `proving_spike.rs` to a real device" — but flagged the blocker as *provisioning* ("no Xcode/NDK in this environment"). That framing was incomplete: there was *also* a hard dependency-graph blocker (gnark cgo) that would have failed the build even with a phone in hand. **That blocker is now removed and proven removed** (real `.so` produced). The mobile program's center of gravity therefore moves, cleanly and without hedging, from *"can the core even build for arm64?"* (answered: yes) to the single existential unknown both briefs already named: **on-device Plonky2 proving cost (peak RSS + wall-time) on a real mid-tier phone.** There is now zero compile-layer ambiguity in front of that measurement.

**B. Lock the gate as a durable invariant, not a one-shot fix — the parity guarantee is the asset.** The whole value is "node byte-identical / FFI gnark-off," and it is fragile: any future contributor re-adding an ungated `use gnark_plonky2_wrapper`, or running the mobile build in full-workspace scope (feature unification), silently re-breaks mobile with a cgo link error indistinguishable from "the gate is wrong." The Leverage and Risk sections converge: a tiny CI tripwire — `cargo build -p psy_wallet_core_ffi --target aarch64-linux-android` plus the two `cargo tree` assertions (gnark NOT in FFI graph; gnark STILL in node graph) — protects the entire mobile track in perpetuity and de-risks the *next* gate by guaranteeing the thing being profiled keeps compiling.

**C. Treat "it links" as necessary-not-sufficient and say so in every claim.** This gate unblocked *compilation only*. The Narrative honesty guardrail is non-negotiable: the external message is "the full ZK prover now cross-compiles to phone-native arm64 — we deleted a node-side settlement library the wallet never called, without changing the node by a byte," NOT "native mobile wallet shipped" and NOT any phone proving time. The proving-cost gate is still open; conflating the two torches credibility with the exact privacy-maximalist audience that is the beachhead.

---

## 2. THE SINGLE HIGHEST-IMPACT NEXT MOVE

**Run `proving_spike.rs` on a real mid-tier Android device (now that arm64 links) and record peak RSS + wall-clock for `register_user → exec_contract_call → exec_claim_with_external_proof`. One number on a real phone is worth more than the rest of this brief.**

This is the same move the parity brief named — but it is now *actionable*, not blocked. The gate converted "the build won't even produce an artifact" into "we have a 20MB gnark-free `.so`; load it on hardware and measure."

- **Why this and nothing else first:** it is the only mobile risk with *zero* evidence, it is invisible in the green build, and it is *existential*. If the spike passes on a ~6GB Android, the wedge ("your phone proves it, nothing leaves the device") is real and nearly uncontested — green-light the SwiftUI/Compose UIs and the public message. If it fails (OOM / 20–40s / jetsam SIGKILL on a foreground app mid-send), the architecture must include delegated proving (the existing prove-proxy:9999 fallback) *before* two UIs fork on a false premise — a far cheaper correction now.
- **It is now a provisioning + measurement problem, not a code problem.** The cgo blocker is gone; `build-android.sh`/`build-ios.sh` exist; `cargo-ndk` + the NDK at the spec'd path are present. The remaining external constraints: rustup targets must be installed (`aarch64-apple-ios`, `aarch64-apple-ios-sim`, plus the Android set) and a physical device/Xcode environment must be supplied.
- **Acceptance criteria:** report `{circuit, wall_ms, peak_rss_bytes}` per action on (a) a ~6GB Android device and (b) an older iPhone, against a hard foreground-safe memory budget. Land the CI tripwire (Adjustment B) in the same increment so the gate can never silently regress.
- **Bundle the cheap co-blocker (from the parity brief):** make the written key-custody call — enclave-callback signing (the Mode-A external-signature path is the exact template) vs. software-held key encrypted-at-rest — so UI teams build against the real trust boundary, not the enclave promise the current `private_key_str: String` FFI signature contradicts.

Everything else (durable crash-survivable tx-queue, read-only methods off the proving mutex, panic→`catch_unwind`→`PsyError`, App/Play positioning review, CI image for the bus-factor-1 toolchain) is real and queued — but all downstream of knowing whether the phone can prove at all.

---

## 3. CONSOLIDATED BRIEF FOR THE NEXT AGENT

**Mission of the next run:** Load the now-buildable gnark-free arm64 FFI on a real device and measure on-device Plonky2 proving cost (peak RSS + wall-time) for the first-run sequence; land a CI tripwire that keeps the gate from regressing; make the written key-custody decision. This is a measurement + provisioning + CI increment — NOT a circuit/prover/FFI-logic change.

**What you already have (proven today):**
- A real Android arm64-v8a artifact: `target/aarch64-linux-android/release/libpsy_wallet_core_ffi.so` (20MB), gnark-free, built in 2m19s via `cargo ndk -t arm64-v8a build --release -p psy_wallet_core_ffi`.
- The 15-method uniffi surface (parity brief) is compiler-complete AND now cross-compilable.
- The gate commit `9f9c9135` on `parth-generic-v1@feat/bridge-app-unified` — 4 files, node byte-identical.

**The gate's load-bearing facts — carry verbatim, never "clean up":**
- gnark is gated behind `gnark-wrap`, which stays in `default` of BOTH `psy_plonky2_circuits` and `psy_prover`. The node gets it via defaults; the FFI (already `default-features = false` on `psy_prover`) does not.
- `psy_prover` deps `psy_plonky2_circuits` by **direct path** `../../psy_plonky2_circuits` with `default-features = false, features = ["default-no-gnark"]`. This is load-bearing: a `workspace = true` dep would silently ignore `default-features = false` and re-pull gnark. Do NOT revert it to `workspace = true`.
- `default-no-gnark` is a named handle holding the exact historical default contents (`std`, the 3 `parth_core/serialize_*`, `serialize_speedy`); it exists because inline `dep/feature` slash syntax is illegal in a dep `features = []` array. Keep it; it is what makes the node feature set byte-identical.
- The 3 `worker_prove_groth16` methods (`bridge_wrap.rs`) carry `#[cfg(feature = "gnark-wrap")]`; the 2 prove_proxy RPC fns have `#[cfg(not(feature="gnark-wrap"))]` early-return error arms (`let _ = &input;` guards) + `#[cfg(feature="gnark-wrap")]` original bodies. Signatures unchanged so the `#[rpc]` trait impl still holds. The gnark-OFF arms are intentional clear-error stubs the FFI never calls.

**Build / verify discipline (FlowState — the env traps that look like gate bugs):**
- **Run feature checks FIRST, before any cross-build** — they are env-independent and decide "is the gate correct" in seconds, decoupled from "is my toolchain set up":
  - FFI: `cargo tree -p psy_wallet_core_ffi -i gnark-plonky2-wrapper` → expect "not in tree."
  - Node: from the parth workspace, `cargo tree -p psy_prover -i gnark-plonky2-wrapper` → expect still present.
- Scope the mobile build to the FFI package (`-p psy_wallet_core_ffi`), NEVER a bare workspace `cargo build` — a sibling pulling `psy_prover` with defaults can re-arm gnark via feature unification (Risk R1).
- `build-std` is set **globally** in `.cargo/config.toml` (not wasm-scoped), so every new target recompiles libstd from source — validate ONE target (Android arm64-v8a, already proven) before fanning out; pay the cold cost once.
- Disk: ~11GB free, ~20GB target dir; `cargo clean -p psy_wallet_core_ffi` between experiments, not a full nuke. `export CARGO_NET_GIT_FETCH_WITH_CLI=true`; toolchain pinned `nightly-2025-09-20`; `ANDROID_NDK_HOME=/opt/homebrew/share/android-commandlinetools/ndk/27.2.12479018`, `ANDROID_HOME=/opt/homebrew/share/android-commandlinetools`.
- Avoid `cd <unicode-path> && <cmd>` one-liners — the Chinese-path dir under `&&` silently no-ops to the litepaper root; drive builds with absolute `--manifest-path` or an exported `$PSY` root.

**Innovation surface this gate unlocks (signer-neutral, zero circuit change — preserve it):**
1. Seedless wallet keyed by Secure Enclave / StrongBox via the Mode-A external-signature path (`get_sig_hash → host sign_prehash → exec_contract_call_with_external_signature`) — the key never leaves hardware; Psy still proves auth in-circuit.
2. "Sign on phone, prove on desktop" via detached note-inclusion proofs (`prove_private_note_inclusion_json` → portable artifact → `exec_claim_with_external_proof_json`) — the graceful-degradation hedge if on-device proving cost is high.
3. Byte-identical cross-platform conformance as an audit feature: same Rust core for web/iOS/Android; publish a byte-equal proof vector across all three.

**Dominant risks for the next run (own explicitly):**
- **[EXISTENTIAL] On-device proving time/memory — still zero device evidence.** The gate removed compilation as the excuse; physics is now the only thing left. Defuse via Move #2.
- **[HIGH] Gate regression via feature unification or an ungated `use`.** Defuse via the CI tripwire (Adjustment B); never run the mobile build in full-workspace scope.
- **[HIGH] Secret key crosses FFI as plain `String`** — contradicts the enclave promise; resolve in the bundled custody decision.
- **[MED] Lock files / `[patch]` churn.** Both `Cargo.lock`s were reverted clean; the gnark `[patch]` (local path) is inert for the FFI but load-bearing for the node — leave it; never hand-edit the lock or patch block.

**Positioning / Narrative guardrails:**
- Lead with the *property* ("the proof never leaves your phone"), stay SILENT on proving speed until the device spike lands. External claim = "the core cross-compiles to arm64," NOT "mobile wallet shipped."
- Psy is an independent chain — never L1/L2/rollup; gnark is Psy's *own* settlement wrap, not an Ethereum artifact. Use "Deposit"/"Withdraw," never "bridge" as a verb; show the Psy ID / shielded address; "shielded," never "anonymous."
- One-line hook (builder/investor): "We just made a full ZK prover cross-compile to arm64 — the only blocker was a Go library the wallet never even calls, gated out without changing the node by a byte."

**Repo state of record:** psy-sdk-fresh on `feat/mode-a-external-sig`; working tree carries only pre-existing unrelated noise (`Cargo.toml`, `Cargo.lock`, `config.json`, `rust-toolchain.toml`, `pnpm-lock.yaml`, `psy-wallet-core-ffi/src/lib.rs`) — leave it. The gate itself is committed in the sibling parth repo at `9f9c9135` (4 files only). Not pushed; never push or touch deploy branches without an explicit ask.

---

_Bottom line: the gate is a clean, narrow, proven win — the native core now produces a gnark-free arm64 `.so`, and the node stays byte-identical, verified at the feature-resolution level and by a real `cargo ndk` build. That removes the last compile-layer unknown in front of the mobile thesis. Everything that can still quietly invalidate the wedge is now downstream and physical: this has never proven on a phone, and the key-custody story doesn't yet match the enclave promise. Spend the next increment on a real-device proving + memory measurement, a CI tripwire to keep the gate from regressing, and the enclave-callback signing decision._
