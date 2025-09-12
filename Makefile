export DARGO_STD_PATH := $(PWD)/qed_compiler/qed-std/std.qed
export SQLX_OFFLINE=true

PROFILE := release
LOG_LEVEL := qed_rollup_utils=trace,tikv_client=debug,qed_store=trace,qed_user_cli=debug,qed_dev_cli=debug,qed_api_services=info,qed_rollup_cli=debug,qed_node=trace,qed_common_circuit=trace,qed_rollup_circuit=trace,qed_prover=trace,qed_data=trace,plonky2=error

default: build wallet-build

check:
	@cargo check --workspace --all-targets --tests --benches --examples --bins

fix:
	# @cargo machete --fix
	@cargo fix --all-targets --allow-dirty --allow-staged

build: config_gen_v2
	@RUSTFLAGS="-A warnings" cargo build --profile ${PROFILE} -p qed_precompiles
	@RUSTFLAGS="-A warnings" cargo build --profile ${PROFILE} --bin qed_user_cli --bin qed_rollup_cli --bin qed_dev_cli --bin dargo --bin qed-lsp-server --bin qed_api_services

fmt:
	@cargo fmt

install:
	@cargo install --path qed_compiler/qed-dargo-cli
	@cargo install --path qed_compiler/qed-lsp-server
	@cargo install --path qed_user_cli
	@cargo install --path qed_rollup_cli
	@cargo install --path qed_dev_cli

clean:
	@rm -r target

DARGO_CLI_COMPILE = RUST_LOG=$(LOG_LEVEL) ./target/${PROFILE}/dargo compile --program-dir qed_compiler/tests --debug --entry-path
DARGO_CLI_EXECUTE = RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/dargo execute --program-dir qed_compiler/tests --debug --entry-path
DARGO_CLI_TEST    = RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/dargo test --file

ci:
	@$(DARGO_CLI_TEST) qed_compiler/tests/in_mod_attr_test.qed
	# @$(DARGO_CLI_TEST) qed_compiler/tests/should_panic_test.qed
	@$(DARGO_CLI_TEST) qed_compiler/tests/for_if_test.qed
	@$(DARGO_CLI_TEST) qed_compiler/tests/array_struct_modification_test.qed
	@$(DARGO_CLI_TEST) qed_compiler/tests/conditional_assert_test.qed
	@$(DARGO_CLI_TEST) qed_compiler/tests/guta_nullifier_calculation_test.qed
	@$(DARGO_CLI_TEST) qed_compiler/tests/root_calculation_test.qed

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
	@$(DARGO_CLI_EXECUTE) hash_two_to_one_test.qed
	@$(DARGO_CLI_EXECUTE) verify_proof_test.qed
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
	@$(DARGO_CLI_EXECUTE) check_secp_sign_test.qed
	@$(DARGO_CLI_EXECUTE) clear_entire_tree_test.qed

	@RUST_LOG=${LOG_LEVEL} cargo test --profile ${PROFILE} \
	       --package qed-ast \
	       --package qed-parser \
	       -- \
	       --nocapture

	@RUST_LOG=${LOG_LEVEL} cargo test --profile ${PROFILE} \
	       --package qed-sema \
	       --package qed-interpreter \
	       -- \
	       --nocapture

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
FILE                     := $(PWD)/qed_compiler/tests/opcode_test.qed
PARAMETERS               := 1,2
USER0_PRIVATE_KEY        := 17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a
USER0_PUBLIC_KEY         := 6ee6d9596a34a5de293cb550d5d100d00b30487245777018677cc803345633c5
USER0_SECP_ZK_PUBLIC_KEY := 49deab842acf3d26236419d4fce1b2cb01081aef55d4ef0e566f980e3890cf2f
USER1_PRIVATE_KEY        := f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d
USER1_PUBLIC_KEY         := 0aa313de0677ed55f51cca7094b519d53d661f131f481a03e12e45f0f3389f12
USER1_SECP_ZK_PUBLIC_KEY := ac85e11f5c8a53241502c4519567aa3f02d30b1639fc49bb94c1d61335197e1a
USER2_PRIVATE_KEY        := 73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268
USER2_PUBLIC_KEY         := 3622af1955a3a547e7112ed381602a0dc8b30eaaf98d716342b2b9f941616382
USER2_SECP_ZK_PUBLIC_KEY := 2b280f7354a6648e1d37499d211876d4f68d703c4dbf4dd8ccee032a4da5f220
USER3_PRIVATE_KEY        := 88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d
USER3_PUBLIC_KEY         := cc2ddec960c6c9529befb8746b3b53a09f5e63a5b5868b69654d740017726f1f
USER3_SECP_ZK_PUBLIC_KEY := 1e44a84db33d52243068ccfbf6519beb1e072d1128ee792fe62896a3338db2a4

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
SIGN_TYPE                := zk

