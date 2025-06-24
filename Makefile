export DARGO_STD_PATH := $(PWD)/qed_compiler/qed-std/std.qed

PROFILE                  := release
LOG_LEVEL                := qed_user_prover=info,qed_user_cli=debug,qed_rollup_cli=debug,qed_realm_node=debug,qed_coordinator_node=debug,qed_node=debug,qed_common_circuit=debug,qed_rollup_circuit=debug,qed_prover=debug,qed_data=debug,plonky2=error

check:
	@cargo check --all-targets --examples

fix:
	# @cargo machete --fix
	@cargo fix --all-targets --allow-dirty --allow-staged

build: common_config_generator
	@cargo build --profile ${PROFILE} --bin qed_user_cli --bin qed_rollup_cli --bin dargo --bin qed_user_prover

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

WATCHED_DIRS := qed_rollup_circuit qed_common_circuit qed_prover qed_core/src/config/network_constants.rs qed_crypto/src/common/user_id.rs

common_config_generator:
	@if git diff --name-only --diff-filter=M | grep -q -E "$(subst $() $(),|,$(WATCHED_DIRS)).*\.rs$$"; then \
		echo "Changes detected in watched directories. Running common_config_generator..."; \
		RUST_LOG=${RUST_LOG} cargo run --profile ${PROFILE} --package qed_prover --example config_gen_v2; \
	else \
		echo "No changes detected in watched directories. Skipping common_config_generator."; \
	fi

.PHONY: check fix build format run test update-snapshots

################################################################################
#                                   TMP                                        #
################################################################################
PROJECT_DIR              := $(PWD)/examples
FILE                     := $(PWD)/examples/src/main.qed
PARAMETERS               :=
USER0_PRIVATE_KEY        := 17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a
USER16384_0_PRIVATE_KEY  := f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d

USER1_PRIVATE_KEY        := 73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268
USER16384_1_PRIVATE_KEY  := 88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d

CURRENT_USER_PRIVATE_KEY := ${USER0_PRIVATE_KEY}

CHECKPOINT_ID            := 1
LEAF_CHECKPOINT_ID       := ${CHECKPOINT_ID}
USER_ID                  := 0
CONTRACT_ID              := 0
SLOT_ID                  := 0
CONTRACT_STATE_HEIGHT    := 32
REALM_ID                 := 0

COORDINATOR_RPC_URL      := $(shell jq -r '.coordinator_configs[].rpc_url[]' rpc.config)
REALM_RPC_URL            := $(shell jq -r '.realm_configs[0].rpc_url[]' rpc.config)

init:
	@mkdir -p $(PWD)/db
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}
	@cp qed_compiler/tests/new_token.qed ${FILE}

.PHONY: launch
launch: shutdown init compile

.PHONY: shutdown
shutdown:
	@sudo rm -fr redis-data
	@redis-cli 'FLUSHALL' > /dev/null 2>&1 || true
	@redis-cli -u redis://127.0.0.1:6380 'FLUSHALL' > /dev/null 2>&1 || true
	@redis-cli -u redis://127.0.0.1:6381 'FLUSHALL' > /dev/null 2>&1 || true
	@sudo rm -fr $(PWD)/db
	@rm -fr ${PROJECT_DIR}

run-all:
	@./scripts/run_all.sh

run-scenario0:
	@./scripts/run_scenario0.sh

interpret:
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR} && ../target/${PROFILE}/dargo execute --debug --entry-path ${FILE} --parameters ${PARAMETERS}

compile:
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR} && ../target/${PROFILE}/dargo compile --entry-path ${FILE} --contract-name=ContractRef --method-names simple_mint simple_transfer simple_claim

run-coordinator-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor

run-coordinator-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge

run-coordinator-edge-1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge --listen-addr=0.0.0.0:9545

run-coordinator-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-worker

run-realm-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor --redis-uri=redis://127.0.0.1:6380

run-realm-worker:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker --redis-uri=redis://127.0.0.1:6380

run-realm-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge --redis-uri=redis://127.0.0.1:6380

run-realm-edge-1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge --redis-uri=redis://127.0.0.1:6380 --listen-addr=0.0.0.0:9546

run-realm-processor16384:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
      --redis-uri=redis://127.0.0.1:6381 \
      --node-id=2 \
      --realm-id=16384 \
      --worker-queue-suffix=rwq16384 \
      --notifications-queue-suffix=rnq16384 \
      --proof-store-key-suffix=RP16384 \
      --path=./db/realm16384

