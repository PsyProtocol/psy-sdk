PSY_COMPILER_WASM_WORKSPACE ?= ../parth-generic-v1/client_prover/psy_ide
PSY_COMPILER_WASM_CRATE ?= psy_wasm

check:
	@cargo check --workspace --all-targets --tests --benches --examples --bins
	@cd psy-ts-sdk/packages/psy-sdk && pnpm install && pnpm type-check

build:
	@$(MAKE) wasm-build
	@cd psy-ts-sdk/packages/psy-sdk && pnpm install && pnpm run build:wasm
	@cd psy-ts-sdk/packages/psy-sdk && pnpm install && pnpm run build

wasm-build:
	@cd psy-rust-sdk && wasm-pack build --target web --out-dir ../psy-ts-sdk/packages/psy-sdk/src/local-web-prover --out-name psy_prover --no-pack --release
	@cp .github/templates/.gitignore.wasm ./psy-ts-sdk/packages/psy-sdk/src/local-web-prover/.gitignore
	@cd $(PSY_COMPILER_WASM_WORKSPACE) && wasm-pack build $(PSY_COMPILER_WASM_CRATE) --target web --out-dir $(PWD)/psy-ts-sdk/packages/psy-sdk/src/local-web-compiler --out-name psy_compiler --no-pack --release
	@cp .github/templates/.gitignore.wasm ./psy-ts-sdk/packages/psy-sdk/src/local-web-compiler/.gitignore
