# STRATEGY BRIEF — Web Wallet Mode-A (External-Signature Submit Path)

_Repositioning / strategic realignment, synthesized from the 8-role PsyVerse Flow strategy pass + the execution outcome. Branch: `feat/mode-a-external-sig`. Date: 2026-06-23._

_Sibling brief: `STRATEGY_BRIEF_MOBILE_CORE_PARITY.md` (the iOS/Android core-parity run — separate product, software-held keys, NOT MetaMask). This brief is the Web wallet. Both ride the same `psy_wallet_core_ffi`._

---

## Executive Summary

**What this is.** The web Psy wallet (Mode-A) is the only private-payments wallet you can drive with a MetaMask you already have: your existing EVM key signs each transaction, the proof is built client-side, and the key never leaves the extension. The load-bearing primitive is the **external-signature submit path** in the shared Rust core — the SDK method that authorizes a contract-call end-cap with an externally-produced `eth_sign` signature instead of a software-held key.

**What changed this cycle (the strategic update).** The open question that gated the entire web track is now **answered and proven on live staging, with zero circuit changes.** Before this run, two things were known: (1) the `a_mode_spike` proved the *circuit* accepts a raw k256 `sign_prehash` (byte-identical to MetaMask `eth_sign`) over the Psy `sig_hash`, and (2) the 15 FFI methods proved+submitted with a *held* key. The single unknown was the **submit path**: could the SDK take a caller-supplied external signature and drive a *real contract call* to chain acceptance? It can. A real public `simple_mint` call was submitted to live staging authorized **solely** by an external signature — accepted, real tx hash, no `VirtualTarget`.

**The deliverable, verified:**
- **NEW FFI methods** in `psy-wallet-core-ffi/src/lib.rs`: `get_sig_hash` (lib.rs:1016) primes the session and returns the exact 32-byte hash to sign; `exec_contract_call_with_external_signature` (lib.rs:1071) consumes that primed session + the external sig and submits. A SEC1-compressed-pubkey preflight (0x02/0x03) turns an opaque in-circuit failure into a clear error.
- **NEW staging spike** `mode_a_external_sig_contract_call_accepted_by_staging` (lib.rs:1311): **PASS** — `tx=ee4eda74d380ff4753c764c7d8959ac320c6c587be4272fd464449628da48796`, submit ~492s, sighash ~119s (real Plonky2 proving, nothing stubbed).
- **The 15 existing methods and `a_mode_spike` are untouched.** `cargo check -p psy_wallet_core_ffi` green; `a_mode_spike` re-verified PASS (574s) after the preflight edit.

**The strategic consequence.** The position graduates **from feasibility to foundation.** The wedge — "bring your own MetaMask key to private payments, key never exported, no circuit changes" — is no longer a claim about a spike; it is a working submit path with a staging tx hash. The program's center of gravity now moves off "prove the submit path" (DONE) and onto **two questions the green build cannot answer: (a) does production MetaMask still expose raw `eth_sign` over an arbitrary 32-byte hash, and (b) is the parth-generic-v1 prover dependency landable.** Both are external/integration risks, not cryptography risks.

---

## 1. Recommended Strategic Adjustment

**Pivot from "prove the external-sig submit primitive" (DONE) to "harden the seam and validate the one assumption that can retroactively invalidate it — `eth_sign` availability — before any web/WASM wiring is built."**

The 8 strategy roles converge on one realignment, and the execution outcome makes it actionable:

1. **The primitive is proven; stop treating it as the open question.** Innovation, Leverage, and Conviction all framed the submit path as "a one-variable swap, not a leap." That swap is now collected: the held-key end-cap authorization is replaced by `prove_secp_sign(external_sig)` through a `SignatureUser` impl that consumes the *unchanged* secp256k1 circuit. The hypothetical hard blocker (a circuit verifying an EIP-191-prefixed or differently-bound message) **did not materialize.** The two-call seam is the contract for the web team now, not an open work item.

