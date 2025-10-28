#!/bin/bash

# Professional Installation Script for QED Prove Proxy Manager
# Automatically copies binaries and scripts to appropriate system directories

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Standard installation paths (FHS compliant)
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
BIN_DIR="${INSTALL_PREFIX}/bin"
SBIN_DIR="${INSTALL_PREFIX}/sbin"
ETC_DIR="/etc/qed"
LIB_DIR="${INSTALL_PREFIX}/lib/qed"
VAR_DIR="/var/lib/qed"
RUN_DIR="/var/run/qed_prove_proxy"
LOG_DIR="/var/log/qed_prove_proxy"
SYSTEMD_DIR="/etc/systemd/system"

# Default configuration
DEFAULT_SERVICE_USER="qed"
DEFAULT_SERVICE_GROUP="qed"
DEFAULT_NUM_PROXIES="3"
DEFAULT_BASE_PORT="9090"
DEFAULT_BINARY_NAME="qed_user_cli"

# Parse command line arguments
show_help() {
    cat << EOF
${GREEN}QED Prove Proxy Manager - Professional Installer${NC}

Usage: $0 [OPTIONS] <BINARY_PATH>

Arguments:
    BINARY_PATH         Path to the qed_user_cli binary to install

Options:
    --user USER         User to run service as (default: qed)
    --group GROUP       Group for service user (default: qed)
    --create-user       Create system user if it doesn't exist
    --num-proxies N     Number of proxy instances (default: 3)
    --base-port PORT    Starting port number (default: 9090)
    --qed-config-path PATH  Path to config.json for RPC; will be installed to /etc/qed/config.json
    --prefix PREFIX     Installation prefix (default: /usr/local)
    --uninstall         Remove the installation
    --upgrade           Upgrade existing installation (preserves config)
    --dry-run           Show what would be done without making changes
    -h, --help          Show this help message

Examples:
    # Standard installation
    sudo $0 /path/to/qed_user_cli --create-user

    # Custom installation
    sudo $0 ./qed_user_cli --user myuser --num-proxies 5 --base-port 8080

    # Upgrade existing installation
    sudo $0 /new/version/qed_user_cli --upgrade

    # Uninstall
    sudo $0 --uninstall

Directory Structure After Installation:
    ${BIN_DIR}/              # User binaries
    ├── qed_user_cli         # Main binary

    ${SBIN_DIR}/             # System binaries
    ├── qed_prove_proxy_manager  # Manager script

    ${ETC_DIR}/              # Configuration
    ├── prove_proxy_config.conf  # Main config

    ${VAR_DIR}/              # Variable data
    ${RUN_DIR}/              # PID files
    ${LOG_DIR}/              # Log files

EOF
}

# Variables for parsed arguments
BINARY_PATH=""
SERVICE_USER="$DEFAULT_SERVICE_USER"
SERVICE_GROUP="$DEFAULT_SERVICE_GROUP"
CREATE_USER=false
NUM_PROXIES="$DEFAULT_NUM_PROXIES"
BASE_PORT="$DEFAULT_BASE_PORT"
DRY_RUN=false
UNINSTALL=false
UPGRADE=false
QED_CONFIG_PATH=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --user)
            SERVICE_USER="$2"
            shift 2
            ;;
        --group)
            SERVICE_GROUP="$2"
            shift 2
            ;;
        --create-user)
            CREATE_USER=true
            shift
            ;;
        --num-proxies)
            NUM_PROXIES="$2"
            shift 2
            ;;
        --base-port)
            BASE_PORT="$2"
            shift 2
            ;;
        --prefix)
            INSTALL_PREFIX="$2"
            BIN_DIR="${INSTALL_PREFIX}/bin"
            SBIN_DIR="${INSTALL_PREFIX}/sbin"
            LIB_DIR="${INSTALL_PREFIX}/lib/qed"
            shift 2
            ;;
        --qed-config-path)
            QED_CONFIG_PATH="$2"
            shift 2
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --upgrade)
            UPGRADE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        -*)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
        *)
            BINARY_PATH="$1"
            shift
            ;;
    esac
