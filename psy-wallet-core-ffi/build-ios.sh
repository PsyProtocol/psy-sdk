#!/usr/bin/env bash
# Build the iOS .xcframework + Swift bindings for the Psy ZK wallet core.
#
# Requirements (run on macOS):
#   - Xcode + command line tools
#   - rustup target add aarch64-apple-ios aarch64-apple-ios-sim
#   - the parth-generic-v1 path deps present (this repo expects them next to it)
set -euo pipefail
cd "$(dirname "$0")"

CRATE=psy_wallet_core_ffi
LIB=lib${CRATE}.a
OUT=bindings/swift
XCF=PsyWalletCore.xcframework

echo "▸ cargo build (device + simulator, release)…"
cargo build --release -p "$CRATE" --target aarch64-apple-ios
cargo build --release -p "$CRATE" --target aarch64-apple-ios-sim

echo "▸ generate Swift bindings…"
rm -rf "$OUT" && mkdir -p "$OUT"
cargo run --bin uniffi-bindgen -- generate \
  --library "target/aarch64-apple-ios/release/${LIB}" \
  --language swift --out-dir "$OUT"

# uniffi emits <module>.swift + <module>FFI.h + <module>FFI.modulemap.
# The xcframework needs the headers (.h + modulemap) alongside each static lib.
echo "▸ assemble ${XCF}…"
rm -rf "$XCF"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/${LIB}"     -headers "$OUT" \
  -library "target/aarch64-apple-ios-sim/release/${LIB}" -headers "$OUT" \
  -output "$XCF"

echo "✓ ${XCF} + ${OUT}/*.swift ready — drag both into the SwiftUI app."
