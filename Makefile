export DARGO_STD_PATH := $(PWD)/qed_compiler/qed-std/std.qed

PROFILE                  := release
LOG_LEVEL                := qed_user_cli=debug,qed_dev_cli=debug,qed_rollup_cli=debug,qed_node=debug,qed_common_circuit=debug,qed_rollup_circuit=debug,qed_prover=debug,qed_data=debug,plonky2=error

default: build-release wasm-build

build-release:
	@RUSTFLAGS="-A warnings"  cargo build --release

check:
	@cargo check --all-targets --examples

fix:
	# @cargo machete --fix
	@cargo fix --all-targets --allow-dirty --allow-staged

build: config_gen_v2
	@cargo build --profile ${PROFILE} --bin qed_user_cli --bin qed_rollup_cli --bin qed_dev_cli --bin dargo --bin qed-lsp-server

fmt:
	@cargo fmt

clean:
	@rm -r target

DARGO_CLI_COMPILE = RUST_LOG=$(LOG_LEVEL) cd qed_compiler/tests && ../../target/${PROFILE}/dargo compile --debug --entry-path
DARGO_CLI_EXECUTE = RUST_LOG=${LOG_LEVEL} cd qed_compiler/tests && ../../target/${PROFILE}/dargo execute --debug --entry-path

ci:
	@RUST_LOG=${LOG_LEVEL} cargo test --profile ${PROFILE} \
        --package qed-ast \
        --package qed-parser \
        --package qed-sema \
        --package qed-interpreter \
        -- \
        --nocapture
	@RUST_LOG=${LOG_LEVEL} cargo run --profile ${PROFILE} --package dargo test --file qed_compiler/tests/in_mod_attr_test.qed
	@RUST_LOG=${LOG_LEVEL} cargo run --profile ${PROFILE} --package dargo test --file qed_compiler/tests/should_panic_test.qed

	@$(DARGO_CLI_COMPILE) ctx_test.qed
	@$(DARGO_CLI_COMPILE) storage_test.qed --contract-name=SimpleContract --method-names set_a set_b set_c set_d get_a get_b get_c get_d
	@$(DARGO_CLI_COMPILE) basic_ups.qed --contract-name=Contract --method-names simple_mint simple_transfer simple_claim
	@$(DARGO_CLI_COMPILE) token.qed --contract-name=ContractRef --method-names simple_mint simple_transfer simple_claim
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
	@$(DARGO_CLI_EXECUTE) token.qed --contract-name=ContractRef --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100
	@$(DARGO_CLI_EXECUTE) two_user_ups.qed --contract-name=Contract --method-names=simple_mint --method-names=simple_transfer --parameters 1000 --parameters 2,100

update-snapshots:
	@cargo insta review

WATCHED_DIRS := qed_rollup_circuit qed_common_circuit qed_prover/src/dpn qed_prover/src/ups qed_core/src/config/network_constants.rs qed_crypto/src/common/user_id.rs

config_gen_v2:
	@if git diff --name-only --diff-filter=M | grep -q -E "$(subst $() $(),|,$(WATCHED_DIRS)).*\.rs$$"; then \
		echo "Changes detected in watched directories. Running config_gen_v2..."; \
		RUST_LOG=${RUST_LOG} cargo run --profile ${PROFILE} --package qed_prover --example config_gen_v2; \
	else \
		echo "No changes detected in watched directories. Skipping config_gen_v2."; \
	fi

.PHONY: check fix build format run test update-snapshots

################################################################################
#                                   TMP                                        #
################################################################################
PROJECT_DIR              := $(PWD)/examples
FILE                     := $(PWD)/examples/src/main.qed
PARAMETERS               :=
USER0_PRIVATE_KEY        := 17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a
USER0_PUBLIC_KEY         := 6ee6d9596a34a5de293cb550d5d100d00b30487245777018677cc803345633c5
USER1_PRIVATE_KEY        := f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d
USER1_PUBLIC_KEY         := 0aa313de0677ed55f51cca7094b519d53d661f131f481a03e12e45f0f3389f12
USER2_PRIVATE_KEY        := 73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268
USER2_PUBLIC_KEY         := 3622af1955a3a547e7112ed381602a0dc8b30eaaf98d716342b2b9f941616382
USER3_PRIVATE_KEY        := 88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d
USER3_PUBLIC_KEY         := cc2ddec960c6c9529befb8746b3b53a09f5e63a5b5868b69654d740017726f1f

CURRENT_USER_PRIVATE_KEY := ${USER0_PRIVATE_KEY}
CURRENT_USER_PUBLIC_KEY  := ${USER0_PUBLIC_KEY}

