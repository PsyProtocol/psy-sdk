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

GET_USER0_BALANCE="make CHECKPOINT_ID=100 USER_ID=0 balance-of"
GET_USER1_BALANCE="make CHECKPOINT_ID=100 USER_ID=8388608 REALM_RPC_URL=http://127.0.0.1:8547 balance-of"

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
    run_make_command "$GET_USER1_BALANCE" "get user 8388608 token info"
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Execute the scenario commands
log_message "Starting Scenario 0..."
sleep 30

run_make_command "make deploy-contract" "Deploy Contract"
sleep 30
run_make_command "make register-user" "Register User"
sleep 30
run_make_command "make build-block" "Build Block 1"
sleep 30
run_make_command "make build-block" "Build Block 2"
sleep 30
run_make_command "make mint" "Mint"
sleep 30
run_make_command "make build-block" "Build Block 3"
sleep 30
run_make_command "make build-block" "Build Block 4"
sleep 30

get_user_token_info

run_make_command "make transfer" "Transfer"
sleep 30
run_make_command "make build-block" "Build Block 5"
sleep 30
run_make_command "make build-block" "Build Block 6"
sleep 30

get_user_token_info

sleep 10
run_make_command "make claim" "Claim"
run_make_command "make build-block" "Build Block 7"
sleep 30
run_make_command "make build-block" "Build Block 8"
sleep 30

get_user_token_info

run_make_command "make return-back" "return-back"
sleep 30
run_make_command "make build-block" "Build Block 9"
sleep 30
run_make_command "make build-block" "Build Block 10"
sleep 30

get_user_token_info

log_message "Scenario 0 completed successfully."