COORDINATOR_RPC_URL      := $(shell jq -r '.network.coordinator_configs[].rpc_url[]' config.json)
REALM_RPC_URL            := $(shell jq -r '.network.realm_configs[${REALM_ID}].rpc_url[]' config.json)

GLOBAL_USER_TREE_HEIGHT  := $(shell jq -r '.network.global_user_tree_height' config.json)
REALM_USER_TREE_HEIGHT   := $(shell jq -r '.network.realm_user_tree_height' config.json)
REALM_TREE_LEAF_LEVEL    := $(shell echo $$(($(GLOBAL_USER_TREE_HEIGHT) - $(REALM_USER_TREE_HEIGHT))))

init:
	# Create main project directory
	@mkdir -p ${PROJECT_DIR}
	# Create token contract subdirectory
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}/token
	@cp qed_compiler/tests/new_token.qed ${PROJECT_DIR}/token/src/main.qed
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}/rewards
	@cp qed_compiler/tests/rewards.qed ${PROJECT_DIR}/rewards/src/main.qed
	@./target/${PROFILE}/dargo new ${PROJECT_DIR}/mining_rewards
	@cp qed_compiler/tests/mining_rewards.qed ${PROJECT_DIR}/mining_rewards/src/main.qed
	@mkdir -p $(PWD)/db
	@echo "Starting Redis containers..."
	@docker run -d --name qed-redis-coordinator -p 6379:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm0 -p 6380:6379 redis:alpine redis-server --save ""
	@docker run -d --name qed-redis-realm1 -p 6381:6379 redis:alpine redis-server --save ""
	@sleep 10
	# @echo "Starting ScyllaDB containers..."
	# @docker run -d --name qed-scylla-coordinator -p 9042:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm0 -p 9043:9042 scylladb/scylla:latest
	# @docker run -d --name qed-scylla-realm1 -p 9044:9042 scylladb/scylla:latest
	@echo "Waiting for databases to be ready..."
	@echo "Starting TimescaleDB container..."
	@docker run -d --name timescaledb -p 5432:5432 -e POSTGRES_PASSWORD=password timescale/timescaledb:latest-pg17
	@sleep 5
	@cd ./qed_api_services && export DATABASE_URL="postgres://postgres:password@localhost/postgres" && cargo sqlx database create && cargo sqlx migrate run
	@sleep 5

init-api-services:
	@docker start timescaledb || docker run -d --name timescaledb -p 5432:5432 -e POSTGRES_PASSWORD=password timescale/timescaledb:latest-pg17
	@sleep 5
	@cd ./qed_api_services && export DATABASE_URL="postgres://postgres:password@localhost/postgres" && cargo sqlx database create && cargo sqlx migrate run

.PHONY: shutdown
shutdown:
	@echo "Stopping and removing database containers..."
	@docker rm -f qed-redis-coordinator timescaledb qed-redis-realm0 qed-redis-realm1 > /dev/null 2>&1 || true
	@docker exec qed-redis-coordinator redis-cli FLUSHALL > /dev/null 2>&1 || true
	@docker exec qed-redis-realm0 redis-cli FLUSHALL > /dev/null 2>&1 || true
	@docker exec qed-redis-realm1 redis-cli FLUSHALL > /dev/null 2>&1 || true
	@redis-cli -p 6379 FLUSHALL > /dev/null 2>&1 || true
	@redis-cli -p 6380 FLUSHALL > /dev/null 2>&1 || true
	@redis-cli -p 6381 FLUSHALL > /dev/null 2>&1 || true
	# @docker rm -f qed-scylla-coordinator qed-scylla-realm0 qed-scylla-realm1 > /dev/null 2>&1 || true
	@rm -fr ${PROJECT_DIR} ${PWD}/db logs > /dev/null 2>&1 || true
	@echo "Removing user job tracker JSON files..."
	@rm -f ${USER0_PUBLIC_KEY}.json ${USER0_SECP_ZK_PUBLIC_KEY}.json ${USER1_PUBLIC_KEY}.json ${USER1_SECP_ZK_PUBLIC_KEY}.json ${USER2_PUBLIC_KEY}.json ${USER2_SECP_ZK_PUBLIC_KEY}.json ${USER3_PUBLIC_KEY}.json ${USER3_SECP_ZK_PUBLIC_KEY}.json > /dev/null 2>&1 || true

