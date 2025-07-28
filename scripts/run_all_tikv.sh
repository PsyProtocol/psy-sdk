#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

# Verify PD cluster health
echo "Verifying PD cluster health..."
for i in {0..2}; do
    port=$((2379 + i * 2))
    echo "Checking PD$i on port $port..."
    for attempt in {1..10}; do
        if curl -s "http://localhost:$port/health" > /dev/null; then
            echo "PD$i is healthy on port $port"
            break
        else
            echo "Attempt $attempt: PD$i not ready on port $port, waiting..."
            sleep 5
        fi
    done
done

# Define log directory and ensure it exists
LOG_DIR="./logs"
mkdir -p "$LOG_DIR"

# Define log files for each service
COORDINATOR_PROCESSOR_LOG="$LOG_DIR/coordinator-processor-tikv.log"
COORDINATOR_WORKER_LOG="$LOG_DIR/coordinator-worker.log"
REALM_PROCESSOR_LOG="$LOG_DIR/realm-processor-tikv.log"
REALM_WORKER_LOG="$LOG_DIR/realm-worker.log"
COORDINATOR_EDGE_LOG="$LOG_DIR/coordinator-edge-tikv.log"
REALM_EDGE_LOG="$LOG_DIR/realm-edge-tikv.log"

REALM_PROCESSOR1_LOG="$LOG_DIR/realm-processor1-tikv.log"
REALM_WORKER1_LOG="$LOG_DIR/realm-worker1.log"
REALM_EDGE1_LOG="$LOG_DIR/realm-edge1-tikv.log"

LOCAL_USER_PROVER_LOG="$LOG_DIR/local-user-prover.log"
WEB_WALLET_LOG="$LOG_DIR/web_wallet.log"

# Clear log files at startup
echo "Clearing log files..."
: > "$COORDINATOR_PROCESSOR_LOG"
: > "$COORDINATOR_WORKER_LOG"
: > "$REALM_PROCESSOR_LOG"
: > "$REALM_WORKER_LOG"
: > "$COORDINATOR_EDGE_LOG"
: > "$REALM_EDGE_LOG"
: > "$REALM_PROCESSOR1_LOG"
: > "$REALM_WORKER1_LOG"
: > "$REALM_EDGE1_LOG"
: > "$LOCAL_USER_PROVER_LOG"
: > "$WEB_WALLET_LOG"

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
        # Run service and append both stdout and stderr to log file
        ( $service_cmd 2>&1 | sed 's/\o033\[[0-9;]*m//g' ) >> "$log_file" &
        local pid=$!
        PIDS+=("$pid")  # Add PID to array
        wait $pid
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $service_name (PID: $pid) stopped. Restarting in 5 seconds..." | tee -a "$log_file"
        sleep 5
    done
}

# Start TiKV cluster first
echo "Starting TiKV cluster..."
make init-tikv

# Group 1: Start processor and worker services in background (using TiKV)
run_service "make run-coordinator-processor-tikv" "coordinator-processor-tikv" "$COORDINATOR_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-coordinator-worker" "coordinator-worker" "$COORDINATOR_WORKER_LOG" &
PIDS+=($!)
run_service "make run-realm-processor-tikv" "realm-processor-tikv" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor1-tikv" "realm-processor1-tikv" "$REALM_PROCESSOR1_LOG" &
PIDS+=($!)
run_service "make run-realm-worker" "realm-worker" "$REALM_WORKER_LOG" &
PIDS+=($!)
run_service "make run-realm-worker1" "realm-worker" "$REALM_WORKER1_LOG" &
PIDS+=($!)

# Group 2: Start edge services (depend on processors/workers)
sleep 3
run_service "make run-coordinator-edge-tikv" "coordinator-edge-tikv" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge-tikv" "realm-edge-tikv" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge1-tikv" "realm-edge1-tikv" "$REALM_EDGE1_LOG" &
PIDS+=($!)

sleep 1
run_service "make run-user-prover" "local-user-prover" "$LOCAL_USER_PROVER_LOG" &
PIDS+=($!)
run_service "make run-web-wallet" "web-wallet" "$WEB_WALLET_LOG" &
PIDS+=($!)

# Wait for all background processes
wait

echo "[$(date '+%Y-%m-%d %H:%M:%S')] All TiKV services started" | tee -a "$COORDINATOR_PROCESSOR_LOG"