PROFILE                 := release
LOG_LEVE                := info
FILE                    := tests/storage_test.qed
PARAMETERS              :=

check:
	@cargo check --all-targets --examples

fix:
	# @cargo machete --fix
	@cargo fix --all-targets --allow-dirty --allow-staged

build:
	@cargo build --profile ${PROFILE}

fmt:
	@cargo fmt

interpret:
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file ${FILE} --parameters ${PARAMETERS}

compile:
	@RUST_LOG=${LOG_LEVE} cd tests && \
	cargo run --release --package qed-dargo-cli compile --debug --entry-path ctx_test.qed && \
    cargo run --release --package qed-dargo-cli compile --debug --entry-path storage_test.qed --contract-name=Contract --method-names set_a set_b set_c set_d get_a get_b get_c get_d && \
    cargo run --release --package qed-dargo-cli compile --debug --entry-path basic_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim && \
    cargo run --release --package qed-dargo-cli compile --debug --entry-path token.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim && \
    cargo run --release --package qed-dargo-cli compile --debug --entry-path two_user_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

test:
	@RUST_LOG=${LOG_LEVE} cargo test --release -- --nocapture
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-dargo-cli test --file tests/test.qed

	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path assert_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path ctx_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path inline_module_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path opcode_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path parameter_passing_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path pub_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path return_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path self_test.qed
	# @RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-clexecute i --debug --entry-path storage_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path trait_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path hash_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path first_class_function_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path type_alias_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path const_test.qed --parameters 1
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path while_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path for_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path lambda_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path generics_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path type_hint_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path exp_test.qed --parameters 2,3
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path array_test.qed --parameters 1,1
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path u32_test.qed --parameters 2,3
	# @RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-clexecute i --debug --entry-path enum_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path tuple_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path ambiguity_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path match_test.qed --parameters 100
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path if_test.qed
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path block_test.qed

	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 133700 --parameters 2,1000
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters=2,100
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path token.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100
	@RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package qed-dargo-cli execute --debug --entry-path two_user_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100

update-snapshots:
	@cargo insta review

.PHONE: check fix build format run test update-snapshots