done

# Function to execute or display command
run_cmd() {
    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY-RUN]${NC} $@"
        return 0
    else
        "$@"
    fi
}

# Function to copy file with backup
copy_with_backup() {
    local src="$1"
    local dst="$2"

    if [ -f "$dst" ] && [ "$UPGRADE" = true ]; then
        echo -e "${YELLOW}  Backing up existing $(basename $dst)${NC}"
        run_cmd cp "$dst" "${dst}.bak.$(date +%Y%m%d_%H%M%S)"
    fi

    run_cmd cp "$src" "$dst"
}

# Uninstall function
perform_uninstall() {
    echo -e "${YELLOW}Uninstalling QED Prove Proxy Manager...${NC}"

    # Stop service
    if systemctl is-active --quiet qed_prove_proxy_manager; then
        echo "Stopping service..."
        run_cmd systemctl stop qed_prove_proxy_manager
    fi

    # Disable service
    if systemctl is-enabled --quiet qed_prove_proxy_manager 2>/dev/null; then
        echo "Disabling service..."
        run_cmd systemctl disable qed_prove_proxy_manager
    fi

    # Remove files
    echo "Removing installed files..."
    run_cmd rm -f "${BIN_DIR}/qed_user_cli"
    run_cmd rm -f "${SBIN_DIR}/qed_prove_proxy_manager"
    run_cmd rm -f "${SYSTEMD_DIR}/qed_prove_proxy_manager.service"
    run_cmd rm -f /etc/logrotate.d/qed_prove_proxy

    # Optional: Remove config and logs
    echo -e "${YELLOW}Remove configuration and logs? (y/N)${NC}"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        run_cmd rm -rf "$ETC_DIR"
        run_cmd rm -rf "$LOG_DIR"
        run_cmd rm -rf "$RUN_DIR"
        run_cmd rm -rf "$VAR_DIR"
    else
        echo "Keeping configuration and logs"
    fi

    echo -e "${GREEN}Uninstallation complete${NC}"
    exit 0
}

# Check for uninstall
if [ "$UNINSTALL" = true ]; then
    perform_uninstall
fi

