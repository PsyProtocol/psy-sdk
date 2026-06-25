# STRATEGY BRIEF — Shared Mobile Core, Now In The Build (FFI wired + API-drift closed)

_Repositioning / strategic realignment, synthesized from the 8-role PsyVerse Flow strategy pass (Mission, Positioning, Conviction, FlowState/DX, Innovation, Leverage, Narrative, Risk) + the execution outcome. Repo: `psy-sdk`. Working branch: `feat/mode-a-personal-sign`. Increment commit: `8509e8e6`. Date: 2026-06-25._

_Sibling briefs in this repo — do NOT clobber, they cover adjacent layers of the SAME `psy_wallet_core_ffi` core:_
- _`STRATEGY_BRIEF_MOBILE_CORE_PARITY.md` — the 15-method uniffi surface / wasm↔FFI parity run._
- _`STRATEGY_BRIEF_GNARK_FFI_GATE.md` (2026-06-24, `9fd5341b`) — the **gnark mobile-FFI gate**: feature-gating gnark behind `gnark-wrap` so the FFI cross-compiles gnark-free; real Android arm64 `.so` built in 2m19s. That gate ENABLED the wiring this brief lands._

This is the post-increment realignment. It states (1) what changed and the recommended strategic adjustment, (2) the single highest-impact next move, (3) the consolidated brief for the next agent.

---

## 0. WHAT WE NOW KNOW (the increment that just landed)

**The shared native-mobile core is now an actual member of the build and compiles green. "Compiler-complete on host" is now "wired, drift-free, `cargo check` 0."**

The prior gnark gate proved the FFI *could* cross-compile to a phone. This increment proved it is *in the workspace and stays in sync with the chain*. Two real, load-bearing blockers were found and closed in the merge of `feat/mode-a-external-sig` into `feat/mode-a-personal-sign`:

1. **The FFI crate was not in the build.** `psy-wallet-core-ffi/` used `{ workspace = true }` deps but was absent from `[workspace.members]`, so `cargo build -p psy_wallet_core_ffi` — the exact invocation inside `build-ios.sh` / `build-android.sh` — failed with "did not match any packages." It compiled in isolation and was invisible to the release scripts. Fixed: added to `members` (`Cargo.toml:2`).
2. **API drift vs the pinned parth rev `ac198474`.** Four FFI call sites called `.to_string()` on session returns that are now **structs, not hashes**: `exec_contract_call_json` (returns `TxMetadata` → `serde_json::to_string`) and `sign_and_submit` ×3 incl. the external-sig path (return `TxSubmitMetadata` → `.tx_hash.to_string()`). Each fix mirrors the already-correct WASM SDK (`wasm/mod.rs:681`, `:1932`) — i.e. the wasm and FFI surfaces were silently diverging at the seam where the chain's return types changed.

**Verified acceptance (all green):** `cargo check -p psy_wallet_core_ffi --all-targets` → 0 (lib, `uniffi-bindgen` bin, `proving_spike` bin, tests); `cargo check -p psy_rust_sdk` → 0 (the workspace-member addition + `rustls-tls` feature unification did not regress the web SDK). Residual output is pre-existing parth-dependency warnings only. Committed `8509e8e6`, **not pushed**, tree clean, 8 other local branches untouched.

**A second, decision-shaping learning — the supplied feedback was a context mismatch.** The `WALLET_RELEASE_REVIEW` feedback (Nostr outer-tag leak `23f4bd3`; device-key vault `e3581b7`; symbols `publishEncryptedPrivatePayment`, `IStorageData`, `claimables.ts`) targets the **psy-wallet browser-extension repo**, not psy-sdk. Verified: neither hash resolves here (`git cat-file` → not a valid object), none of the symbols/files exist in this tree. Those blockers were already closed *in their own repo* (per the execution outcome) and are **not actionable in psy-sdk**. The realignment lesson: in a monorepo-of-repos, a feedback item is only a blocker for the repo that owns the symbol — route by symbol-ownership, not by topic.