2. **The make-or-break has shifted from circuit acceptance to `eth_sign` availability.** Risk flagged this as the highest-stakes latent risk, and it is now the program's gating question: the entire wedge rests on production MetaMask producing a signature over an arbitrary 32-byte Psy `sig_hash` with **no EIP-191 prefix**. Many wallet UIs deprecate or block raw `eth_sign` (security warnings, some providers refuse it). If the only available path is `personal_sign`/EIP-712 — which prepend a prefix and change the signed bytes — then the unmodified circuit will reject it, and authorizing the prefixed hash **would require a circuit change = the hard blocker.** This must be answered against a real, current MetaMask **before** the web Connect→…→Send wiring forks, because it can retroactively invalidate the staging proof's relevance to the shipping product. This is a 30-minute browser check, not an engineering project — do it first.

3. **Resolve the cross-repo dependency honestly (it is co-blocking the "self-contained SDK" claim).** The Mode-A submit path in this repo depends on commit `9abda3c1` in the **parth-generic-v1** tree (new `signature/users/external_secp256k1_user.rs`, a `mod.rs` export, and `register_external_secp_user` in `memory_wallet.rs`). That tree is outside the allowed workspace, and the commit was made by a prior run in violation of the "work ONLY in psy-sdk-fresh" constraint. It is **not** a circuit change — it is a `SignatureUser` that only *consumes* the unchanged secp circuit — so it is not the hard blocker. But the SDK foundation is **not self-contained within psy-sdk-fresh**: that prover change must land alongside via the Psy core team as the canonical upstream API (`register_external_secp_user` + a public two-phase signing surface). Frame the ask to the core team as a 1–2 line public-API addition, not a redesign.

4. **Lock the two-call seam as the binding contract so the WASM/web port can wire to it without re-discovery.** Promote the hand-off shape into the README as the contract: `get_sig_hash(pk_hash, call_data) → sighash_hex` ; web does `await ethereum.request(eth_sign, sighash_hex)` (raw hash, no prefix) ; `exec_contract_call_with_external_signature(pk_hash, call_data, signature_hex)` where `signature_hex = compressedPubkey(33) ‖ r(32) ‖ s(32)`. Critically, document the **load-bearing nonce-binding invariant** found this run: the submit method must **reuse the session already primed by `get_sig_hash`** — re-running `start_session`/`prove_contract_call` shifts the nonce, the recomputed sighash no longer matches the signed one, and the in-circuit ECDSA check fails with `VirtualTarget set twice`. The web port must call these two methods as a bound pair with no state advance between them, and reject submit if the nonce moved (stale-sig replay guard).

**What to explicitly NOT do now:** do not write user-facing "key never leaves MetaMask" launch copy until the `eth_sign` check passes and the web flow is wired+tested (it is a per-tx claim, not a session claim — every send is one fresh popup); do not start wasm-pack/web build this run (explicitly out of scope); do not refactor the 15 existing methods or `a_mode_spike` (staging-proven, fidelity is the value); do not bake "MetaMask" into the core type — keep the input as raw `(pk[33], r‖s[64], sighash)` so the same primitive serves hardware/MPC/secp-passkey signers later. First users, in order: **(1) internal Psy `/app` web-wallet developers** (the captive, immediate consumer — this method is the literal blocker for their Connect→…→Send flow), (2) EVM-native users who refuse a second seed phrase, (3) privacy maximalists wanting hardware-custody of spend authority, (4) downstream SDK/Safe/AA integrators.

---

## 2. The Single Highest-Impact Next Move

**Open a current production MetaMask and confirm it will return a signature over an arbitrary 32-byte hash with no EIP-191 prefix (raw `eth_sign`, or whatever the closest available primitive is). One browser check answers whether the entire differentiated web track is real or needs a circuit change.**