# Validate inputs
if [ -z "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary path not specified${NC}"
    echo "Usage: $0 [OPTIONS] <BINARY_PATH>"
    echo "Use -h or --help for more information"
    exit 1
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

# Validate rpc-config file if provided
if [ -n "$QED_CONFIG_PATH" ]; then
    if [ ! -f "$QED_CONFIG_PATH" ]; then
        echo -e "${RED}Error: QED config file not found at $QED_CONFIG_PATH${NC}"
        exit 1
    fi
fi
# Check if running as root
if [ "$EUID" -ne 0 ] && [ "$DRY_RUN" != true ]; then
    echo -e "${RED}This installer requires root privileges${NC}"
    echo "Please run with sudo"
    exit 1
fi

# Installation header
echo -e "${GREEN}═══════════════════════════════════════════════${NC}"
echo -e "${GREEN}   QED Prove Proxy Manager - Installation${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}Installation Plan:${NC}"
echo "  Binary source: $BINARY_PATH"
echo "  Install prefix: $INSTALL_PREFIX"
echo "  Service user: $SERVICE_USER"
echo "  Service group: $SERVICE_GROUP"
echo "  Number of proxies: $NUM_PROXIES"
echo "  Port range: ${BASE_PORT}-$((BASE_PORT + NUM_PROXIES - 1))"
if [ -n "$QED_CONFIG_PATH" ]; then
    echo "  QED config source: $QED_CONFIG_PATH"
    echo "  QED config dest:   $ETC_DIR/config.json"
fi
echo ""

echo -e "${BLUE}Directories:${NC}"
echo "  Binaries: $BIN_DIR"
echo "  Scripts: $SBIN_DIR"
echo "  Config: $ETC_DIR"
echo "  Logs: $LOG_DIR"
echo "  Runtime: $RUN_DIR"
echo ""

if [ "$DRY_RUN" != true ]; then
    echo -e "${YELLOW}Press Enter to continue or Ctrl+C to abort...${NC}"
    read
fi

# Create system user if requested
if [ "$CREATE_USER" = true ]; then
    if ! id "$SERVICE_USER" &>/dev/null; then
        echo -e "${YELLOW}Creating system user '$SERVICE_USER'...${NC}"
        run_cmd useradd --system --shell /bin/false --home "$VAR_DIR" \
            --comment "QED Prove Proxy Service" "$SERVICE_USER"
        echo -e "${GREEN}✓ User created${NC}"
    else
        echo -e "${BLUE}User '$SERVICE_USER' already exists${NC}"
    fi
fi

# Verify user exists
if ! id "$SERVICE_USER" &>/dev/null && [ "$DRY_RUN" != true ]; then
    echo -e "${RED}Error: User '$SERVICE_USER' does not exist${NC}"
    echo "Use --create-user to create it automatically"
    exit 1
fi

# Create directory structure
echo -e "${YELLOW}Creating directory structure...${NC}"
run_cmd mkdir -p "$BIN_DIR"
run_cmd mkdir -p "$SBIN_DIR"
run_cmd mkdir -p "$ETC_DIR"
run_cmd mkdir -p "$LIB_DIR"
run_cmd mkdir -p "$VAR_DIR"
run_cmd mkdir -p "$LOG_DIR"

# Set directory permissions
run_cmd chown "$SERVICE_USER:$SERVICE_GROUP" "$VAR_DIR"
run_cmd chown "$SERVICE_USER:$SERVICE_GROUP" "$LOG_DIR"

echo -e "${GREEN}✓ Directories created${NC}"

# Install binary
echo -e "${YELLOW}Installing binary...${NC}"
echo "  $BINARY_PATH → $BIN_DIR/qed_user_cli"
copy_with_backup "$BINARY_PATH" "$BIN_DIR/qed_user_cli"
run_cmd chmod 755 "$BIN_DIR/qed_user_cli"
echo -e "${GREEN}✓ Binary installed${NC}"

# Create manager script
echo -e "${YELLOW}Creating manager script...${NC}"

cat > /tmp/qed_prove_proxy_manager << 'MANAGER_SCRIPT_EOF'
#!/bin/bash

# QED Prove Proxy Manager
# Installed by professional installer

set -e

# Configuration file location
CONFIG_FILE="/etc/qed/prove_proxy_config.conf"

# Load configuration
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Configuration file not found at $CONFIG_FILE"
    exit 1
fi

source "$CONFIG_FILE"

# Set paths from configuration
PID_FILE="${RUN_DIR}/manager.pid"
MANAGER_LOG="${LOG_DIR}/manager.log"

# Ensure directories exist
mkdir -p "$RUN_DIR"
mkdir -p "$LOG_DIR"

# Function to log
log_to_file() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$MANAGER_LOG"
}

# Array for child PIDs
declare -a CHILD_PIDS=()

# Check port availability
check_port() {
    local port=$1
    lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1 && return 1 || return 0
}

# Run single proxy instance with auto-restart
run_prove_proxy() {
    local instance_id=$1
    local port=$2
    local log_file="${LOG_DIR}/prove_proxy_${instance_id}.log"
    local pid_file="${RUN_DIR}/prove_proxy_${instance_id}.pid"

    while true; do
        if [ -f "${RUN_DIR}/stop" ]; then
            log_to_file "Stop signal received for proxy-${instance_id}"
            break
        fi

        log_to_file "Starting proxy-${instance_id} on port $port"

        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting on port $port" >> "$log_file"

        RUST_LOG=$LOG_LEVEL "$BINARY_PATH" prove-proxy \
            --listen-addr "0.0.0.0:${port}" \
            --rpc-config "${QED_CONFIG_PATH}" >> "$log_file" 2>&1 &

        local proxy_pid=$!
        echo $proxy_pid > "$pid_file"

        wait $proxy_pid
        local exit_code=$?

        rm -f "$pid_file"

        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Exited with code $exit_code" >> "$log_file"
        log_to_file "proxy-${instance_id} stopped with exit code $exit_code"

        if [ -f "${RUN_DIR}/stop" ]; then
            break
        fi

        log_to_file "Restarting proxy-${instance_id} in ${RESTART_DELAY} seconds..."
        sleep $RESTART_DELAY
    done
}