run-realm-worker16384:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker \
      --redis-uri=redis://127.0.0.1:6381 \
      --worker-queue-suffix=rwq16384 \
      --notifications-queue-suffix=rnq16384 \
      --proof-store-key-suffix=RP16384

run-realm-edge16384:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
      --listen-addr=0.0.0.0:8547 \
      --redis-uri=redis://127.0.0.1:6381 \
      --coordinator-addr=http://127.0.0.1:8545 \
      --node-id=2 \
      --realm-id=16384 \
      --worker-queue-suffix=rwq16384 \
      --notifications-queue-suffix=rnq16384 \
      --proof-store-key-suffix=RP16384 \
      --path=./db/realm16384

run-realm-edge16384-1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
      --listen-addr=0.0.0.0:9547 \
      --redis-uri=redis://127.0.0.1:6381 \
      --coordinator-addr=http://127.0.0.1:8545 \
      --node-id=2 \
      --realm-id=16384 \
      --worker-queue-suffix=rwq16384 \
      --notifications-queue-suffix=rnq16384 \
      --proof-store-key-suffix=RP16384 \
      --path=./db/realm16384

run-user-prover:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_prover

run-web-wallet:
	@cd qed-ts-sdk/app/qed-wallet && pnpm i && pnpm run dev

# run-realm-processor8192:
# 	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
#       --redis-uri=redis://127.0.0.1:6382 \
#       --node-id=2 \
#       --realm-id=8192 \
#       --worker-queue-suffix=rwq8192 \
#       --notifications-queue-suffix=rnq8192 \
#       --proof-store-key-suffix=RP8192 \
#       --path=./db/realm8192

# run-realm-worker8192:
# 	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-worker \
#       --redis-uri=redis://127.0.0.1:6382 \
#       --worker-queue-suffix=rwq8192 \
#       --notifications-queue-suffix=rnq8192 \
#       --proof-store-key-suffix=RP8192

# run-realm-edge8192:
# 	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
#       --listen-addr=0.0.0.0:8548 \
#       --redis-uri=redis://127.0.0.1:6382 \
#       --coordinator-addr=http://127.0.0.1:8545 \
#       --node-id=2 \
#       --realm-id=8192 \
#       --worker-queue-suffix=rwq8192 \
#       --notifications-queue-suffix=rnq8192 \
#       --proof-store-key-suffix=RP8192 \
#       --path=./db/realm8192

generate-access-token:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli generate-access-token

get-public-key:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli get-public-key --private-key=${CURRENT_USER_PRIVATE_KEY}

random-wallet:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-wallet

register-user:
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
      -H "Content-Type: application/json" \
      -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "352637524d9b8482d65b9c8bc78d0d4849a063bc53558158f84ee3863081ab4b" }, "id": 1 }' | jq .
	@sleep 0.5
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
	     -H "Content-Type: application/json" \
	     -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "cad421940097e1a1257a0d85faf9441d6e52d17f2dcda0da6da5c3a4ea80fe15" }, "id": 1 }' | jq .

register-user2:
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
      -H "Content-Type: application/json" \
      -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "948eecedbc5579156b0ba347124538e2f1beb430f86615d656cea54bfc20a4b3" }, "id": 1 }' | jq .
	@sleep 0.5
	@RUST_LOG=${LOG_LEVEL} curl -X POST ${COORDINATOR_RPC_URL} \
	     -H "Content-Type: application/json" \
	     -d '{ "jsonrpc": "2.0", "method": "qed_register_user", "params": { "fingerprint": "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0", "public_key_param": "e002b20332ebaabb07f0c1acd1d209558115796bddc1b407ee2e67f55b71c42e" }, "id": 1 }' | jq .

random-register-user-batch:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-register-user-batch --total-user $(TOTAL_USER)

deploy-contract:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${CURRENT_USER_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/target/examples.json

mint:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${CURRENT_USER_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000

transfer:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${CURRENT_USER_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 536870912 --inputs 500

claim:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER16384_0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 0

return-back:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER16384_0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 500

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

image:
	docker build \
		-c 512 \
		-t qedprotocol/qed-rollup:latest \
		-f Dockerfile .

wasm-build:
	@cd qed_user_prover && wasm-pack build --target web --out-dir ../qed-ts-sdk/packages/qed-sdk/src/local-web-prover --no-pack --release --no-default-features --features wasm32
	@cd qed_user_prover && wasm-pack build --target nodejs --out-dir ../qed-ts-sdk/packages/qed-sdk/src/local-prover  --no-pack --release --no-default-features --features wasm32

help:
	@grep -E '^[a-zA-Z_-]+:.*?' Makefile | cut -d: -f1 | sort