- **Why this and nothing else first:** it is the only assumption with *zero* current evidence, it is invisible in the green staging proof (the spike used a synthetic k256 `sign_prehash`, not a real MetaMask popup), and it is *existential* to the web wedge. If raw `eth_sign` over an arbitrary hash is available → the staging proof transfers directly to the product, green-light the WASM port + web wiring. If MetaMask only offers prefixed `personal_sign`/EIP-712 → the unmodified circuit rejects it, and the choice is (i) a circuit change to verify the prefixed hash (the documented hard blocker, collapses the "no circuit changes" moat), or (ii) degrade the web wallet to a software-held key model — at which point it is just the mobile architecture in a browser and the differentiation evaporates. **This determines whether the web track ships at all**, and it is far cheaper to learn now than after two wiring increments.
- **It is a 30-minute browser/provider check, not a code problem.** No prover, no proving, no cross-compile. Inspect the MetaMask provider API surface on a current release; confirm the byte-exact preimage the wallet signs equals the `get_sig_hash` output.
- **Acceptance criteria:** a documented yes/no on "production MetaMask signs an arbitrary 32-byte hash, prefix-free, and the signed bytes equal `get_sig_hash` output" — plus, if no, the exact prefix/encoding MetaMask imposes, so the circuit-change cost can be scoped precisely.
- **Bundle the cheap co-blocker:** in the same pass, escalate the parth-generic-v1 `9abda3c1` dependency to the Psy core team as the canonical upstream API ask, so the SDK foundation becomes self-contained and reproducible from psy-sdk-fresh alone.

Everything else (the WASM-SDK port, the Connect→Login→Balance→Faucet→Claim→Send→Activity web flow, the golden interop-vector standard, sponsored/gasless proving, BYO-signer generalization to MPC/hardware/secp-passkeys) is real and queued — but all of it is downstream of knowing whether the user's MetaMask can produce the signature this primitive consumes.

---

## 3. Consolidated Brief (the one-page contract)

**Thesis (one sentence):** Psy's authorization is a *swappable in-circuit ECDSA verifier*, not a hardwired key scheme — so the SDK can authorize a private-payment end-cap with an external `eth_sign` signature (proven on live staging, zero circuit changes), making every existing EVM key a private-payments key while the proof stays client-side and the key never leaves the wallet.

**State of play (verified against source, 2026-06-23):**
- Two NEW FFI methods exported: `get_sig_hash` (lib.rs:1016) and `exec_contract_call_with_external_signature` (lib.rs:1071), with a SEC1-pubkey preflight. The two-call seam is the documented hand-off contract for the web/WASM port.
- NEW staging spike `mode_a_external_sig_contract_call_accepted_by_staging` (lib.rs:1311): **PASS**, real public contract call accepted, `tx=ee4eda74…48796`, submit ~492s / sighash ~119s, real Plonky2 proving.
- `cargo check -p psy_wallet_core_ffi` green (only pre-existing dependency-crate warnings); `a_mode_spike` re-verified **PASS** (574s) after the preflight edit; the 15 existing methods untouched.
- Commits on `feat/mode-a-external-sig`: `a10f5402` (the two methods), `d2962009` (staging spike + reuse-primed-session fix), `0ded844f` (pubkey preflight). No push. Deploy branch `feat/shield-poseidon-bridge` untouched. Toolchain/gnark/TLS config unchanged.
- **Cross-repo caveat:** depends on parth-generic-v1 commit `9abda3c1` (new `ExternalSecp256K1User` + `register_external_secp_user`) — outside the allowed workspace, made by a prior run; consumes the unchanged secp circuit (NOT a circuit change), but the SDK foundation is not self-contained without it. Escalate to core team.
- Pre-existing dirty files (Cargo.lock/toml, config.json, two `*.d.ts`, pnpm-lock, rust-toolchain) left untouched per safety rules.

