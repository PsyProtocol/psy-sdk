#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

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
    pkill -f psy_dev_cli
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] All processes terminated. Exiting."
    exit 0
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Define log directory and ensure it exists
LOG_DIR="./logs/benchmark"
mkdir -p "$LOG_DIR"

# Assuming binaries are already built in release mode

# Clear results file
: > "$LOG_DIR/results.tmp"

# Function to run benchmark in background and log results
run_benchmark() {
    local test_name=$1
    local cmd=$2
    local log_file="$LOG_DIR/benchmark-$test_name.log"

    echo "========================================="
    echo "Starting benchmark: $test_name"
    echo "Command: $cmd"
    echo "Log file: $log_file"
    echo "========================================="

    # Clear previous log
    : > "$log_file"

    # Run benchmark in background
    {
        local start_time=$(date +%s)
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting benchmark: $test_name" | tee -a "$log_file"

        if eval "$cmd" >> "$log_file" 2>&1; then
            local end_time=$(date +%s)
            local duration=$((end_time - start_time))
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Completed benchmark: $test_name (Duration: ${duration}s)" | tee -a "$log_file"
            echo "✅ $test_name: ${duration}s" >> "$LOG_DIR/results.tmp"
        else
            local end_time=$(date +%s)
            local duration=$((end_time - start_time))
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Failed benchmark: $test_name (Duration: ${duration}s)" | tee -a "$log_file"
            echo "❌ $test_name: FAILED (${duration}s)" >> "$LOG_DIR/results.tmp"
        fi
    } &

    local pid=$!
    PIDS+=("$pid")
    echo "Started benchmark $test_name with PID: $pid"
    echo ""
}

# Benchmark suite based on your history
echo "Starting QED Benchmark Suite..."
echo "================================="

# Start all benchmarks in parallel
run_benchmark "user-registration" "make run-benchmark-register"
run_benchmark "mint-1" "make run-benchmark-mint"
run_benchmark "mint-2" "make run-benchmark-mint"
run_benchmark "mint-3" "make run-benchmark-mint"
run_benchmark "mint-4" "make run-benchmark-mint"
run_benchmark "multi-transfer-1" "make run-benchmark-transfer"
run_benchmark "multi-transfer-2" "make run-benchmark-transfer"
run_benchmark "flow-1" "make run-benchmark-flow"
run_benchmark "flow-2" "make run-benchmark-flow"
run_benchmark "deploy" "make run-benchmark-deploy"

echo "All benchmarks started. Waiting for completion..."

# Wait for all background processes to complete
for pid in "${PIDS[@]}"; do
    wait $pid
    echo "Process $pid completed"
done

# Print summary
echo "========================================="
echo "BENCHMARK RESULTS SUMMARY"
echo "========================================="
if [ -f "$LOG_DIR/results.tmp" ]; then
    cat "$LOG_DIR/results.tmp"
else
    echo "No results found"
fi
echo "========================================="
echo "Detailed logs available in: $LOG_DIR"
echo "Benchmark suite completed at: $(date '+%Y-%m-%d %H:%M:%S')"
