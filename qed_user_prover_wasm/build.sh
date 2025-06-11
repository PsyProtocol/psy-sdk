#!/bin/bash

# QED User Prover WASM Build Script
# This script builds the WebAssembly module for the QED user prover

set -e

echo "Building QED User Prover WASM module..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed. Please install it first:"
    echo "curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Check if wasm-opt is installed (optional but recommended)
if ! command -v wasm-opt &> /dev/null; then
    echo "Warning: wasm-opt is not installed. Install it for better optimization:"
    echo "npm install -g wasm-opt"
fi

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf pkg/
rm -rf target/

# Build the WASM package
echo "Building WASM package..."
wasm-pack build --target web --out-dir pkg --release

# Optimize the WASM binary if wasm-opt is available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM binary..."
    wasm-opt -Oz --enable-mutable-globals pkg/qed_user_prover_wasm_bg.wasm -o pkg/qed_user_prover_wasm_bg.wasm
fi

# Generate TypeScript definitions
echo "Generating TypeScript definitions..."
wasm-pack build --target bundler --out-dir pkg-bundler --release

# Copy TypeScript definitions to main pkg directory
cp pkg-bundler/qed_user_prover_wasm.d.ts pkg/

echo "Build completed successfully!"
echo "Output directory: pkg/"
echo "Main files:"
echo "  - qed_user_prover_wasm.js (JavaScript bindings)"
echo "  - qed_user_prover_wasm_bg.wasm (WebAssembly binary)"
echo "  - qed_user_prover_wasm.d.ts (TypeScript definitions)"
echo "  - package.json (NPM package metadata)"

# Display file sizes
echo ""
echo "File sizes:"
ls -lh pkg/qed_user_prover_wasm_bg.wasm pkg/qed_user_prover_wasm.js

echo ""
echo "To use in a web project:"
echo "1. Copy the pkg/ directory to your web project"
echo "2. Import the module in your JavaScript/TypeScript code"
echo "3. Initialize the WASM module before use"