**Risk register (ranked):**
1. **[EXISTENTIAL] Production MetaMask may not expose raw `eth_sign` over an arbitrary 32-byte hash.** The staging proof used a synthetic k256 sig; real-wallet availability is unverified. If only prefixed `personal_sign`/EIP-712 is available, the unmodified circuit rejects it → circuit change (hard blocker) or degrade to software-held keys (differentiation lost). **Defuse via Move #2 before any web wiring.**
2. **[HIGH] Cross-repo dependency on parth-generic-v1 `9abda3c1`.** The submit path is not self-contained in psy-sdk-fresh and the prover change was committed outside the allowed workspace. Escalate as the canonical upstream public-API ask; do not silently rely on a local edit.
3. **[HIGH] Nonce-binding / stale-sig replay.** `get_sig_hash` and the submit must be a bound pair with no state advance between them; re-deriving the session shifts the nonce and the in-circuit ECDSA check fails (`VirtualTarget set twice`). The web port must enforce this and reject submit if the nonce moved.
4. **[MED-HIGH] Per-tx popup UX.** "Key never leaves MetaMask" is a per-transaction claim — every Send/Claim/end-cap is one fresh `eth_sign` prompt. Position as "every payment individually authorized by your wallet," not invisible signing; batch where possible (web-wiring concern, not this SDK run).
5. **[MED] WASM-port divergence.** Host-cargo spike passes but `wasm32` build of the new methods may surface async_trait `?Send` / k256-in-wasm / getrandom issues. The trait is already wasm-aware (`async_trait(?Send)` cfg); keep new methods free of `Send`-bound assumptions. Defer actual wasm-pack to its own run.
6. **[MED] Chain-source coupling.** SDK WASM must match the chain's `user_id` strategy; generic-v1 vs shield-poseidon are not interop. Confirm which chain the shipping web wallet targets before declaring the product "proven on staging."

**The two-call hand-off seam (the contract for the WASM/web port):**
1. `sighash_hex = get_sig_hash(pk_hash, call_data_json)` — primes the session, returns the exact bytes to sign.
2. web: `signature = await ethereum.request(eth_sign, sighash_hex)` — **raw hash, no EIP-191 prefix**.
3. `tx = exec_contract_call_with_external_signature(pk_hash, call_data_json, signature_hex)` where `signature_hex = compressedPubkey(33) ‖ r(32) ‖ s(32)`. Must reuse the session primed in step 1 (no state advance between 1 and 3).

**Demo that makes people get it (≤60s, once the web flow is wired):** open the web wallet — **there is no "create wallet" screen, no seed phrase**, just "Connect MetaMask." Connect → approve. Click Send → MetaMask pops once to sign the tx hash → approve. The transfer settles on Psy, shielded (show the explorer: no readable amounts, no sender↔receiver link). Kicker: *"MetaMask just authorized a zero-knowledge proof. Your key never left the extension, and the chain never saw who paid whom."* The single image if only one: the Connect-MetaMask screen with no seed-phrase step — *"You logged into a private chain with the wallet you already have, and never created a thing to lose."*

**Don't claim:** user-facing "key never leaves MetaMask" copy before the `eth_sign` check passes and the flow is wired+tested; "MetaMask everywhere" (Mode-A is web-only; iOS/Android hold keys in software — see sibling brief); a self-contained SDK (depends on the parth prover change until upstreamed); L1/L2 or "bridge" (independent ZK chain; "Deposit"/"Withdraw"; show the Psy ID / shielded address).

---

_Bottom line: the load-bearing primitive is proven — a real contract call authorized solely by an external `eth_sign` signature was accepted on live staging with zero circuit changes, and the two-call seam is shaped for the port. The position has graduated from feasibility to foundation. The remaining risk is no longer cryptography; it is whether production MetaMask will actually hand us a prefix-free signature over the Psy sig_hash, and whether the upstream prover dependency lands cleanly. Answer the `eth_sign` question in a browser before building any web wiring — it is the one thing that can quietly invalidate the differentiated web track after the UI is built on top of it._
