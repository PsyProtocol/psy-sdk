#!/bin/bash

DIR=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd $DIR/..

# Exit on error
set -e

# Configuration with defaults
NUM_PROXIES=${1:-3}  # Default to 3 prove proxies if not specified
BASE_PORT=${2:-9090}  # Default base port
LOG_LEVEL=${LOG_LEVEL:-"qed_rollup_utils=trace,qed_user_cli=debug,qed_prover=trace"}
PROFILE=${PROFILE:-release}

# Define log directory and ensure it exists
LOG_DIR="./logs/prove_proxies"
mkdir -p "$LOG_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored messages
print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Array to store PIDs of background processes
declare -a PIDS=()
declare -a PORTS=()
declare -a LOG_FILES=()

# Function to kill all tracked processes
cleanup() {
    print_message "$YELLOW" "[$(date '+%Y-%m-%d %H:%M:%S')] Received interrupt. Terminating all processes..."

    # Kill all tracked PIDs
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Killing process $pid..."
            kill -TERM "$pid" 2>/dev/null
        fi
    done

    # Additional cleanup for any stragglers
    pkill -f "qed_user_cli prove-proxy" 2>/dev/null || true

    print_message "$GREEN" "[$(date '+%Y-%m-%d %H:%M:%S')] All processes terminated. Exiting."
    exit 0
}

# Trap SIGINT (Ctrl+C) and SIGTERM to call cleanup
trap cleanup SIGINT SIGTERM

# Function to run a single prove proxy instance
run_prove_proxy() {
    local instance_id=$1
    local port=$2
    local log_file=$3
    local service_name="prove-proxy-${instance_id}"

    print_message "$BLUE" "[$(date '+%Y-%m-%d %H:%M:%S')] Starting $service_name on port $port (logging to $log_file)..."

    while true; do
        # Run service with unbuffered output and append both stdout and stderr to log file
        RUST_LOG=$LOG_LEVEL stdbuf -oL -eL ./target/${PROFILE}/qed_user_cli prove-proxy \
            --listen-addr "0.0.0.0:${port}" 2>&1 | \
            stdbuf -oL sed 's/\x1b\[[0-9;]*m//g' | \
            while IFS= read -r line; do
                echo "[${service_name}] $line"
            done >> "$log_file" &

        local pid=$!
        PIDS+=("$pid")

        # Wait for the process
        wait $pid

        print_message "$YELLOW" "[$(date '+%Y-%m-%d %H:%M:%S')] $service_name (PID: $pid, Port: $port) stopped. Restarting in 5 seconds..." | tee -a "$log_file"
        sleep 5

        # Remove the dead PID from array
        PIDS=("${PIDS[@]/$pid}")
    done
}

# Function to check if a port is available
check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        return 1  # Port is in use
    else
        return 0  # Port is available
    fi
}

# Function to display status
show_status() {
    print_message "$YELLOW" "\n===== Prove Proxy Status ====="
    printf "${BLUE}%-15s %-10s %-35s${NC}\n" "Instance" "Port" "Log File"
    printf "%-15s %-10s %-35s\n" "--------" "----" "--------"

    for i in $(seq 0 $((NUM_PROXIES - 1))); do
        local port=${PORTS[$i]}
        local log_file=${LOG_FILES[$i]}
        local instance_name="proxy-$i"

        if check_port $port; then
            printf "%-15s ${RED}%-10s${NC} %-35s\n" "$instance_name" "$port" "$log_file"
        else
            printf "%-15s ${GREEN}%-10s${NC} %-35s\n" "$instance_name" "$port ✓" "$log_file"
        fi
    done
    echo ""
}

# Function to tail all logs
tail_all_logs() {
    print_message "$YELLOW" "Tailing logs from all prove proxies (Ctrl+C to stop)..."

    # Build the tail command with all log files
    local log_files=""
    for log_file in "${LOG_FILES[@]}"; do
        if [ -f "$log_file" ]; then
            log_files="$log_files $log_file"
        fi
    done

    if [ -n "$log_files" ]; then
        tail -F $log_files 2>/dev/null
    else
        print_message "$RED" "No log files found yet."
    fi
}

# Function to display help
show_help() {
    cat << EOF
${GREEN}QED Prove Proxy Manager${NC}

${YELLOW}Usage:${NC}
    $0 [NUM_PROXIES] [BASE_PORT]

${YELLOW}Arguments:${NC}
    NUM_PROXIES    Number of prove proxy instances to run (default: 3)
    BASE_PORT      Starting port number (default: 9090)

${YELLOW}Environment Variables:${NC}
    LOG_LEVEL      Rust log level (default: qed_rollup_utils=trace,qed_user_cli=debug,qed_prover=trace)
    PROFILE        Build profile to use (default: release)

${YELLOW}Examples:${NC}
    $0             # Run 3 proxies on ports 9090-9092
    $0 5           # Run 5 proxies on ports 9090-9094
    $0 5 8080      # Run 5 proxies on ports 8080-8084
    $0 10 7000     # Run 10 proxies on ports 7000-7009

${YELLOW}Features:${NC}
    • Automatic restart on failure
    • Separate log files for each instance
    • Port conflict detection
    • Graceful shutdown with Ctrl+C
    • Status display showing all instances

${YELLOW}Log Files:${NC}
    Logs are stored in: ${LOG_DIR}/
    Each instance has its own log file: prove_proxy_<instance_id>.log

${YELLOW}Commands During Runtime:${NC}
    Ctrl+C         Stop all prove proxy instances

${YELLOW}Monitoring:${NC}
    To tail logs in another terminal:
    tail -f ${LOG_DIR}/prove_proxy_*.log

    To check specific instance:
    tail -f ${LOG_DIR}/prove_proxy_0.log

EOF
}