**The single most strategically important fact:** the two layers that gate the entire "three skins, one core" thesis — *does it cross-compile?* (gnark gate, done 06-24) and *is it wired in and drift-free with the chain?* (this increment, done 06-25) — are now both green. What remains unproven is **not** code-completeness. It is **physics**: nobody has run a Plonky2 proof on an actual phone.

---

## 1. RECOMMENDED STRATEGIC ADJUSTMENT

The 8-role pass converges hard on one point, and the increment sharpens it rather than changing it:

**Stop spending leverage on making the core *exist* and *compile*. That work is now done. Spend the next unit of effort on the one unmeasured physics number that validates — or kills — the whole native-mobile bet: peak RSS + wall-time of on-device Plonky2 proving.**

Why this is the adjustment, grounded in the sections:

- **Leverage §1 + Conviction "reject 'WASM mismatch is a small bug'":** every downstream asset — 3,630 LOC of Swift, 2,654 of Kotlin, two UI tracks, the headline "your phone proves it" message — is staked on a single number that has *zero device evidence*. `register_user` clears at ~5.7s on an M-series Mac; a mid-tier phone is an unknown. If a 30s proof OOM-kills a foreground "sending money" app, the architecture must add a `prove-proxy:9999` delegated-proving fallback — a correction **10× cheaper before two UI tracks fork than after.** Now that the FFI links for arm64 (gnark gate) *and* is in the build (this increment), the spike is no longer blocked by anything but provisioning a device.

- **Positioning §3 + Risk #1:** the strategic wedge ("the only network where x402 payments are private") and the worst kill-risk (shipping a privacy product that isn't private) both live in the *user-facing wallet repo*, not here. psy-sdk's job in the wedge is to be the **trustworthy, non-drifting core all three clients share.** This increment's API-drift fix is exactly that job: it caught the wasm↔FFI seam diverging at the chain's return-type boundary. So the adjustment for psy-sdk specifically is: **make drift impossible, not just currently-absent.**

- **Leverage §2 (the durability multiplier):** the multiplier is "three skins, one core," but it only pays if drift is structurally prevented. We just found drift the hard way (manually, via a broken `.to_string()`). The repositioning is to convert that manual catch into an **enforced gate**: a CI smoke test that runs the real first-run sequence (`new → get_random_keypair → register_user → exec_contract_call`) against staging on every chain bump, plus a **circuit-fingerprint assert at `new()`** so a chain redeploy fails fast with "app update required" instead of silently bricking installed wallets (the documented `VirtualTarget` mismatch class).

- **Leverage §3 + Conviction belief #3 ("keys never leave the device"):** every FFI method still takes `private_key_str: String`, which over uniffi lands in a pageable, non-zeroized Swift/Kotlin string — directly contradicting the Secure-Enclave promise. The already-merged Mode-A external-signature path (proven: external `eth_sign` accepted by the secp256k1 circuit, zero circuit change) is the template to reuse as the **enclave-signing callback boundary**. This is the custody primitive that makes the headline security claim *true* across web + iOS + Android at once.

**Net:** psy-sdk pivots from *build-the-core* to *prove-and-protect-the-core* — measure it on a real device, gate it against chain drift, and harden its key-custody seam. Everything else (UI polish, growth copy) is downstream of the device measurement and should not be funded ahead of it.

---

## 2. THE SINGLE HIGHEST-IMPACT NEXT MOVE

**Run the on-device proving spike — record `{circuit, wall_ms, peak_rss_bytes}` for `register → send → claim` on a real mid-tier Android (~6GB) and an older iPhone.**

This is the highest-leverage move in the entire ecosystem, and the two blockers that previously stood in front of it are now both gone:
- The FFI cross-compiles gnark-free to arm64 (gnark gate, `9fd5341b`, real 20MB `.so` built).
- The FFI is in the workspace and `cargo check`-green with no chain drift (this increment, `8509e8e6`).

