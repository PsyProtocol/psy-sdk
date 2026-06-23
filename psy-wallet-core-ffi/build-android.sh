#!/usr/bin/env bash
# Build the Android .so (all ABIs) + Kotlin bindings for the Psy ZK wallet core.
#
# Requirements:
#   - Android NDK installed, ANDROID_NDK_HOME set
#   - cargo install cargo-ndk
#   - rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   - JNI_LIBS points at your Android app's jniLibs dir
set -euo pipefail
cd "$(dirname "$0")"

CRATE=psy_wallet_core_ffi
LIB=lib${CRATE}.so
JNI_LIBS="${JNI_LIBS:-../psy-android/app/src/main/jniLibs}"
OUT=bindings/kotlin

echo "▸ cargo ndk build (arm64-v8a, armeabi-v7a, x86_64; release)…"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$JNI_LIBS" \
  build --release -p "$CRATE"

echo "▸ generate Kotlin bindings…"
rm -rf "$OUT" && mkdir -p "$OUT"
cargo run --bin uniffi-bindgen -- generate \
  --library "target/aarch64-linux-android/release/${LIB}" \
  --language kotlin --out-dir "$OUT"

echo "✓ .so -> ${JNI_LIBS}/<abi>/, Kotlin -> ${OUT}/ — add the package to the app + java.library.load."