# Main execution
main() {
    # Parse arguments
    if [ "$1" = "-h" ] || [ "$1" = "--help" ] || [ "$1" = "help" ]; then
        show_help
        exit 0
    fi

    # Validate NUM_PROXIES
    if ! [[ "$NUM_PROXIES" =~ ^[0-9]+$ ]] || [ "$NUM_PROXIES" -lt 1 ]; then
        print_message "$RED" "Error: Number of proxies must be a positive integer"
        show_help
        exit 1
    fi

    # Validate BASE_PORT
    if ! [[ "$BASE_PORT" =~ ^[0-9]+$ ]] || [ "$BASE_PORT" -lt 1024 ] || [ "$BASE_PORT" -gt 65535 ]; then
        print_message "$RED" "Error: Base port must be between 1024 and 65535"
        show_help
        exit 1
    fi

    # Check if binary exists
    if [ ! -f "./target/${PROFILE}/qed_user_cli" ]; then
        print_message "$RED" "Error: Binary ./target/${PROFILE}/qed_user_cli not found"
        print_message "$YELLOW" "Please run 'make build' first"
        exit 1
    fi

    # Display configuration
    print_message "$GREEN" "========================================="
    print_message "$GREEN" "     QED Prove Proxy Manager"
    print_message "$GREEN" "========================================="
    print_message "$YELLOW" "\nConfiguration:"
    echo "  Number of proxies: ${NUM_PROXIES}"
    echo "  Port range: ${BASE_PORT}-$((BASE_PORT + NUM_PROXIES - 1))"
    echo "  Log directory: ${LOG_DIR}"
    echo "  Profile: ${PROFILE}"
    echo "  Log level: ${LOG_LEVEL}"
    echo ""

    # Check for port conflicts
    print_message "$YELLOW" "Checking port availability..."
    local conflicts=0
    for i in $(seq 0 $((NUM_PROXIES - 1))); do
        local port=$((BASE_PORT + i))
        if ! check_port $port; then
            print_message "$RED" "  Port $port is already in use!"
            conflicts=$((conflicts + 1))
        else
            print_message "$GREEN" "  Port $port is available ✓"
        fi
    done

    if [ $conflicts -gt 0 ]; then
        print_message "$RED" "\nError: $conflicts port(s) are already in use. Please choose a different port range."
        exit 1
    fi

    # Clear old log files
    print_message "$YELLOW" "\nClearing old log files..."
    for i in $(seq 0 $((NUM_PROXIES - 1))); do
        local log_file="$LOG_DIR/prove_proxy_${i}.log"
        : > "$log_file"
        LOG_FILES+=("$log_file")
    done

    # Start all prove proxy instances
    print_message "$GREEN" "\nStarting prove proxy instances..."
    for i in $(seq 0 $((NUM_PROXIES - 1))); do
        local port=$((BASE_PORT + i))
        local log_file="$LOG_DIR/prove_proxy_${i}.log"

        PORTS+=($port)

        # Start the prove proxy in background
        run_prove_proxy $i $port "$log_file" &

        # Small delay between starts to avoid race conditions
        sleep 0.5
    done

    # Wait a bit for services to start
    sleep 2

    # Show status
    show_status

    print_message "$GREEN" "✓ All prove proxy instances started successfully!"
    print_message "$YELLOW" "\nUseful commands:"
    echo "  • View logs: tail -f ${LOG_DIR}/prove_proxy_*.log"
    echo "  • Check specific instance: tail -f ${LOG_DIR}/prove_proxy_0.log"
    echo "  • Stop all: Ctrl+C"
    echo ""
    print_message "$BLUE" "Monitoring all instances. Press Ctrl+C to stop all services."

    # Optional: Show a simple monitoring view
    if [ -t 0 ]; then  # Check if running in interactive terminal
        echo ""
        echo "Press 'l' to tail logs, 's' to show status, or Ctrl+C to exit"
        while true; do
            read -t 1 -n 1 key
            case $key in
                l|L)
                    tail_all_logs
                    ;;
                s|S)
                    show_status
                    ;;
            esac
        done
    else
        # Wait for all background processes
        wait
    fi
}

# Run main function
main "$@"