# Cleanup function
cleanup() {
    log_to_file "Shutting down manager"
    touch "${RUN_DIR}/stop"

    for pid_file in "${RUN_DIR}"/prove_proxy_*.pid; do
        if [ -f "$pid_file" ]; then
            local pid=$(cat "$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                kill -TERM "$pid" 2>/dev/null || true
            fi
            rm -f "$pid_file"
        fi
    done

    for pid in "${CHILD_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid" 2>/dev/null || true
        fi
    done

    sleep 2
    pkill -f "qed_user_cli prove-proxy" 2>/dev/null || true

    rm -f "${RUN_DIR}/stop"
    rm -f "$PID_FILE"
    log_to_file "Shutdown complete"
    exit 0
}

# Main
case "${1:-}" in
    start)
        if [ -f "$PID_FILE" ]; then
            old_pid=$(cat "$PID_FILE")
            if kill -0 "$old_pid" 2>/dev/null; then
                echo "Manager already running with PID $old_pid"
                exit 1
            fi
        fi

        rm -f "${RUN_DIR}/stop"
        echo $$ > "$PID_FILE"
        trap cleanup SIGTERM SIGINT

        log_to_file "Manager starting with PID $$"
        echo "Starting QED Prove Proxy Manager..."
        echo "Configuration: $CONFIG_FILE"
        echo "Proxies: $NUM_PROXIES on ports ${BASE_PORT}-$((BASE_PORT + NUM_PROXIES - 1))"

        # Check ports
        for i in $(seq 0 $((NUM_PROXIES - 1))); do
            port=$((BASE_PORT + i))
            if ! check_port $port; then
                echo "Error: Port $port is already in use"
                rm -f "$PID_FILE"
                exit 1
            fi
        done

        # Start proxies
        for i in $(seq 0 $((NUM_PROXIES - 1))); do
            port=$((BASE_PORT + i))
            run_prove_proxy $i $port &
            CHILD_PIDS+=($!)
            sleep $START_DELAY
        done

        echo "All proxies started successfully"
        log_to_file "All $NUM_PROXIES proxies started"
        wait
        ;;

    stop)
        if [ -f "$PID_FILE" ]; then
            pid=$(cat "$PID_FILE")
            if kill -0 "$pid" 2>/dev/null; then
                echo "Stopping manager..."
                kill -TERM "$pid"
            else
                echo "Manager not running (stale PID)"
                rm -f "$PID_FILE"
            fi
        else
            echo "Manager is not running"
        fi
        ;;

    restart)
        $0 stop
        sleep 2
        $0 start
        ;;

    status)
        if [ -f "$PID_FILE" ]; then
            pid=$(cat "$PID_FILE")
            if kill -0 "$pid" 2>/dev/null; then
                echo "Manager is running (PID: $pid)"
                echo ""
                echo "Proxy Status:"
                for i in $(seq 0 $((NUM_PROXIES - 1))); do
                    port=$((BASE_PORT + i))
                    pid_file="${RUN_DIR}/prove_proxy_${i}.pid"
                    if [ -f "$pid_file" ] && kill -0 $(cat "$pid_file") 2>/dev/null; then
                        echo "  proxy-$i (port $port): Running"
                    else
                        echo "  proxy-$i (port $port): Stopped"
                    fi
                done
            else
                echo "Manager not running (stale PID file)"
            fi
        else
            echo "Manager is not running"
        fi
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|status}"
        exit 1
        ;;
