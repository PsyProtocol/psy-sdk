# PSY Config Package Makefile

.PHONY: build build-wasm build-wasm-web build-wasm-node test clean help

# Default target
help:
	@echo "Available targets:"
	@echo "  build           - Build the Rust library"
	@echo "  build-wasm      - Build both web and node WASM packages"
	@echo "  build-wasm-web  - Build WASM package for web (ES modules)"
	@echo "  build-wasm-node - Build WASM package for Node.js"
	@echo "  test            - Run tests"
	@echo "  clean           - Clean build artifacts"
	@echo ""
	@echo "Environment variables:"
	@echo "  PSY_CONFIG_PATH - Path to config.json file (default: auto-discover)"
	@echo "  PSY_NETWORK     - Network to use (default: localhost)"

# Build the regular Rust library
build:
	cargo build --release

# Build both WASM targets
build-wasm:
	@echo "[psy_config] WASM bundles are now produced via psy_rust_sdk bindings."
	@echo "Use \"make build-wasm\" under psy_sdk/psy-rust-sdk instead."

# Build WASM for web (ES modules, for browsers)
build-wasm-web:
	@echo "[psy_config] WASM bindings have moved to psy_rust_sdk."
	@echo "No action taken."

# Build WASM for Node.js
build-wasm-node:
	@echo "[psy_config] WASM bindings have moved to psy_rust_sdk."
	@echo "No action taken."

# Run tests
test:
	cargo test

# Test with specific network
test-network:
	PSY_NETWORK=testnet cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -rf pkg/

# Example: Build with custom config and network
build-custom:
	PSY_CONFIG_PATH=./example-config.json PSY_NETWORK=mainnet cargo build --release

# Example: Build WASM with custom config
build-wasm-custom:
	@echo "[psy_config] WASM bindings have moved to psy_rust_sdk."