So the spike is now purely a **provisioning** task, not an engineering one. The code already exists: `proving_spike.rs` (a build target that `cargo check` just confirmed compiles), `build-ios.sh`, `build-android.sh`.

Concrete handoff:
1. Cross-compile the FFI for both targets (`cargo ndk -t arm64-v8a build --release` for Android — proven to finish in 2m19s; `build-ios.sh` for the xcframework — already built once).
2. **Android first** — it's the tighter memory budget *and* its `jniLibs/` was the placeholder, so it's the true unknown. (iOS xcframework already built once; Android is where OOM risk is highest.)
3. Run `register_user → sign_and_submit (transfer) → claim` via `proving_spike`, capture peak RSS and wall-time per circuit.
4. **Decision the number forces:** if peak RSS fits comfortably under a foreground app's budget and wall-time is tolerable with a progress UI → the "your phone proves it" architecture is validated, both UI tracks proceed. If it OOMs or exceeds ~30s → add the `prove-proxy:9999` delegated-proving fallback to the core **now**, before the Swift/Kotlin UIs harden around a pure-local assumption.

One real-device number resolves the public message, the architecture decision, and whether two UI investments are sound — worth more than any further code-completeness work.

_Honest blocker to flag in the handoff:_ this environment has no Xcode/NDK device toolchain provisioned, so the spike cannot be executed here — it must run on a machine with a phone attached. That provisioning gap is the only thing between us and the number.

---

## 3. CONSOLIDATED BRIEF (for the next agent)

**Repo / safety:** `psy-sdk`, branch `feat/mode-a-personal-sign`, ahead of origin (`8509e8e6` + this brief), **never pushed**, tree clean, other 8 branches untouched. Commit narrowly; one fix per commit; do not clobber the sibling briefs.

**State of the core (psy-sdk):**
- Web SDK (`psy_rust_sdk`, WASM) — green, ships the wallet today; the reference for correct return-type handling.
- Native FFI (`psy_wallet_core_ffi`) — 1441-line lib, 15 uniffi methods, **now a workspace member**, `cargo check --all-targets` = 0, cross-compiles to Android arm64 gnark-free. iOS xcframework built once; Android `jniLibs/` still a placeholder until the spike build runs.
- gnark is feature-gated behind `gnark-wrap` (in `default` for node parity, out for the FFI). Node stays byte-identical.

**Do, in priority order:**
1. **On-device proving spike** (§2) — the one number. Highest leverage; only blocked by device provisioning.
2. **Drift gate** — staging smoke test of the real first-run sequence on every chain bump + circuit-fingerprint assert at `new()` → fail fast with "app update required", not a silent brick. (This increment found drift manually; make it impossible.)
3. **Enclave-signing custody primitive** — reuse the merged Mode-A external-signature callback so the FFI stops taking `private_key_str: String`; makes "key never leaves the device" true across all three clients.

**Do NOT:**
- Treat psy-wallet-extension feedback (Nostr tags, device-vault, `claimables.ts`, etc.) as actionable here — it's a different repo; route blockers by symbol-ownership. Those are already closed in psy-wallet.
- Fund UI polish or "your phone proves it" growth copy ahead of the device measurement — both are downstream of it (Risk + Leverage distribution caveat agree).
- Re-arm gnark on the FFI — keep the `cargo tree -p psy_wallet_core_ffi -i gnark-plonky2-wrapper` (must be empty) tripwire.
- Chase general "ZK L1 for everything" framing; the wedge is private payments / private x402, and psy-sdk's role is the trustworthy shared core beneath it.

**The one-line operating creed:** the core now exists, cross-compiles, and is wired in drift-free — the win condition is no longer code-completeness, it's the first real-device proof that the phone can actually prove, behind a drift gate and an enclave-custody seam that make the privacy and security claims true across web, iOS, and Android simultaneously.