run-all: shutdown init compile
	@./scripts/run_all.sh

run-scenario0:
	@./scripts/run_scenario0.sh

interpret:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/dargo execute --program-dir $(dir ${FILE}) --debug --entry-path $(notdir ${FILE}) --parameters ${PARAMETERS}

compile:
	# Compile token contract
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR}/token && ../../target/${PROFILE}/dargo compile --entry-path src/main.qed --contract-name=ContractRef --method-names simple_mint simple_burn simple_transfer simple_claim
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR}/rewards && ../../target/${PROFILE}/dargo compile --entry-path src/main.qed --contract-name=ContractRef --method-names simple_mint simple_burn simple_transfer simple_claim batch_claim_pm_rewards
	@RUST_LOG=${LOG_LEVEL} cd ${PROJECT_DIR}/mining_rewards && ../../target/${PROFILE}/dargo compile --entry-path src/main.qed --contract-name=ContractRef --method-names advance_to_checkpoint advance_to_checkpoint_and_seal claim_simple_reward claim_guta_proof

run-api-services:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_api_services

run-coordinator-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor --database lmdbx --lmdbx-path ${PWD}/db/coordinator

run-coordinator-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge --database lmdbx --lmdbx-path ${PWD}/db/coordinator

run-realm-processor:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor --redis-uri=redis://127.0.0.1:6380 --database lmdbx --lmdbx-path ${PWD}/db/realm0

run-realm-edge:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge --redis-uri=redis://127.0.0.1:6380 --database lmdbx --lmdbx-path ${PWD}/db/realm0

run-realm-processor1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
      --redis-uri=redis://127.0.0.1:6381 \
      --database lmdbx \
      --lmdbx-path ${PWD}/db/realm1 \
      --node-id=2 \
      --realm-id=1 \
      --queue-biz-key=rwq1

run-realm-edge1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
      --listen-addr=0.0.0.0:8547 \
      --redis-uri=redis://127.0.0.1:6381 \
      --database lmdbx \
      --lmdbx-path ${PWD}/db/realm1 \
      --coordinator-addr=http://127.0.0.1:8545 \
      --realm-id=1 \
      --queue-biz-key=rwq1

run-watcher-coordinator:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli watcher \
	--node-id 0 \
	--node-type coordinator \
	--redis-url redis://127.0.0.1:6379 \
	--api-endpoint "http://localhost:3000" \
	--database lmdbx \
	--lmdbx-path ${PWD}/db/coordinator

run-watcher-realm0:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli watcher \
	--node-id 0 \
	--node-type realm \
	--redis-url redis://127.0.0.1:6380 \
	--api-endpoint "http://localhost:3000" \
	--database lmdbx \
    --lmdbx-path ${PWD}/db/realm0

run-watcher-realm1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli watcher \
	--node-id 1 \
	--node-type realm \
	--redis-url redis://127.0.0.1:6381 \
	--api-endpoint "http://localhost:3000" \
	--database lmdbx \
    --lmdbx-path ${PWD}/db/realm1

run-api-service:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_api_services

run-worker0:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli worker \
      --config=./config.json \
      --private-key=${USER0_PRIVATE_KEY}

run-worker1:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli worker \
      --config=./config.json \
      --private-key=${USER1_PRIVATE_KEY}

run-worker2:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli worker \
      --config=./config.json \
      --private-key=${USER2_PRIVATE_KEY}

TIKV_PD_ENDPOINTS := 127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383

init-tikv:
	@echo "Starting TiKV cluster..."
	@docker-compose -f ./scripts/docker-compose.tikv.yml up -d
	@echo "Waiting for TiKV to be ready..."
	@sleep 15
	@echo "TiKV cluster is ready"

shutdown-tikv:
	@echo "Stopping TiKV cluster..."
	@docker-compose -f ./scripts/docker-compose.tikv.yml down -v
	@echo "TiKV cluster stopped"

run-coordinator-processor-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-processor \
		--database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace coordinator

run-coordinator-edge-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli coordinator-edge \
		--database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace coordinator

run-realm-processor-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
		--redis-uri=redis://127.0.0.1:6380 \
		--database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm0

run-realm-edge-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
		--redis-uri=redis://127.0.0.1:6380 \
		--database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm0

run-realm-processor1-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-processor \
		--redis-uri=redis://127.0.0.1:6381 \
		--database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm1 \
		--realm-id=1 \
		--queue-biz-key=rwq1

