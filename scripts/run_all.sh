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
REALM_PROCESSOR2_LOG="$LOG_DIR/realm-processor2.log"
REALM_EDGE2_LOG="$LOG_DIR/realm-edge2.log"
REALM_PROCESSOR3_LOG="$LOG_DIR/realm-processor3.log"
REALM_EDGE3_LOG="$LOG_DIR/realm-edge3.log"
REALM_PROCESSOR4_LOG="$LOG_DIR/realm-processor4.log"
REALM_EDGE4_LOG="$LOG_DIR/realm-edge4.log"
REALM_PROCESSOR5_LOG="$LOG_DIR/realm-processor5.log"
REALM_EDGE5_LOG="$LOG_DIR/realm-edge5.log"
REALM_PROCESSOR6_LOG="$LOG_DIR/realm-processor6.log"
REALM_EDGE6_LOG="$LOG_DIR/realm-edge6.log"
REALM_PROCESSOR7_LOG="$LOG_DIR/realm-processor7.log"
REALM_EDGE7_LOG="$LOG_DIR/realm-edge7.log"

WORKER0_LOG="$LOG_DIR/worker0.log"
WORKER1_LOG="$LOG_DIR/worker1.log"
WORKER2_LOG="$LOG_DIR/worker2.log"

API_SERVICES_LOG="$LOG_DIR/api-services.log"

WATCHER_COORDINATOR_LOG="$LOG_DIR/watcher-coordinator.log"
WATCHER_REALM0_LOG="$LOG_DIR/watcher-realm0.log"
WATCHER_REALM1_LOG="$LOG_DIR/watcher-realm1.log"

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
    pkill -f psy_dev_cli
    pkill -f register_user
    pkill -f psy_services
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

# Group 1: Start coordinator services
run_service "make run-coordinator-processor" "coordinator-processor" "$COORDINATOR_PROCESSOR_LOG" &
PIDS+=($!)
sleep 5
run_service "make run-coordinator-edge" "coordinator-edge" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)

# Group 2: Start realm services (depend on coordinator)
sleep 8
run_service "make run-realm-processor" "realm-processor" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor1" "realm-processor1" "$REALM_PROCESSOR1_LOG" &
PIDS+=($!)
run_service "make run-realm-processor2" "realm-processor2" "$REALM_PROCESSOR2_LOG" &
PIDS+=($!)
run_service "make run-realm-processor3" "realm-processor3" "$REALM_PROCESSOR3_LOG" &
PIDS+=($!)

sleep 5
run_service "make run-realm-edge" "realm-edge" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge1" "realm-edge1" "$REALM_EDGE1_LOG" &
PIDS+=($!)
run_service "make run-realm-edge2" "realm-edge2" "$REALM_EDGE2_LOG" &
PIDS+=($!)
run_service "make run-realm-edge3" "realm-edge3" "$REALM_EDGE3_LOG" &
PIDS+=($!)
#run_service "make run-realm-edge4" "realm-edge4" "$REALM_EDGE4_LOG" &
#PIDS+=($!)
#run_service "make run-realm-edge5" "realm-edge5" "$REALM_EDGE5_LOG" &
#PIDS+=($!)
#run_service "make run-realm-edge6" "realm-edge6" "$REALM_EDGE6_LOG" &
#PIDS+=($!)
#run_service "make run-realm-edge7" "realm-edge7" "$REALM_EDGE7_LOG" &
#PIDS+=($!)

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
run_service "make run-watcher-coordinator" "watcher-coordinator" "$WATCHER_COORDINATOR_LOG" &
PIDS+=($!)
run_service "make run-watcher-realm0" "watcher-realm0" "$WATCHER_REALM0_LOG" &
PIDS+=($!)
run_service "make run-watcher-realm1" "watcher-realm1" "$WATCHER_REALM1_LOG" &
PIDS+=($!)

sleep 1
run_service "make run-prove-proxy" "local-prove-proxy" "$LOCAL_PROVE_PROXY_LOG" &
PIDS+=($!)

# Wait for all background processes
wait

echo "[$(date '+%Y-%m-%d %H:%M:%S')] All services started" | tee -a "$COORDINATOR_PROCESSOR_LOG"
