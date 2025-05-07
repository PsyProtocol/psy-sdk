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
COORDINATOR_WORKER_LOG="$LOG_DIR/coordinator-worker.log"
REALM_PROCESSOR_LOG="$LOG_DIR/realm-processor.log"
REALM_WORKER_LOG="$LOG_DIR/realm-worker.log"
COORDINATOR_EDGE_LOG="$LOG_DIR/coordinator-edge.log"
REALM_EDGE_LOG="$LOG_DIR/realm-edge.log"

REALM_PROCESSOR2048_LOG="$LOG_DIR/realm-processor2048.log"
REALM_WORKER2048_LOG="$LOG_DIR/realm-worker2048.log"
REALM_EDGE2048_LOG="$LOG_DIR/realm-edge2048.log"

# Clear log files at startup
echo "Clearing log files..."
: > "$COORDINATOR_PROCESSOR_LOG"
: > "$COORDINATOR_WORKER_LOG"
: > "$REALM_PROCESSOR_LOG"
: > "$REALM_WORKER_LOG"
: > "$COORDINATOR_EDGE_LOG"
: > "$REALM_EDGE_LOG"

# Run shutdown, init, compile
make shutdown
make init
make compile

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
        $service_cmd >> "$log_file" 2>&1 &
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
run_service "make run-coordinator-worker" "coordinator-worker" "$COORDINATOR_WORKER_LOG" &
PIDS+=($!)
run_service "make run-realm-processor" "realm-processor" "$REALM_PROCESSOR_LOG" &
PIDS+=($!)
run_service "make run-realm-processor2048" "realm-processor2048" "$REALM_PROCESSOR2048_LOG" &
PIDS+=($!)
run_service "make run-realm-worker" "realm-worker" "$REALM_WORKER_LOG" &
PIDS+=($!)
run_service "make run-realm-worker2048" "realm-worker2048" "$REALM_WORKER2048_LOG" &
PIDS+=($!)

# Group 2: Start edge services (depend on processors/workers)
run_service "make run-coordinator-edge" "coordinator-edge" "$COORDINATOR_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge" "realm-edge" "$REALM_EDGE_LOG" &
PIDS+=($!)
run_service "make run-realm-edge2048" "realm-edge2048" "$REALM_EDGE2048_LOG" &
PIDS+=($!)

# Wait for all background processes
wait

echo "[$(date '+%Y-%m-%d %H:%M:%S')] All services started" | tee -a "$COORDINATOR_PROCESSOR_LOG"