esac
MANAGER_SCRIPT_EOF

if [ "$DRY_RUN" != true ]; then
    mv /tmp/qed_prove_proxy_manager "$SBIN_DIR/qed_prove_proxy_manager"
fi
run_cmd chmod 755 "$SBIN_DIR/qed_prove_proxy_manager"
echo -e "${GREEN}✓ Manager script installed${NC}"

# Install RPC config if specified
if [ -n "$QED_CONFIG_PATH" ]; then
    echo -e "${YELLOW}Installing QED config file...${NC}"
    run_cmd cp "$QED_CONFIG_PATH" "$ETC_DIR/config.json"
    run_cmd chmod 644 "$ETC_DIR/config.json"
    echo -e "${GREEN}✅ QED config installed at $ETC_DIR/config.json${NC}"
else
    if [ -f "$ETC_DIR/config.json" ]; then
        echo -e "${BLUE}Using existing RPC config at $ETC_DIR/config.json${NC}"
    else
        echo -e "${RED}Error: No --qed-config-path provided and no $ETC_DIR/config.json found.${NC}"
        echo -e "${RED}         The service will rely on the binary's default 'config.json' relative path.${NC}"
    fi
fi

# Create configuration file
echo -e "${YELLOW}Creating configuration file...${NC}"

if [ -f "$ETC_DIR/prove_proxy_config.conf" ] && [ "$UPGRADE" = true ]; then
    echo -e "${YELLOW}  Keeping existing configuration${NC}"
else
    cat > /tmp/prove_proxy_config.conf << EOF
# QED Prove Proxy Configuration
# Generated by installer on $(date)

# Installation paths (DO NOT MODIFY - set by installer)
BINARY_PATH="$BIN_DIR/qed_user_cli"
RUN_DIR="$RUN_DIR"
LOG_DIR="$LOG_DIR"
CONFIG_DIR="$ETC_DIR"
VAR_DIR="$VAR_DIR"

# Service configuration
SERVICE_USER="$SERVICE_USER"
SERVICE_GROUP="$SERVICE_GROUP"

# RPC Configuration
QED_CONFIG_PATH="$ETC_DIR/config.json"

# Proxy settings (MODIFY AS NEEDED)
NUM_PROXIES=$NUM_PROXIES
BASE_PORT=$BASE_PORT
#LOG_LEVEL="qed_rollup_utils=trace,qed_user_cli=debug,psy_prover=trace"
LOG_LEVEL="qed_rollup_utils=trace,tikv_client=warn,psy_store=trace,qed_user_cli=debug,qed_dev_cli=debug,qed_api_services=info,qed_rollup_cli=debug,psy_node=trace,psy_common_circuit=trace,psy_network_circuit=trace,psy_prover=trace,psy_data=trace,plonky2=error"

RESTART_DELAY=5
START_DELAY=0.5
BOOT_DELAY=60

# Resource limits
MAX_OPEN_FILES=65536
RESTART_LIMIT_INTERVAL=200
RESTART_LIMIT_BURST=5
TIMEOUT_START_SEC=90
TIMEOUT_STOP_SEC=90
EOF

    if [ "$DRY_RUN" != true ]; then
        mv /tmp/prove_proxy_config.conf "$ETC_DIR/prove_proxy_config.conf"
    fi
    run_cmd chmod 644 "$ETC_DIR/prove_proxy_config.conf"
    echo -e "${GREEN}✓ Configuration file created${NC}"
fi

# Create systemd service
echo -e "${YELLOW}Creating systemd service...${NC}"

# Determine if boot delay should be applied
# Only apply boot delay for system startup, not manual starts
cat > /tmp/qed_prove_proxy_manager.service << EOF
[Unit]
Description=QED Prove Proxy Manager Service
After=network-online.target
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
PIDFile=$RUN_DIR/manager.pid
User=$SERVICE_USER
Group=$SERVICE_GROUP

