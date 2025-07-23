export DARGO_STD_PATH := $(PWD)/qed_compiler/qed-std/std.qed

PROFILE                  := release
LOG_LEVEL                := tikv_client=debug,qed_store=debug, qed_user_cli=debug,qed_dev_cli=debug,qed_rollup_cli=debug,qed_node=debug,qed_common_circuit=debug,qed_rollup_circuit=debug,qed_prover=debug,qed_data=debug,plonky2=error

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
USER32_0_PRIVATE_KEY  := f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d

USER1_PRIVATE_KEY        := 73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268
USER32_1_PRIVATE_KEY  := 88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d

CURRENT_USER_PRIVATE_KEY := ${USER0_PRIVATE_KEY}

CHECKPOINT_ID            := 1
LEAF_CHECKPOINT_ID       := ${CHECKPOINT_ID}
USER_ID                  := 0
CONTRACT_ID              := 0
SLOT_ID                  := 0
CONTRACT_STATE_HEIGHT    := 32
REALM_ID                 := 0
REGISTRATION_ID          := 1
STRATEGY                 := 2

COORDINATOR_RPC_URL      := $(shell jq -r '.coordinator_configs[].rpc_url[]' rpc.config)
REALM_RPC_URL            := $(shell jq -r '.realm_configs[0].rpc_url[]' rpc.config)

init:
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}
	@cp qed_compiler/tests/new_token.qed ${FILE}
	@mkdir -p $(PWD)/db
	@echo "Starting Redis containers..."
	@docker run -d --name qed-redis-coordinator -p 6379:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm0 -p 6380:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm32 -p 6381:6379 redis:alpine redis-server --save ""
	# @echo "Starting ScyllaDB containers..."
	# @docker run -d --name qed-scylla-coordinator -p 9042:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm0 -p 9043:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm32 -p 9044:9042 scylladb/scylla:latest
	@echo "Waiting for databases to be ready..."
	@sleep 10

.PHONY: shutdown
shutdown:
	@echo "Stopping and removing database containers..."
	@docker rm -f qed-redis-coordinator qed-redis-realm0 qed-redis-realm32 > /dev/null 2>&1 || true
	# @docker rm -f qed-scylla-coordinator qed-scylla-realm0 qed-scylla-realm32 > /dev/null 2>&1 || true
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
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor --backend-type lmdbx --path ${PWD}/db/coordinator

run-coordinator-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge --backend-type lmdbx --path ${PWD}/db/coordinator

run-coordinator-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-worker

run-realm-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor --redis-uri=redis://127.0.0.1:6380 --backend-type lmdbx --path ${PWD}/db/realm0

run-realm-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge --redis-uri=redis://127.0.0.1:6380 --backend-type lmdbx --path ${PWD}/db/realm0

run-realm-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker --redis-uri=redis://127.0.0.1:6380

run-realm-processor32:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
      --redis-uri=redis://127.0.0.1:6381 \
      --backend-type lmdbx \
      --path ${PWD}/db/realm32 \
      --node-id=2 \
      --realm-id=32 \
      --worker-queue-suffix=rwq32 \
      --notifications-queue-suffix=rnq32 \
      --proof-store-key-suffix=RP32

run-realm-edge32:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
      --listen-addr=0.0.0.0:8547 \
      --redis-uri=redis://127.0.0.1:6381 \
      --backend-type lmdbx \
      --path ${PWD}/db/realm32 \
      --coordinator-addr=http://127.0.0.1:8545 \
      --node-id=2 \
      --realm-id=32 \
      --worker-queue-suffix=rwq32 \
      --notifications-queue-suffix=rnq32 \
      --proof-store-key-suffix=RP32

run-realm-worker32:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker \
      --redis-uri=redis://127.0.0.1:6381 \
      --worker-queue-suffix=rwq32 \
      --notifications-queue-suffix=rnq32 \
      --proof-store-key-suffix=RP32


TIKV_PD_ENDPOINTS := 127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383

init-tikv:
	@echo "Starting TiKV cluster..."
	@docker-compose -f ./scripts/docker-compose.tikv.yml up -d
	@echo "Waiting for TiKV to be ready..."
	@sleep 30
	@echo "TiKV cluster is ready"

shutdown-tikv:
	@echo "Stopping TiKV cluster..."
	@docker-compose -f ./scripts/docker-compose.tikv.yml down -v
	@echo "TiKV cluster stopped"

run-coordinator-processor-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor \
		--backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace coordinator

run-coordinator-edge-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge \
		--backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace coordinator

run-realm-processor-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
		--redis-uri=redis://127.0.0.1:6380 \
		--backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm0

run-realm-edge-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
		--redis-uri=redis://127.0.0.1:6380 \
		--backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm0

run-realm-processor32-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
		--redis-uri=redis://127.0.0.1:6381 \
		--backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm32 \
		--node-id=2 \
		--realm-id=32 \
		--worker-queue-suffix=rwq32 \
		--notifications-queue-suffix=rnq32 \
		--proof-store-key-suffix=RP32

run-realm-edge32-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
		--listen-addr=0.0.0.0:8547 \
        --redis-uri=redis://127.0.0.1:6381 \
        --backend-type tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm32 \
        --coordinator-addr=http://127.0.0.1:8545 \
		--node-id=2 \
		--realm-id=32 \
		--worker-queue-suffix=rwq32 \
		--notifications-queue-suffix=rnq32 \
		--proof-store-key-suffix=RP32

