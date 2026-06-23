# psy-wallet-core-ffi

**One Rust core → web (WASM) + iOS (Swift) + Android (Kotlin).** This crate is the
**native** FFI binding over the Psy ZK wallet core (`psy_prover::session::WalletSession`
+ `UserProverWorkerStore`) — the exact same core that `psy-rust-sdk`'s
`WasmRpcServer` exposes to the web wallet via wasm-bindgen. Here we expose it to
**Swift + Kotlin** via [uniffi](https://mozilla.github.io/uniffi-rs/), so the iOS
and Android **standalone Psy ZK wallets** (native UIs, own ZK key in Secure
Enclave / Keystore — *not* MetaMask) reuse 100% of the proving/identity/chain
logic instead of reimplementing it.

```
psy_prover / psy_provider  (ZK wallet + Plonky2 prover, shared)
   │ wasm-bindgen → WASM ...... Web wallet (React)        [psy-rust-sdk]
   │ uniffi       → Swift ..... iOS wallet (SwiftUI)      [this crate]
   └ uniffi       → Kotlin .... Android wallet (Compose)  [this crate]
```

## Why this works
The core already compiles natively (the CLI prover runs off-WASM), the crate is
`cdylib`/`staticlib` (FFI-ready), and the wallet API is **JSON-in / JSON-out** —
trivial to cross a uniffi boundary. So mobile gets **native Plonky2 proving** (no
22 MB WASM download, faster than a WebView), with the same JSON contracts as web.

## Status
- ✅ **Ported & ready** (enough to run the proving spike): `new`, `register_user`,
  `add_user`, `get_zk_public_key_json`. `register_user` proves the ZK-signature
  circuit, so it is the workload the spike measures.
- 🚧 **Scaffolded** (compile-clean stubs that name the exact `wasm/mod.rs` line to
  port): `exec_contract_call_json` (wasm/mod.rs:358), `exec_claim_batch_json`,
  `prove_private_note_inclusion_json` (:729), `sign_and_submit` (:1120). Each is a
  mechanical port — same `self.wallet_session.*` sequence, behind the async Mutex.

## 🚩 Step 1: the on-device proving spike (do this first)
Plonky2 proving memory/time on a phone is the make-or-break unknown. Validate it
before building any UI:

```bash
# Desktop baseline (peak RSS via /usr/bin/time):
PSY_CONFIG=/path/to/config.json /usr/bin/time -l \
  cargo run --release --bin proving_spike
```
Then call the same `register_user` through the uniffi binding from a minimal iOS
+ Android test target and record **wall time + peak memory**. If memory is too
tight, the fallbacks (smaller proof config / circuit tuning / — last resort — a
proving service) need to be known now, because they change the product.

## Build
```bash
./build-ios.sh        # → PsyWalletCore.xcframework + bindings/swift/   (needs macOS + Xcode)
./build-android.sh    # → jniLibs/<abi>/*.so + bindings/kotlin/         (needs Android NDK + cargo-ndk)
```
Prereqs: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim
aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`,
`cargo install cargo-ndk`, and the `parth-generic-v1/client_prover` path deps
present next to this repo (the workspace points at them).

## Usage from Swift (sketch)
```swift
let core = try await PsyWalletCore(rpcConfigJson: configJson)
let pkHash = try await core.registerUser(privateKeyStr: keyHex, signType: "zk")
```
Kotlin is identical (uniffi generates idiomatic suspend functions).

## What's NOT in this crate (per-platform)
- **Key storage:** iOS Secure Enclave / Keychain, Android Keystore — the app owns
  the ZK private key; this core only receives it for proving.
- **UI:** SwiftUI / Jetpack Compose, native per platform.