RuntimeDirectory=qed_prove_proxy
RuntimeDirectoryMode=0755

Environment="RUST_BACKTRACE=1"
Environment="HOME=$VAR_DIR"

# Main commands
ExecStart=/usr/local/sbin/qed_prove_proxy_manager start
ExecStop=/usr/local/sbin/qed_prove_proxy_manager stop
ExecReload=/bin/kill -HUP \$MAINPID

# Restart policy
Restart=on-failure
RestartSec=10
StartLimitInterval=200
StartLimitBurst=5

# Security
PrivateTmp=true
NoNewPrivileges=true
ProtectHome=true
ReadWritePaths=$LOG_DIR $VAR_DIR

# Resource limits
LimitNOFILE=65536
LimitCORE=0

# Timeouts
TimeoutStartSec=60
TimeoutStopSec=90

[Install]
WantedBy=multi-user.target
EOF

if [ "$DRY_RUN" != true ]; then
    mv /tmp/qed_prove_proxy_manager.service "$SYSTEMD_DIR/qed_prove_proxy_manager.service"
fi
echo -e "${GREEN}✓ Systemd service created${NC}"

# Create logrotate configuration
echo -e "${YELLOW}Setting up log rotation...${NC}"

cat > /tmp/qed_prove_proxy_logrotate << EOF
$LOG_DIR/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0644 $SERVICE_USER $SERVICE_GROUP
    sharedscripts
    postrotate
        systemctl reload qed_prove_proxy_manager > /dev/null 2>&1 || true
    endscript
}
EOF

if [ "$DRY_RUN" != true ]; then
    mv /tmp/qed_prove_proxy_logrotate /etc/logrotate.d/qed_prove_proxy
fi
echo -e "${GREEN}✓ Log rotation configured${NC}"

# Reload systemd and enable service
if [ "$DRY_RUN" != true ]; then
    echo -e "${YELLOW}Enabling service...${NC}"
    systemctl daemon-reload
    systemctl enable qed_prove_proxy_manager.service
    echo -e "${GREEN}✓ Service enabled for auto-start${NC}"
fi

# Installation complete
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════${NC}"
echo -e "${GREEN}       Installation Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}Installed Files:${NC}"
echo "  Binary: $BIN_DIR/qed_user_cli"
echo "  Manager: $SBIN_DIR/qed_prove_proxy_manager"
echo "  Config: $ETC_DIR/prove_proxy_config.conf"
echo "  Service: $SYSTEMD_DIR/qed_prove_proxy_manager.service"
echo ""
echo -e "${BLUE}Service Commands:${NC}"
echo "  Start:    sudo systemctl start qed_prove_proxy_manager"
echo "  Stop:     sudo systemctl stop qed_prove_proxy_manager"
echo "  Status:   sudo systemctl status qed_prove_proxy_manager"
echo "  Logs:     sudo journalctl -u qed_prove_proxy_manager -f"
echo ""
echo -e "${BLUE}Management:${NC}"
echo "  Config:   sudo nano $ETC_DIR/prove_proxy_config.conf"
echo "  Logs:     tail -f $LOG_DIR/*.log"
echo "  Manual:   sudo $SBIN_DIR/qed_prove_proxy_manager {start|stop|status}"
echo ""

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}This was a dry run. No changes were made.${NC}"
else
    echo -e "${GREEN}The service is ready to start!${NC}"
    echo -e "${GREEN}Run: sudo systemctl start qed_prove_proxy_manager${NC}"
fi

# Automatically start and verify the service
if [ "$DRY_RUN" != true ]; then
    echo -e "${YELLOW}Starting QED Prove Proxy Manager service...${NC}"
    systemctl enable --now qed_prove_proxy_manager.service
    sleep 2
    systemctl status qed_prove_proxy_manager --no-pager -l || true
    echo -e "${GREEN}✅ Service started successfully${NC}"
else
    echo -e "${YELLOW}[DRY-RUN] Would have started qed_prove_proxy_manager.service${NC}"
fi