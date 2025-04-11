export DARGO_STD_PATH := $(PWD)/qed-std/std.qed

PROFILE                 := release
LOG_LEVE                := info
FILE                    := tests/trait_test.qed
PARAMETERS              := 2,3

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

DARGO_CLI_COMPILE = RUST_LOG=$(LOG_LEVEL) cd tests && cargo run --release --package dargo compile --debug --entry-path
compile:
	@$(DARGO_CLI_COMPILE) ctx_test.qed
	@$(DARGO_CLI_COMPILE) storage_test.qed --contract-name=Contract --method-names set_a set_b set_c set_d get_a get_b get_c get_d
	@$(DARGO_CLI_COMPILE) basic_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim
	@$(DARGO_CLI_COMPILE) token.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim
	@$(DARGO_CLI_COMPILE) two_user_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

DARGO_CLI_EXECUTE = RUST_LOG=${LOG_LEVE} cd tests && cargo run --release --package dargo execute --debug --entry-path
test:
	@RUST_LOG=${LOG_LEVE} cargo test --release -- --nocapture
	@$(DARGO_CLI_EXECUTE) basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 133700 --parameters 2,1000
	@$(DARGO_CLI_EXECUTE) basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters=2,100
	@$(DARGO_CLI_EXECUTE) token.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100
	@$(DARGO_CLI_EXECUTE) two_user_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100

update-snapshots:
	@cargo insta review

.PHONE: check fix build format run test update-snapshots
