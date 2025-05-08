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

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Execute the scenario commands
log_message "Starting Scenario 0..."
sleep 10

run_make_command "make deploy-contract" "Deploy Contract"
run_make_command "make register-user" "Register User"
run_make_command "make build-block" "Build Block 1"
sleep 20
run_make_command "make build-block" "Build Block 2"
sleep 20
run_make_command "make mint" "Mint"
run_make_command "make build-block" "Build Block 3"
sleep 20
run_make_command "make build-block" "Build Block 4"
sleep 20
run_make_command "make transfer" "Transfer"
run_make_command "make build-block" "Build Block 5"
sleep 20
run_make_command "make build-block" "Build Block 6"
sleep 20
run_make_command "make claim" "Claim"
run_make_command "make build-block" "Build Block 7"
sleep 20
run_make_command "make build-block" "Build Block 8"

log_message "Scenario 0 completed successfully."
