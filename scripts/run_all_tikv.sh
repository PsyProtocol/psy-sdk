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
REALM_PROCESSOR_LOG="$LOG_DIR/realm-processor-tikv.log"
COORDINATOR_EDGE_LOG="$LOG_DIR/coordinator-edge-tikv.log"
REALM_EDGE_LOG="$LOG_DIR/realm-edge-tikv.log"

REALM_PROCESSOR1_LOG="$LOG_DIR/realm-processor1-tikv.log"
REALM_EDGE1_LOG="$LOG_DIR/realm-edge1-tikv.log"

WORKER0_LOG="$LOG_DIR/worker0.log"
WORKER1_LOG="$LOG_DIR/worker1.log"
WORKER2_LOG="$LOG_DIR/worker2.log"

API_SERVICES_LOG="$LOG_DIR/api-services.log"

WATCHER_COORDINATOR_LOG="$LOG_DIR/watcher-coordinator-tikv.log"
WATCHER_REALM0_LOG="$LOG_DIR/watcher-realm0-tikv.log"
WATCHER_REALM1_LOG="$LOG_DIR/watcher-realm1-tikv.log"

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
: > "$API_SERVICES_LOG"
: > "$WATCHER_COORDINATOR_LOG"
: > "$WATCHER_REALM0_LOG"
: > "$WATCHER_REALM1_LOG"

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
    pkill -f psy_user_cli
    pkill -f psy_node_cli
    pkill -f psy_api_service
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

# Group 1: Start processor and worker services in background (using TiKV)
run_service "make run-coordinator-processor-tikv" "coordinator-processor-tikv" "$COORDINATOR_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor-tikv" "realm-processor-tikv" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor1-tikv" "realm-processor1-tikv" "$REALM_PROCESSOR1_LOG" &
PIDS+=($!)

# Group 2: Start edge services (depend on processors)
sleep 8
run_service "make run-coordinator-edge-tikv" "coordinator-edge-tikv" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge-tikv" "realm-edge-tikv" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge1-tikv" "realm-edge1-tikv" "$REALM_EDGE1_LOG" &
PIDS+=($!)

# Group 3: Start worker services (depend on edges)
sleep 2
run_service "make run-worker0" "worker0" "$WORKER0_LOG" &
PIDS+=($!)
run_service "make run-worker1" "worker1" "$WORKER1_LOG" &
PIDS+=($!)
run_service "make run-worker2" "worker2" "$WORKER2_LOG" &
PIDS+=($!)

run_service "make run-api-services" "api-services" "$API_SERVICES_LOG" &
PIDS+=($!)

sleep 1
run_service "make run-watcher-coordinator-tikv" "watcher-coordinator-tikv" "$WATCHER_COORDINATOR_LOG" &
PIDS+=($!)
run_service "make run-watcher-realm0-tikv" "watcher-realm0-tikv" "$WATCHER_REALM0_LOG" &
PIDS+=($!)
run_service "make run-watcher-realm1-tikv" "watcher-realm1-tikv" "$WATCHER_REALM1_LOG" &
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

echo "[$(date '+%Y-%m-%d %H:%M:%S')] All TiKV services started" | tee -a "$COORDINATOR_PROCESSOR_LOG"