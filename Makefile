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
	@RUST_LOG=${LOG_LEVE} cargo run --package qed-cli interpret --file tests/opcode_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --package qed-cli interpret --file tests/parameter_passing_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --package qed-cli interpret --file tests/pub_test.qed --params 2 3
	@RUST_LOG=${LOG_LEVE} cargo run --package qed-cli interpret --file tests/return_test.qed --params 2 3

compile:
	@RUST_LOG=${LOG_LEVE} cargo run --package qed-cli compile --file tests/002.qed

test:
	@RUST_LOG=${LOG_LEVE} cargo test -- --nocapture

update-snapshots:
	@cargo insta review

.PHONE: check fix build format run test update-snapshots
