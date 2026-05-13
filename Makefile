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