CHECKPOINT_ID            := 0
LEAF_CHECKPOINT_ID       := ${CHECKPOINT_ID}
USER_ID                  := 0
CONTRACT_ID              := 0
SLOT_ID                  := 0
CONTRACT_STATE_HEIGHT    := 32
REALM_ID                 := 0
REGISTRATION_ID          := 1
STRATEGY                 := 2

COORDINATOR_RPC_URL      := $(shell jq -r '.network.coordinator_configs[].rpc_url[]' config.json)
REALM_RPC_URL            := $(shell jq -r '.network.realm_configs[0].rpc_url[]' config.json)

init:
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}
	@cp qed_compiler/tests/new_token.qed ${FILE}
	@mkdir -p $(PWD)/db
	@echo "Starting Redis containers..."
	@docker run -d --name qed-redis-coordinator -p 6379:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm0 -p 6380:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm1 -p 6381:6379 redis:alpine redis-server --save ""
	# @echo "Starting ScyllaDB containers..."
	# @docker run -d --name qed-scylla-coordinator -p 9042:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm0 -p 9043:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm1 -p 9044:9042 scylladb/scylla:latest
	@echo "Waiting for databases to be ready..."
	@sleep 10

.PHONY: shutdown
shutdown:
	@echo "Stopping and removing database containers..."
	@docker rm -f qed-redis-coordinator qed-redis-realm0 qed-redis-realm1 > /dev/null 2>&1 || true
	# @docker rm -f qed-scylla-coordinator qed-scylla-realm0 qed-scylla-realm1 > /dev/null 2>&1 || true
	@rm -fr ${PROJECT_DIR} ${PWD}/db > /dev/null 2>&1 || true

run-all: shutdown init compile
	@./scripts/run_all.sh

run-scenario0:
	@./scripts/run_scenario0.sh

interpret:
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR} && ../target/${PROFILE}/dargo execute --debug --entry-path ${FILE} --parameters ${PARAMETERS}

compile:
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR} && ../target/${PROFILE}/dargo compile --entry-path ${FILE} --contract-name=ContractRef --method-names simple_mint simple_transfer simple_claim

run-coordinator-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor --database lmdbx --lmdbx-path ${PWD}/db/coordinator

run-coordinator-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge --database lmdbx --lmdbx-path ${PWD}/db/coordinator

run-coordinator-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-worker --edge-url=http://127.0.0.1:8545

run-realm-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor --redis-uri=redis://127.0.0.1:6380 --database lmdbx --lmdbx-path ${PWD}/db/realm0

run-realm-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge --redis-uri=redis://127.0.0.1:6380 --database lmdbx --lmdbx-path ${PWD}/db/realm0

run-realm-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker --redis-uri=redis://127.0.0.1:6380 --edge-url=http://127.0.0.1:8546

run-realm-processor1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
      --redis-uri=redis://127.0.0.1:6381 \
      --database lmdbx \
      --lmdbx-path ${PWD}/db/realm1 \
      --node-id=2 \
      --realm-id=1 \
      --worker-queue-suffix=rwq1 \
      --notifications-queue-suffix=rnq1 \
      --proof-store-key-suffix=RP1

run-realm-edge1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
      --listen-addr=0.0.0.0:8547 \
      --redis-uri=redis://127.0.0.1:6381 \
      --database lmdbx \
      --lmdbx-path ${PWD}/db/realm1 \
      --coordinator-addr=http://127.0.0.1:8545 \
      --node-id=2 \
      --realm-id=1 \
      --worker-queue-suffix=rwq1 \
      --notifications-queue-suffix=rnq1 \
      --proof-store-key-suffix=RP1

run-realm-worker1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker \
      --redis-uri=redis://127.0.0.1:6381 \
      --worker-queue-suffix=rwq1 \
      --notifications-queue-suffix=rnq1 \
      --proof-store-key-suffix=RP1 \
      --edge-url=http://127.0.0.1:8547

run-user-prover:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli local-prover

run-web-wallet:
	@cd qed-ts-sdk/app/qed-wallet && pnpm i && pnpm run dev

generate-access-token:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli generate-access-token

get-public-key:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli get-public-key --private-key=${CURRENT_USER_PRIVATE_KEY}

random-wallet:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-wallet

register-user:
	@echo "Registering all 4 users..."
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER0_PRIVATE_KEY} | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER1_PRIVATE_KEY} | tail -5 | jq .
	@sleep 0.5
	# @RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER2_PRIVATE_KEY} | tail -5 | jq .
	# @sleep 0.5
	# @RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER3_PRIVATE_KEY} | tail -5 | jq .

random-register-user-batch:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-register-user-batch --total-user $(TOTAL_USER)

deploy-contract:
	@echo "Deploying contracts..."
	@echo "USER0 deploying contract 0..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${USER0_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/target/examples.json
	@echo "USER1 deploying contract 1..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${USER1_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/target/examples.json

