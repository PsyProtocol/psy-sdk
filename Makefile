export DARGO_STD_PATH := $(PWD)/qed_compiler/qed-std/std.qed

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

DARGO_CLI_COMPILE = RUST_LOG=$(LOG_LEVEL) cd qed_compiler/tests && cargo run --release --package dargo compile --debug --entry-path
DARGO_CLI_EXECUTE = RUST_LOG=${LOG_LEVE} cd qed_compiler/tests && cargo run --release --package dargo execute --debug --entry-path

ci:
	@RUST_LOG=${LOG_LEVE} cargo test --release --package qed-ast --package qed-parser --package qed-sema --package qed-interpreter -- --nocapture
	@RUST_LOG=${LOG_LEVE} cargo run --release --package dargo test --file qed_compiler/tests/in_mod_attr_test.qed
	@RUST_LOG=${LOG_LEVE} cargo run --release --package dargo test --file qed_compiler/tests/should_panic_test.qed

	@$(DARGO_CLI_COMPILE) ctx_test.qed
	@$(DARGO_CLI_COMPILE) storage_test.qed --contract-name=SimpleContract --method-names set_a set_b set_c set_d get_a get_b get_c get_d
	@$(DARGO_CLI_COMPILE) basic_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim
	@$(DARGO_CLI_COMPILE) token.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim
	@$(DARGO_CLI_COMPILE) two_user_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim

	@$(DARGO_CLI_EXECUTE) assert_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) ctx_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) inline_module_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) opcode_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) parameter_passing_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) pub_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) return_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) self_test.qed
	@$(DARGO_CLI_EXECUTE) storage_test.qed
	@$(DARGO_CLI_EXECUTE) trait_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) hash_test.qed
	@$(DARGO_CLI_EXECUTE) first_class_function_test.qed
	@$(DARGO_CLI_EXECUTE) type_alias_test.qed
	@$(DARGO_CLI_EXECUTE) const_test.qed --parameters 1
	@$(DARGO_CLI_EXECUTE) while_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) for_test.qed
	@$(DARGO_CLI_EXECUTE) lambda_test.qed
	@$(DARGO_CLI_EXECUTE) generics_test.qed
	@$(DARGO_CLI_EXECUTE) polymorphism.qed
	@$(DARGO_CLI_EXECUTE) type_hint_test.qed
	@$(DARGO_CLI_EXECUTE) exp_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) array_test.qed --parameters 1,1
	@$(DARGO_CLI_EXECUTE) u32_test.qed --parameters 2,3
	# @$(DARGO_CLI_EXECUTE) enum_test.qed
	@$(DARGO_CLI_EXECUTE) tuple_test.qed
	@$(DARGO_CLI_EXECUTE) ambiguity_test.qed
	@$(DARGO_CLI_EXECUTE) match_test.qed --parameters 100
	@$(DARGO_CLI_EXECUTE) if_test.qed
	@$(DARGO_CLI_EXECUTE) block_test.qed
	@$(DARGO_CLI_EXECUTE) path_test.qed
	@$(DARGO_CLI_EXECUTE) should_panic_test.qed --parameters 2,3
	@$(DARGO_CLI_EXECUTE) basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 133700 --parameters 2,1000
	@$(DARGO_CLI_EXECUTE) basic_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters=2,100
	@$(DARGO_CLI_EXECUTE) token.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100
	@$(DARGO_CLI_EXECUTE) two_user_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100

update-snapshots:
	@cargo insta review

################################################################################
#                                   TMP                                        #
################################################################################
COORDINATOR_DB_PATH     := $(PWD)/db/coordinator
REALM_DB_PATH           := $(PWD)/db/realm

PROJECT_DIR             := $(PWD)/examples
FILE                    := $(PWD)/examples/src/main.qed
PARAMETERS              :=

init:
	@mkdir $(PWD)/db
	@cd $(PWD)/db && cargo run --release --package dargo new ${PROJECT_DIR}
	@cp qed_compiler/tests/new_token.qed ${FILE}

cleanup:
	@redis-cli 'FLUSHALL'
	@rm -fr $(PWD)/db
	@rm -fr ${PROJECT_DIR}

interpret:
	@RUST_LOG=${LOG_LEVE} cd ${PROJECT_DIR} && cargo run --release --package dargo execute --debug --entry-path ${FILE} --parameters ${PARAMETERS}

compile:
	@RUST_LOG=${LOG_LEVE} cd ${PROJECT_DIR} && cargo run --release --package dargo compile --entry-path ${FILE} --contract-name=UserStateRef --method-names mint transfer claim

run-coordinator-processor:
	@RUST_LOG=${LOG_LEVE} cargo run --package qed_coordinator_node --bin qed_coordinator_node --release processor --coordinator-db-path ${COORDINATOR_DB_PATH}

run-coordinator-edge:
	@RUST_LOG=${LOG_LEVE} cargo run --package qed_coordinator_edge --release processor --coordinator-db-path ${COORDINATOR_DB_PATH}

run-realm-processor:
	@RUST_LOG=${LOG_LEVE} cargo run -r --package qed_realm_node processor

run-realm-edge:
	@RUST_LOG=${LOG_LEVE} cargo run -r --package qed_realm_node edge

run-worker:
	@RUST_LOG=${LOG_LEVE} cargo run --package qed_coordinator_node --bin qed_coordinator_node  --release worker

.PHONE: check fix build format run test update-snapshots
