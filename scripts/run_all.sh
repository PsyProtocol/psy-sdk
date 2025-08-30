#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

# Define log directory and ensure it exists
LOG_DIR="./logs"
mkdir -p "$LOG_DIR"

# Define log files for each service
COORDINATOR_PROCESSOR_LOG="$LOG_DIR/coordinator-processor.log"
REALM_PROCESSOR_LOG="$LOG_DIR/realm-processor.log"
COORDINATOR_EDGE_LOG="$LOG_DIR/coordinator-edge.log"
REALM_EDGE_LOG="$LOG_DIR/realm-edge.log"

REALM_PROCESSOR1_LOG="$LOG_DIR/realm-processor1.log"
REALM_EDGE1_LOG="$LOG_DIR/realm-edge1.log"

WORKER0_LOG="$LOG_DIR/worker0.log"
WORKER1_LOG="$LOG_DIR/worker1.log"
WORKER2_LOG="$LOG_DIR/worker2.log"

# LOCAL_USER_PROVER_LOG="$LOG_DIR/local-user-prover.log"
LOCAL_PROVE_PROXY_LOG="$LOG_DIR/local-prove-proxy.log"
WEB_WALLET_LOG="$LOG_DIR/web_wallet.log"

# Clear log files at startup
echo "Clearing log files..."
: > "$COORDINATOR_PROCESSOR_LOG"
: > "$REALM_PROCESSOR_LOG"
: > "$COORDINATOR_EDGE_LOG"
: > "$REALM_EDGE_LOG"
: > "$REALM_PROCESSOR1_LOG"
: > "$REALM_EDGE1_LOG"
: > "$LOCAL_PROVE_PROXY_LOG"
: > "$WEB_WALLET_LOG"
: > "$WORKER0_LOG"
: > "$WORKER1_LOG"
: > "$WORKER2_LOG"

# Array to store PIDs of background processes
declare -a PIDS=()

# Function to kill all tracked processes
cleanup() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Received interrupt. Terminating all processes..."
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Killing process $pid..."
            kill -TERM "$pid" 2>/dev/null
        fi
    done
    pkill -f qed_user_cli
    pkill -f qed_rollup_cli
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] All processes terminated. Exiting."
    exit 0
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Function to run a service and append both stdout and stderr to log file
run_service() {
    local service_cmd=$1
    local service_name=$2
    local log_file=$3
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting $service_name (logging to $log_file)..." | tee -a "$log_file"
    while true; do
        # Run service with unbuffered output and append both stdout and stderr to log file
        stdbuf -oL -eL $service_cmd 2>&1 | stdbuf -oL sed 's/\x1b\[[0-9;]*m//g' >> "$log_file" &
        local pid=$!
        PIDS+=("$pid")  # Add PID to array
        wait $pid
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $service_name (PID: $pid) stopped. Restarting in 5 seconds..." | tee -a "$log_file"
        sleep 5
    done
}

# Group 1: Start processor and worker services in background
run_service "make run-coordinator-processor" "coordinator-processor" "$COORDINATOR_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor" "realm-processor" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor1" "realm-processor1" "$REALM_PROCESSOR1_LOG" &
PIDS+=($!)

# Group 2: Start edge services (depend on processors)
sleep 6
run_service "make run-coordinator-edge" "coordinator-edge" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge" "realm-edge" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge1" "realm-edge1" "$REALM_EDGE1_LOG" &
PIDS+=($!)

# Group 3: Start worker services (depend on edges)
sleep 2
run_service "make run-worker0" "worker0" "$WORKER0_LOG" &
PIDS+=($!)
run_service "make run-worker1" "worker1" "$WORKER1_LOG" &
PIDS+=($!)
run_service "make run-worker2" "worker2" "$WORKER2_LOG" &
PIDS+=($!)

sleep 1
# run_service "make run-user-prover" "local-user-prover" "$LOCAL_USER_PROVER_LOG" &
# PIDS+=($!)
run_service "make run-web-wallet" "web-wallet" "$WEB_WALLET_LOG" &
PIDS+=($!)
run_service "make run-prove-proxy" "local-prove-proxy" "$LOCAL_PROVE_PROXY_LOG" &
PIDS+=($!)

# Wait for all background processes
wait

echo "[$(date '+%Y-%m-%d %H:%M:%S')] All services started" | tee -a "$COORDINATOR_PROCESSOR_LOG"
