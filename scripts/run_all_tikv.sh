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
REALM_PROCESSOR2_LOG="$LOG_DIR/realm-processor2-tikv.log"
REALM_EDGE2_LOG="$LOG_DIR/realm-edge2-tikv.log"
REALM_PROCESSOR3_LOG="$LOG_DIR/realm-processor3-tikv.log"
REALM_EDGE3_LOG="$LOG_DIR/realm-edge3-tikv.log"
REALM_PROCESSOR4_LOG="$LOG_DIR/realm-processor4-tikv.log"
REALM_EDGE4_LOG="$LOG_DIR/realm-edge4-tikv.log"
REALM_PROCESSOR5_LOG="$LOG_DIR/realm-processor5-tikv.log"
REALM_EDGE5_LOG="$LOG_DIR/realm-edge5-tikv.log"
REALM_PROCESSOR6_LOG="$LOG_DIR/realm-processor6-tikv.log"
REALM_EDGE6_LOG="$LOG_DIR/realm-edge6-tikv.log"
REALM_PROCESSOR7_LOG="$LOG_DIR/realm-processor7-tikv.log"
REALM_EDGE7_LOG="$LOG_DIR/realm-edge7-tikv.log"
WORKER0_LOG="$LOG_DIR/worker0.log"
WORKER1_LOG="$LOG_DIR/worker1.log"
WORKER2_LOG="$LOG_DIR/worker2.log"
WORKER3_LOG="$LOG_DIR/worker3.log"
WORKER4_LOG="$LOG_DIR/worker4.log"
WORKER5_LOG="$LOG_DIR/worker5.log"
WORKER6_LOG="$LOG_DIR/worker6.log"
WORKER7_LOG="$LOG_DIR/worker7.log"
WORKER8_LOG="$LOG_DIR/worker8.log"

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
: > "$REALM_PROCESSOR2_LOG"
: > "$REALM_EDGE2_LOG"
: > "$REALM_PROCESSOR3_LOG"
: > "$REALM_EDGE3_LOG"
: > "$REALM_PROCESSOR4_LOG"
: > "$REALM_EDGE4_LOG"
: > "$REALM_PROCESSOR5_LOG"
: > "$REALM_EDGE5_LOG"
: > "$REALM_PROCESSOR6_LOG"
: > "$REALM_EDGE6_LOG"
: > "$REALM_PROCESSOR7_LOG"
: > "$REALM_EDGE7_LOG"
: > "$LOCAL_PROVE_PROXY_LOG"
: > "$WEB_WALLET_LOG"
: > "$WORKER0_LOG"
: > "$WORKER1_LOG"
: > "$WORKER2_LOG"
: > "$WORKER3_LOG"
: > "$WORKER4_LOG"
: > "$WORKER5_LOG"
: > "$WORKER6_LOG"
: > "$WORKER7_LOG"
: > "$WORKER8_LOG"
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
    pkill -f qed_user_cli
    pkill -f qed_rollup_cli
    pkill -f qed_api_service
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

# Start TiKV cluster first
echo "Starting TiKV cluster..."
make init-tikv

# Group 1: Start processor and worker services in background (using TiKV)
run_service "make run-coordinator-processor-tikv" "coordinator-processor-tikv" "$COORDINATOR_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor-tikv" "realm-processor-tikv" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor1-tikv" "realm-processor1-tikv" "$REALM_PROCESSOR1_LOG" &
PIDS+=($!)
run_service "make run-realm-processor2-tikv" "realm-processor2-tikv" "$REALM_PROCESSOR2_LOG" &
PIDS+=($!)
run_service "make run-realm-processor3-tikv" "realm-processor3-tikv" "$REALM_PROCESSOR3_LOG" &
PIDS+=($!)
run_service "make run-realm-processor4-tikv" "realm-processor4-tikv" "$REALM_PROCESSOR4_LOG" &
PIDS+=($!)
run_service "make run-realm-processor5-tikv" "realm-processor5-tikv" "$REALM_PROCESSOR5_LOG" &
PIDS+=($!)
run_service "make run-realm-processor6-tikv" "realm-processor6-tikv" "$REALM_PROCESSOR6_LOG" &
PIDS+=($!)
run_service "make run-realm-processor7-tikv" "realm-processor7-tikv" "$REALM_PROCESSOR7_LOG" &
PIDS+=($!)


# Group 2: Start edge services (depend on processors)
sleep 8
run_service "make run-coordinator-edge-tikv" "coordinator-edge-tikv" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge-tikv" "realm-edge-tikv" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge1-tikv" "realm-edge1-tikv" "$REALM_EDGE1_LOG" &
PIDS+=($!)
run_service "make run-realm-edge2-tikv" "realm-edge2-tikv" "$REALM_EDGE2_LOG" &
PIDS+=($!)
run_service "make run-realm-edge3-tikv" "realm-edge3-tikv" "$REALM_EDGE3_LOG" &
PIDS+=($!)
run_service "make run-realm-edge4-tikv" "realm-edge4-tikv" "$REALM_EDGE4_LOG" &
PIDS+=($!)
run_service "make run-realm-edge5-tikv" "realm-edge5-tikv" "$REALM_EDGE5_LOG" &
PIDS+=($!)
run_service "make run-realm-edge6-tikv" "realm-edge6-tikv" "$REALM_EDGE6_LOG" &
PIDS+=($!)
run_service "make run-realm-edge7-tikv" "realm-edge7-tikv" "$REALM_EDGE7_LOG" &
PIDS+=($!)

# Group 3: Start worker services (depend on edges)
sleep 2
run_service "make run-worker0" "worker0" "$WORKER0_LOG" &
PIDS+=($!)
run_service "make run-worker1" "worker1" "$WORKER1_LOG" &
PIDS+=($!)
run_service "make run-worker2" "worker2" "$WORKER2_LOG" &
PIDS+=($!)
run_service "make run-worker3" "worker3" "$WORKER3_LOG" &
PIDS+=($!)
run_service "make run-worker4" "worker4" "$WORKER4_LOG" &
PIDS+=($!)
run_service "make run-worker5" "worker5" "$WORKER5_LOG" &
PIDS+=($!)
run_service "make run-worker6" "worker6" "$WORKER6_LOG" &
PIDS+=($!)
run_service "make run-worker7" "worker7" "$WORKER7_LOG" &
PIDS+=($!)
run_service "make run-worker8" "worker8" "$WORKER8_LOG" &
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