run-all-tikv: shutdown-tikv init-tikv
	@./scripts/run_all_tikv.sh

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
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
      -H "Content-Type: application/json" \
      -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "public_key": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "352637524d9b8482d65b9c8bc78d0d4849a063bc53558158f84ee3863081ab4b" } }, "id": 1 }' | jq .
	@sleep 0.5
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
	     -H "Content-Type: application/json" \
         -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "public_key": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "cad421940097e1a1257a0d85faf9441d6e52d17f2dcda0da6da5c3a4ea80fe15" } }, "id": 1 }' | jq .

register-user2:
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
      -H "Content-Type: application/json" \
      -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { { "public_key": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "948eecedbc5579156b0ba347124538e2f1beb430f86615d656cea54bfc20a4b3" }  }, "id": 1 }' | jq .
	@sleep 0.5
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
	     -H "Content-Type: application/json" \
         -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "public_key": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "e002b20332ebaabb07f0c1acd1d209558115796bddc1b407ee2e67f55b71c42e" }  }, "id": 1 }' | jq .

random-register-user-batch:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-register-user-batch --total-user $(TOTAL_USER)

deploy-contract:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${CURRENT_USER_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/target/examples.json
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${USER32_0_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/target/examples.json

mint:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${CURRENT_USER_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000

transfer:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${CURRENT_USER_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 134217728 --inputs 500

claim:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER32_0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 0

return-back:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER32_0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 500

mint2:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000

transfer2:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 500

claim2:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 1

return-back2:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 1 --inputs 500

claim3:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 4194304

return-back3:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 4194304 --inputs 500

balance-of:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_contract_state_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${USER_ID}, ${CONTRACT_ID}, ${CONTRACT_STATE_HEIGHT}, ${SLOT_ID}], "id": 1 }' | jq .

build-block:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_build_block", "params": [], "id": 1 }' | jq .

latest-checkpoint:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_latest_checkpoint", "params": [], "id": 1 }' | jq .

get-contract-leaf-data:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_contract_leaf_data", "params": [${CONTRACT_ID}], "id": 1 }' | jq .

qed-get-checkpoint-leaf-data:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_leaf_data", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

get-checkpoint-global-state-roots:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_global_state_roots", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

qed-get-checkpoint-tree-root:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

qed-get-latest-checkpoint-tree-root:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_latest_checkpoint_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

qed-get-checkpoint-tree-leaf-hash:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_leaf_hash", "params": [${CHECKPOINT_ID}, ${LEAF_CHECKPOINT_ID}], "id": 1 }' | jq .

qed-get-checkpoint-tree-merkle-proof:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${LEAF_CHECKPOINT_ID}], "id": 1 }' | jq .

qed-get-contract-code-definition:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_contract_code_definition", "params": [${CONTRACT_ID}], "id": 1 }' | jq .

get-latest-l2-block-state:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_latest_l2_block_state", "params": [], "id": 1 }' | jq .

get-l2-block-state:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_l2_block_state", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

get-user-leaf-data:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_leaf_data", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

get-realm-user-tree-root:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

get-realm-user-tree-merkle-proof:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_sub_tree_merkle_proof", "params": [${CHECKPOINT_ID}, 0, 15, ${REALM_ID}], "id": 1 }' | jq .

get-user-tree-merkle-proof:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-check-user-id:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_check_user_id_in_realm", "params": [${USER_ID}], "id": 1 }' | jq .

realm-get-latest-l2-block-state:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_latest_l2_block_state", "params": [], "id": 1 }' | jq .

realm-get-l2-block-state:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_l2_block_state", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-checkpoint-leaf-data:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_leaf_data", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-latest-checkpoint-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_latest_checkpoint_tree_root", "params": [], "id": 1 }' | jq .

realm-checkpoint-global-state-roots:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_global_state_roots", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-checkpoint-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-checkpoint-tree-leaf-hash:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_leaf_hash", "params": [${CHECKPOINT_ID}, ${LEAF_CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-checkpoint-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${LEAF_CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-user-leaf-data:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_leaf_data", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-leaf-hash:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_tree_leaf_hash", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-realm-user-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_sub_tree_merkle_proof", "params": [${CHECKPOINT_ID}, 15, 30, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-registration-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_registration_tree_root", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

realm-get-user-bottom-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_bottom_tree_merkle_proof", "params": [15, ${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-contract-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_contract_tree_root", "params": [${CHECKPOINT_ID}, ${USER_ID}], "id": 1 }' | jq .

realm-get-user-contract-state-tree-root:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_contract_state_tree_root", "params": [${CHECKPOINT_ID}, ${USER_ID}, ${CONTRACT_ID}], "id": 1 }' | jq .

realm-get-user-contract-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_contract_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${USER_ID}, ${CONTRACT_ID}], "id": 1 }' | jq .

realm-get-user-contract-state-tree-merkle-proof:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_user_contract_state_tree_merkle_proof", "params": [${CHECKPOINT_ID}, ${USER_ID}, ${CONTRACT_ID}, ${CONTRACT_STATE_HEIGHT}, ${SLOT_ID}], "id": 1 }' | jq .

realm-get-checkpoint-global-state-roots:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_get_checkpoint_global_state_roots", "params": [${CHECKPOINT_ID}], "id": 1 }' | jq .

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