run-realm-edge1-tikv:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli realm-edge \
		--listen-addr=0.0.0.0:8547 \
        --redis-uri=redis://127.0.0.1:6381 \
        --database tikv \
		--tikv-pd-endpoints ${TIKV_PD_ENDPOINTS} \
		--tikv-namespace realm1 \
        --coordinator-addr=http://127.0.0.1:8545 \
		--realm-id=1 \
		--queue-biz-key=rwq1

run-all-tikv: shutdown-tikv init-tikv
	@./scripts/run_all_tikv.sh

run-user-prover:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli local-prover

run-prove-proxy:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli prove-proxy

run-web-wallet:
	@cd qed-ts-sdk/app/qed-wallet && pnpm i && pnpm run dev

run-benchmark:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_dev_cli stress-test --task-type multicall --only-flow --repeat 100

generate-access-token:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_rollup_cli generate-access-token

get-public-key:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli get-public-key --private-key=${CURRENT_USER_PRIVATE_KEY} --sign-type ${SIGN_TYPE}

wallet:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli wallet create

random-wallet:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli random-wallet

register-user:
	@echo "Registering all 4 users..."
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER0_PRIVATE_KEY} --sign-type zk | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER1_PRIVATE_KEY} --sign-type zk | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER2_PRIVATE_KEY} --sign-type zk | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER0_PRIVATE_KEY} --sign-type secp256k1 | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER1_PRIVATE_KEY} --sign-type secp256k1 | tail -5 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER2_PRIVATE_KEY} --sign-type secp256k1 | tail -5 | jq .
	@sleep 0.5
	# @RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER2_PRIVATE_KEY} | tail -5 | jq .
	# @sleep 0.5
	# @RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user --private-key=${USER3_PRIVATE_KEY} | tail -5 | jq .

register-random-user:
	@echo "Registering random users..."
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user | tail -6 | jq .
	@sleep 0.5
	@RUST_LOG=error ./target/${PROFILE}/qed_user_cli register-user | tail -6 | jq .
	@sleep 0.5

deploy-contract:
	@echo "Deploying contracts..."
	@echo "USER0 deploying token contract..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${USER0_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/token/target/token.json
	@echo "USER1 deploying token contract..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli deploy-contract --private-key=${USER1_PRIVATE_KEY} --contract-path ${PROJECT_DIR}/token/target/token.json

multi-contract-call:
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli wallet-session -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID}

mint:
	@echo "All users minting 1000 tokens..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000000000000 --sign-type $(SIGN_TYPE)
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000000000000 --sign-type $(SIGN_TYPE)
	# @RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER2_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000000000000
	# @RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER3_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_mint --inputs 1000000000000

transfer:
	@echo "USER0 transferring 250 to USER1..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 8388608 --inputs 250000000000 --sign-type $(SIGN_TYPE)
	@echo "USER1 transferring 250 to USER0..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 250000000000 --sign-type $(SIGN_TYPE)

claim:
	@echo "USER1 claiming transfer..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 0 --sign-type $(SIGN_TYPE)
	@echo "USER0 claiming transfer..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_claim --inputs 8388608 --sign-type $(SIGN_TYPE)

return-back:
	@echo "USER1 transferring back to USER0..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER1_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 0 --inputs 250000000000 --sign-type $(SIGN_TYPE)
	@echo "USER0 transferring back to USER1..."
	@RUST_LOG=${LOG_LEVEL} ./target/${PROFILE}/qed_user_cli submit-end-caproof -p ${USER0_PRIVATE_KEY} --contract-id ${CONTRACT_ID} --method-name simple_transfer --inputs 8388608 --inputs 250000000000 --sign-type $(SIGN_TYPE)

claim-rewards:
	@RUST_LOG=info ./target/${PROFILE}/qed_user_cli claim-rewards --checkpoint-id ${CHECKPOINT_ID} --private-key ${CURRENT_USER_PRIVATE_KEY} --sign-type secp256k1

get-job-proof:
	@RUST_LOG=info ./target/${PROFILE}/qed_dev_cli get-job-proof --checkpoint-id ${CHECKPOINT_ID} --private-key ${CURRENT_USER_PRIVATE_KEY} --sign-type secp256k1

# Unified RPC commands using qed_user_cli automatic routing
balance-of:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID} --height ${CONTRACT_STATE_HEIGHT} --leaf-id ${SLOT_ID}

reward-of:
	@echo "Getting rewards of all users ..."
	@make reward-of-0
	@make reward-of-1
	@make reward-of-2
	@make reward-of-3

