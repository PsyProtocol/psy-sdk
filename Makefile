PROFILE                 := release
LOG_LEVE                := info

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
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/opcode_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/parameter_passing_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/pub_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/return_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/001.qed
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/003.qed
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/004.qed
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/005.qed
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/006.qed
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/basic_ups.qed --contract-name=Contract --method-name=simple_mint --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/basic_ups.qed --contract-name=Contract --method-name=simple_transfer --params 2 3
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/basic_ups.qed --contract-name=Contract --method-name=simple_claim --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/token.qed --contract-name=Contract --method-name=simple_mint --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/token.qed --contract-name=Contract --method-name=simple_transfer --params 2 3
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/token.qed --contract-name=Contract --method-name=simple_claim --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/two_user_ups.qed --contract-name=Contract --method-name=simple_mint --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/two_user_ups.qed --contract-name=Contract --method-name=simple_transfer --params 2 3
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/two_user_ups.qed --contract-name=Contract --method-name=simple_claim --params 2
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/storage_test.qed
	# @RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli interpret --file tests/ctx_test.qed --params 2 3

compile:
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli compile --file tests/storage_test.qed --contract-name=Contract --method-names set_a set_b set_c set_d get_a get_b get_c get_d

	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli compile --file tests/basic_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli compile --file tests/token.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli compile --file tests/two_user_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

test:
	@RUST_LOG=${LOG_LEVE} cargo test --release -- --nocapture
	@RUST_LOG=${LOG_LEVE} cargo run --release --package qed-cli test --file tests/test.qed

update-snapshots:
	@cargo insta review

.PHONE: check fix build format run test update-snapshots
