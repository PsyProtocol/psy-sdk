#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

# Define log directory and ensure it exists
LOG_DIR="./logs"
mkdir -p "$LOG_DIR"

# Define log file for the scenario
SCENARIO_LOG="$LOG_DIR/scenario0.log"

GET_USER0_BALANCE="make CHECKPOINT_ID=100 USER_ID=0 get-slot-value"
GET_USER1_BALANCE="make CHECKPOINT_ID=100 USER_ID=4194304 REALM_RPC_URL=http://127.0.0.1:8547 get-slot-value"

# Clear log file at startup
echo "Clearing log file..."
: > "$SCENARIO_LOG"

# Function to log messages
log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$SCENARIO_LOG"
}

# Function to run a make command with logging
run_make_command() {
    local cmd=$1
    local desc=$2
    log_message "Running $desc..."
    if (set -o pipefail; $cmd 2>&1 | sed 's/\x1b\[[0-9;]*m//g' >> "$SCENARIO_LOG"); then
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
    run_make_command "$GET_USER1_BALANCE" "get user 4194304 token info"
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Execute the scenario commands
log_message "Starting Scenario 0..."
sleep 30

# run_make_command "make deploy-contract" "Deploy Contract"
# run_make_command "make register-user" "Register User"
# echo "wait for block 1 to be committed"
# sleep 10s
run_make_command "make build-block" "Make build Block 2"
echo "wait for block 2 to be committed"
sleep 10s
run_make_command "make mint" "Mint"
echo "wait for block 3 to be committed"
sleep 10s

get_user_token_info

run_make_command "make transfer" "Transfer"
echo "wait for block 4 to be committed"
sleep 10s
#run_make_command "make build-block" "Make build Block 5"
#echo "wait for block 5 to be committed"
#sleep 10s
get_user_token_info

run_make_command "make claim" "Claim"
echo "wait for block 6 to be committed"
sleep 10s

get_user_token_info

run_make_command "make return-back" "return-back"
echo "wait for block 7 to be committed"
sleep 10s

run_make_command "make claim-rewards" "claim-rewards"
echo "wait for block 8 to be committed"
sleep 10s

get_user_token_info

log_message "Scenario 0 completed successfully."

log_message "Running benchmark"

run_make_command "make run-benchmark" "Benchmark"