reward-of-0:
	@curl -s -X GET "http://localhost:3000/stats/workers/${USER0_SECP_ZK_PUBLIC_KEY}" | jq .
reward-of-1:
	@curl -s -X GET "http://localhost:3000/stats/workers/${USER1_SECP_ZK_PUBLIC_KEY}" | jq .
reward-of-2:
	@curl -s -X GET "http://localhost:3000/stats/workers/${USER2_SECP_ZK_PUBLIC_KEY}" | jq .
reward-of-3:
	@curl -s -X GET "http://localhost:3000/stats/workers/${USER3_SECP_ZK_PUBLIC_KEY}" | jq .

build-block:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_build_block", "params": [], "id": 1 }' | jq .

latest-checkpoint:
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_latest_checkpoint", "params": [], "id": 1 }' | jq .

# Metadata RPC commands
get-contract-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-contract-leaf-data --contract-id ${CONTRACT_ID}

get-checkpoint-leaf-data:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-leaf-data --checkpoint-id ${CHECKPOINT_ID}

get-contract-code-definition:
	@./target/${PROFILE}/qed_user_cli get-contract-code-definition --contract-id ${CONTRACT_ID}

get-latest-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-latest-l2-block-state

get-l2-block-state:
	@./target/${PROFILE}/qed_user_cli get-l2-block-state --checkpoint-id ${CHECKPOINT_ID}

# Tree structure RPC commands
get-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-root --checkpoint-id ${CHECKPOINT_ID}

get-latest-checkpoint-tree-root:
	@./target/${PROFILE}/qed_user_cli get-latest-checkpoint-tree-root

get-checkpoint-tree-leaf-hash:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-leaf-hash --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

get-checkpoint-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-checkpoint-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --leaf-checkpoint-id ${LEAF_CHECKPOINT_ID}

# User tree RPC commands
get-user-leaf-data-by-pubkey:
	@./target/${PROFILE}/qed_user_cli get-user-leaf --checkpoint-id ${CHECKPOINT_ID} --pub-key ${CURRENT_USER_PUBLIC_KEY}

get-user-leaf-data-by-userid:
	@./target/${PROFILE}/qed_user_cli get-user-leaf --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

get-user-leaf-data: get-user-leaf-data-by-pubkey

get-user-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-tree-root --checkpoint-id ${CHECKPOINT_ID}

get-user-tree-leaf-hash:
	@./target/${PROFILE}/qed_user_cli get-user-tree-leaf-hash --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

get-user-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

get-user-sub-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-sub-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --root-level 0 --leaf-level ${REALM_TREE_LEAF_LEVEL} --leaf-index ${REALM_ID}

get-user-registration-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-registration-tree-root --checkpoint-id ${CHECKPOINT_ID}

# User contract tree RPC commands
get-user-contract-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-contract-tree-root --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID}

get-user-contract-state-tree-root:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-root --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID}

get-user-contract-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-contract-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID}

get-user-contract-state-tree-merkle-proof:
	@./target/${PROFILE}/qed_user_cli get-user-contract-state-tree-merkle-proof --checkpoint-id ${CHECKPOINT_ID} --user-id ${USER_ID} --contract-id ${CONTRACT_ID} --height ${CONTRACT_STATE_HEIGHT} --leaf-id ${SLOT_ID}

# Check if user exists in realm
check-user-id:
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{ "jsonrpc": "2.0", "method": "qed_check_user_id_in_realm", "params": [${USER_ID}], "id": 1 }' | jq .

get-graphviz-coordinator:
	@echo "Getting graphviz from coordinator for checkpoint ${CHECKPOINT_ID}..."
	@curl -s -X POST "${COORDINATOR_RPC_URL}" -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "qed_get_graphviz", "params": [${CHECKPOINT_ID}], "id": 1}' | jq -r '.result' | sed 's/\\n/\n/g'

get-graphviz-realm:
	@echo "Getting graphviz from realm0 for checkpoint ${CHECKPOINT_ID}..."
	@curl -s -X POST "${REALM_RPC_URL}" -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "qed_get_graphviz", "params": [${CHECKPOINT_ID}], "id": 1}' | jq -r '.result' | sed 's/\\n/\n/g'

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

wallet-build: wasm-build
	@cd qed-ts-sdk/app/qed-wallet && pnpm i && pnpm build:wasm && pnpm build:extension

help:
	@grep -E '^[a-zA-Z_-]+:.*?' Makefile | cut -d: -f1 | sort
