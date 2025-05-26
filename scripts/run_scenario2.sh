#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

# Define log directory and ensure it exists
LOG_DIR="./logs"
mkdir -p "$LOG_DIR"

# Define log file for the scenario
SCENARIO_LOG="$LOG_DIR/scenario2.log"

GET_USER0_BALANCE="make CHECKPOINT_ID=100 USER_ID=0 balance-of"
GET_USER1_BALANCE="make CHECKPOINT_ID=100 USER_ID=536870912 REALM_RPC_URL=http://127.0.0.1:8547 balance-of"

# Clear log file at startup
# echo "Clearing log file..."
# : > "$SCENARIO_LOG"

# Function to log messages
log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$SCENARIO_LOG"
}

# Function to run a make command with logging
run_make_command() {
    local cmd=$1
    local desc=$2
    log_message "Running $desc..."
    if (set -o pipefail; $cmd 2>&1 | sed 's/\o033\[[0-9;]*m//g' >> "$SCENARIO_LOG"); then
        log_message "$desc completed successfully."
    else
        log_message "Error: $desc failed."
        exit 1
    fi
}

# Function to handle cleanup on interrupt
cleanup() {
    log_message "Received interrupt. Exiting scenario."
    exit 0
}

# Function to get user token info
get_user_token_info() {
    run_make_command "$GET_USER0_BALANCE" "get user 0 token info"
    run_make_command "$GET_USER1_BALANCE" "get user 536870912 token info"
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM


CURRENT_BLOCK=1

# Execute the scenario commands
log_message "Starting Scenario 2..."
echo "wait for block $CURRENT_BLOCK"
sleep 5

run_make_command "make deploy-contract" "Deploy Contract"

# register 2 users
run_make_command "make register-user" "Register User"

# register 2 users
run_make_command "make register-user2" "Register User2"

CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5


run_make_command "make mint" "Mint"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make transfer" "Transfer"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make claim" "Claim"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make return-back" "return-back"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make mint2" "Mint2"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make transfer2" "Transfer2"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

# get_user_token_info

run_make_command "make claim3" "Claim3"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

run_make_command "make return-back3" "return-back3"
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5
CURRENT_BLOCK=$((CURRENT_BLOCK + 1))
run_make_command "make build-block" "Build Block $CURRENT_BLOCK"
sleep 5

get_user_token_info

log_message "Scenario 2 completed successfully."