mint:
	@echo "All users minting 1000 tokens..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000
	# @RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER2_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000
	# @RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER3_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000

transfer:
	@echo "USER0 transferring 250 to USER1..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 8388608 --inputs 250

claim:
	@echo "USER1 claiming transfer..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 0

return-back:
	@echo "USER1 transferring back to USER0..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 250

balance-of:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID} --height ${CONTRACT_STATE_HEIGHT} --leaf-id ${SLOT_ID}

build-block:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_build_block", "params": [], "id": 1 }' | jq .

latest-checkpoint:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_latest_checkpoint", "params": [], "id": 1 }' | jq .

get-contract-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-contract-leaf-data --contract-id ${CONTRACT_ID}

qed-get-checkpoint-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-leaf-data --checkpoint-id ${CHECKPOINT_ID}

get-checkpoint-global-state-roots:
	@echo "Note: qed_get_checkpoint_global_state_roots is not implemented in the CLI yet"

qed-get-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-root --checkpoint-id ${CHECKPOINT_ID}

qed-get-latest-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-latest-checkpoint-tree-root

qed-get-checkpoint-tree-leaf-hash:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-leaf-hash --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

qed-get-checkpoint-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

qed-get-contract-code-definition:
	@./target/${PROFILE}/qed_user_cli get-contract-code-definition --contract-id ${CONTRACT_ID}

get-latest-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-latest-l2-block-state

get-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-l2-block-state --checkpoint-id ${CHECKPOINT_ID}

get-user-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-user-leaf --checkpoint-id ${CHECKPOINT_ID} --pub-key ${CURRENT_USER_PUBLIC_KEY}

get-realm-user-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-tree-root --checkpoint-id ${CHECKPOINT_ID}

get-realm-user-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-sub-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --root-level 0 --leaf-level 15 --leaf-index ${REALM_ID}

get-user-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

realm-check-user-id:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_check_user_id_in_realm", "params": [${USER_ID}], "id": 1 }' | jq .

realm-get-latest-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-latest-l2-block-state

realm-get-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-l2-block-state --checkpoint-id ${CHECKPOINT_ID}

realm-get-checkpoint-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-leaf-data --checkpoint-id ${CHECKPOINT_ID}

realm-get-latest-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-latest-checkpoint-tree-root

realm-checkpoint-global-state-roots:
	@echo "Note: qed_get_checkpoint_global_state_roots is not implemented in the CLI yet"

realm-get-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-root --checkpoint-id ${CHECKPOINT_ID}

realm-get-checkpoint-tree-leaf-hash:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-leaf-hash --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

realm-get-checkpoint-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

realm-get-user-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-user-leaf --checkpoint-id ${CHECKPOINT_ID} --pub-key 3622af1955a3a547e7112ed381602a0dc8b30eaaf98d716342b2b9f941616382

realm-get-user-leaf-hash:
	@./target/${PROFILE}/qed_user_cli get-user-tree-leaf-hash --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

realm-get-user-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-tree-root --checkpoint-id ${CHECKPOINT_ID}

realm-get-realm-user-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-sub-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --root-level 15 --leaf-level 30 --leaf-index ${USER_ID}

realm-get-user-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

realm-get-user-registration-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-registration-tree-root --checkpoint-id ${CHECKPOINT_ID}

realm-get-user-bottom-tree-merkle-proof:
	@echo "Note: qed_get_user_bottom_tree_merkle_proof is not implemented in the CLI yet"

realm-get-user-contract-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-contract-tree-root --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

realm-get-user-contract-state-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-root --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID}

realm-get-user-contract-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-contract-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID}

realm-get-user-contract-state-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID} --height ${CONTRACT_STATE_HEIGHT} --leaf-id ${SLOT_ID}

realm-get-checkpoint-global-state-roots:
	@echo "Note: qed_get_checkpoint_global_state_roots is not implemented in the CLI yet"

get-user-id-from-registration-id:
	@./target/${PROFILE}/qed_dev_cli get-user-id-from-registration-id ${REGISTRATION_ID} --strategy ${STRATEGY}

image:
	docker build \
		-c 512 \
		-t qedprotocol/qed-rollup:latest \
		-f Dockerfile .

wasm-build:
	@cd qed_prover && wasm-pack build --target web --out-dir ../qed-ts-sdk/packages/qed-sdk/src/local-web-prover --no-pack --release --no-default-features
	@cd qed_prover && wasm-pack build --target nodejs --out-dir ../qed-ts-sdk/packages/qed-sdk/src/local-prover  --no-pack --release --no-default-features

help:
	@grep -E '^[a-zA-Z_-]+:.*?' Makefile | cut -d: -f1 | sort
