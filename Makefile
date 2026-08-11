check:
	@cargo check --workspace --all-targets --tests --benches --examples --bins
	@cd psy-ts-sdk && pnpm install && pnpm --filter @psy-protocol/psy-sdk run type-check

build:
	@$(MAKE) wasm-build
	@cd psy-ts-sdk && pnpm install && pnpm --filter @psy-protocol/psy-sdk run build:wasm
	@cd psy-ts-sdk && pnpm install && pnpm --filter @psy-protocol/psy-sdk run build

wasm-build:
	@cd psy-rust-sdk && wasm-pack build --target web --out-dir ../psy-ts-sdk/packages/psy-sdk/src/local-web-prover --out-name psy_prover --no-pack --release
	@cp .github/templates/.gitignore.wasm ./psy-ts-sdk/packages/psy-sdk/src/local-web-prover/.gitignore
	@cd ../psy-compiler && wasm-pack build psy-wasm --target web --out-dir $(PWD)/psy-ts-sdk/packages/psy-sdk/src/local-web-compiler --out-name psy_compiler --no-pack --release
	@cp .github/templates/.gitignore.wasm ./psy-ts-sdk/packages/psy-sdk/src/local-web-compiler/.gitignore
