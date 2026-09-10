#!/bin/sh
# Komodo Linux Periphery Installer & Lifecycle Manager
# Supports systemd, OpenRC (Alpine Linux), Docker, and Podman
# Repository: https://github.com/ABHIMANYU993/komodo

set -e

CORE_ADDRESS=""
CONNECT_AS=""
ONBOARDING_KEY=""
POLLING_RATE="1-sec"
ACTION="install"
ROOT_DIR="/etc/komodo"
FORCE=0
PURGE=0
DOCKER_IMAGE="ghcr.io/abhimanyu993/komodo-periphery:2"

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log_info() { printf "${BLUE}%s${NC}\n" "$1"; }
log_success() { printf "${GREEN}%s${NC}\n" "$1"; }
log_warn() { printf "${YELLOW}%s${NC}\n" "$1"; }
log_err() { printf "${RED}%s${NC}\n" "$1" >&2; }

print_help() {
    cat <<EOF
Komodo Linux Periphery Installer & Lifecycle Manager

Usage:
  setup-periphery.sh [ACTION] [OPTIONS]

Actions:
  --install                 Install or configure Periphery (default)
  --reinstall               Fresh reinstall: wipe keys and re-onboard
  --reconfig                Update configuration (Core address, node name) and restart
  --restart                 Restart running Periphery service
  --status                  Show current running status
  --uninstall               Uninstall Periphery service and binary/container
  --purge                   Used with --uninstall to remove /etc/komodo completely

Options:
  --core-address=<url>      WebSocket URL of Komodo Core (e.g. ws://192.168.31.100:9120)
  --onboarding-key=<key>    One-time onboarding token
  --connect-as=<name>       Server identifier name (defaults to hostname)
  --polling-rate=<rate>     Stats polling rate (default: 1-sec)
  --root-directory=<path>   Periphery root directory (default: /etc/komodo)
  --force                   Force reinstall even if already running
  -h, --help                Show this help message

Examples:
  # Install via curl:
  curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \\
    --core-address="ws://192.168.31.100:9120" \\
    --connect-as="Alpine_VM" \\
    --onboarding-key="YOUR_ONBOARDING_KEY"

  # Install via wget:
  wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \\
    --core-address="ws://192.168.31.100:9120" \\
    --connect-as="Ubuntu_Server" \\
    --onboarding-key="YOUR_ONBOARDING_KEY"
EOF
}

# Parse options
while [ $# -gt 0 ]; do
    case "$1" in
        --install) ACTION="install" ;;
        --reinstall) ACTION="reinstall" ;;
        --reconfig|--update-config) ACTION="reconfig" ;;
        --restart) ACTION="restart" ;;
        --status) ACTION="status" ;;
        --uninstall) ACTION="uninstall" ;;
        --purge) PURGE=1 ;;
        --core-address=*|--core=*) CORE_ADDRESS="${1#*=}" ;;
        --core-address|--core) CORE_ADDRESS="$2"; shift ;;
        --connect-as=*|--name=*) CONNECT_AS="${1#*=}" ;;
        --connect-as|--name) CONNECT_AS="$2"; shift ;;
        --onboarding-key=*|--token=*) ONBOARDING_KEY="${1#*=}" ;;
        --onboarding-key|--token) ONBOARDING_KEY="$2"; shift ;;
        --polling-rate=*|--stats-polling-rate=*) POLLING_RATE="${1#*=}" ;;
        --polling-rate|--stats-polling-rate) POLLING_RATE="$2"; shift ;;
        --root-directory=*) ROOT_DIR="${1#*=}" ;;
        --root-directory) ROOT_DIR="$2"; shift ;;
        --force) FORCE=1 ;;
        -h|--help) print_help; exit 0 ;;
        *) log_err "Unknown option: $1"; print_help; exit 1 ;;
    esac
    shift
done

# Root check
if [ "$(id -u 2>/dev/null || echo 1)" -ne 0 ]; then
    log_err "Error: Root access is required. Run as root or with sudo."
    exit 1
fi

detect_init() {
    if command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
        echo "systemd"
    elif command -v rc-service >/dev/null 2>&1 || [ -f /sbin/openrc-run ] || [ -d /etc/init.d ]; then
        echo "openrc"
    elif command -v docker >/dev/null 2>&1; then
        echo "docker"
    elif command -v podman >/dev/null 2>&1; then
        echo "podman"
    else
        echo "generic"
    fi
}

INIT_SYS=$(detect_init)
IS_ALPINE=0
[ -f /etc/alpine-release ] && IS_ALPINE=1

log_info "Detected Init / Environment: $INIT_SYS (Alpine: $IS_ALPINE)"

CONFIG_FILE="$ROOT_DIR/periphery.config.toml"
KEYS_DIR="$ROOT_DIR/keys"

# ACTION: Status
if [ "$ACTION" = "status" ]; then
    log_info "=== Komodo Periphery Status ==="
    if [ "$INIT_SYS" = "systemd" ]; then
        systemctl status periphery || true
    elif [ "$INIT_SYS" = "openrc" ]; then
        rc-service periphery status || true
    fi
    if command -v docker >/dev/null 2>&1; then
        docker ps -f name=komodo-periphery
    fi
    if command -v podman >/dev/null 2>&1; then
        podman ps -f name=komodo-periphery
    fi
    exit 0
fi

# ACTION: Uninstall
if [ "$ACTION" = "uninstall" ]; then
    log_info "Uninstalling Komodo Periphery..."
    if [ "$INIT_SYS" = "systemd" ]; then
        systemctl stop periphery 2>/dev/null || true
        systemctl disable periphery 2>/dev/null || true
        rm -f /etc/systemd/system/periphery.service
        systemctl daemon-reload 2>/dev/null || true
    elif [ "$INIT_SYS" = "openrc" ]; then
        rc-service periphery stop 2>/dev/null || true
        rc-update del periphery default 2>/dev/null || true
        rm -f /etc/init.d/periphery
    fi
    command -v docker >/dev/null 2>&1 && docker rm -f komodo-periphery 2>/dev/null || true
    command -v podman >/dev/null 2>&1 && podman rm -f komodo-periphery 2>/dev/null || true
    rm -f /usr/local/bin/periphery
    if [ $PURGE -eq 1 ]; then
        rm -rf "$ROOT_DIR"
        log_success "Komodo Periphery completely purged from system."
    else
        log_success "Periphery service removed. Configuration preserved in $ROOT_DIR."
    fi
    exit 0
fi

# ACTION: Restart
if [ "$ACTION" = "restart" ]; then
    if [ "$INIT_SYS" = "systemd" ]; then
        systemctl restart periphery
    elif [ "$INIT_SYS" = "openrc" ]; then
        rc-service periphery restart
    else
        command -v docker >/dev/null 2>&1 && docker restart komodo-periphery
        command -v podman >/dev/null 2>&1 && podman restart komodo-periphery
    fi
    log_success "Restarted Periphery service."
    exit 0
fi

# Validation for install / reconfig
if [ -z "$CORE_ADDRESS" ]; then
    log_err "Error: Missing required argument --core-address (e.g. ws://192.168.31.100:9120)"
    print_help
    exit 1
fi

case "$CORE_ADDRESS" in
    http://*) CORE_ADDRESS="ws://${CORE_ADDRESS#http://}" ;;
    https://*) CORE_ADDRESS="wss://${CORE_ADDRESS#https://}" ;;
esac

if [ -z "$CONNECT_AS" ]; then
    CONNECT_AS="$(hostname 2>/dev/null || echo "linux-node")"
fi

case "$POLLING_RATE" in
    [0-9]*) echo "$POLLING_RATE" | grep -q -- "-sec" || POLLING_RATE="${POLLING_RATE}-sec" ;;
esac

if [ "$ACTION" = "reinstall" ]; then
    log_warn "Fresh reinstall requested. Purging previous authentication keys..."
    rm -rf "$KEYS_DIR"/* 2>/dev/null || true
fi

if [ -n "$ONBOARDING_KEY" ]; then
    rm -rf "$KEYS_DIR"/* 2>/dev/null || true
fi

mkdir -p "$ROOT_DIR" "$KEYS_DIR"
chmod 700 "$ROOT_DIR" "$KEYS_DIR"

# Write config file
cat > "$CONFIG_FILE" <<EOF
core_address = "$CORE_ADDRESS"
connect_as = "$CONNECT_AS"
root_directory = "$ROOT_DIR"
stats_polling_rate = "$POLLING_RATE"
include_disk_mounts = ["$ROOT_DIR", "/host", "/"]
EOF

if [ -n "$ONBOARDING_KEY" ]; then
    echo "onboarding_key = \"$ONBOARDING_KEY\"" >> "$CONFIG_FILE"
fi
chmod 600 "$CONFIG_FILE"

# Deployment Implementation
if [ "$IS_ALPINE" -eq 1 ] || [ "$INIT_SYS" = "openrc" ]; then
    log_info "Configuring Periphery for Alpine Linux / OpenRC..."
    if command -v docker >/dev/null 2>&1 || command -v podman >/dev/null 2>&1; then
        RUNTIME="docker"
        command -v podman >/dev/null 2>&1 && RUNTIME="podman"
        
        log_info "Deploying containerized Periphery via $RUNTIME..."
        $RUNTIME rm -f komodo-periphery 2>/dev/null || true
        
        ONBOARD_ENV=""
        [ -n "$ONBOARDING_KEY" ] && ONBOARD_ENV="-e PERIPHERY_ONBOARDING_KEY=$ONBOARDING_KEY"
        
        $RUNTIME run -d \
            --name komodo-periphery \
            --network host \
            --restart unless-stopped \
            -e PERIPHERY_CORE_ADDRESS="$CORE_ADDRESS" \
            -e PERIPHERY_CONNECT_AS="$CONNECT_AS" \
            $ONBOARD_ENV \
            -e PERIPHERY_STATS_POLLING_RATE="$POLLING_RATE" \
            -e PERIPHERY_INCLUDE_DISK_MOUNTS="$ROOT_DIR,/host,/" \
            -v /var/run/docker.sock:/var/run/docker.sock:ro \
            -v /proc:/proc:ro \
            -v "$KEYS_DIR:/config/keys" \
            -v "$ROOT_DIR:$ROOT_DIR" \
            "$DOCKER_IMAGE"

        # Create OpenRC service to track container state
        cat > /etc/init.d/periphery <<EOF
#!/sbin/openrc-run
name="Komodo Periphery"
description="Komodo Periphery Agent (Container)"

depend() {
    need net $RUNTIME
}

start() {
    ebegin "Starting Komodo Periphery"
    $RUNTIME start komodo-periphery 2>/dev/null || $RUNTIME run -d \\
        --name komodo-periphery \\
        --network host \\
        --restart unless-stopped \\
        -e PERIPHERY_CORE_ADDRESS="$CORE_ADDRESS" \\
        -e PERIPHERY_CONNECT_AS="$CONNECT_AS" \\
        $ONBOARD_ENV \\
        -e PERIPHERY_STATS_POLLING_RATE="$POLLING_RATE" \\
        -e PERIPHERY_INCLUDE_DISK_MOUNTS="$ROOT_DIR,/host,/" \\
        -v /var/run/docker.sock:/var/run/docker.sock:ro \\
        -v /proc:/proc:ro \\
        -v "$KEYS_DIR:/config/keys" \\
        -v "$ROOT_DIR:$ROOT_DIR" \\
        "$DOCKER_IMAGE"
    eend \$?
}

stop() {
    ebegin "Stopping Komodo Periphery"
    $RUNTIME stop komodo-periphery
    eend \$?
}
EOF
        chmod 755 /etc/init.d/periphery
        rc-update add periphery default 2>/dev/null || true
    else
        log_err "Error: Alpine Linux uses musl libc and requires Docker or Podman to run the Periphery agent."
        log_err "Please ensure Docker or Podman is installed and running on the host."
        exit 1
    fi
elif [ "$INIT_SYS" = "systemd" ]; then
    log_info "Configuring Periphery for systemd..."
    systemctl stop periphery 2>/dev/null || true

    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64) BIN_NAME="periphery-x86_64" ;;
        aarch64|arm64) BIN_NAME="periphery-aarch64" ;;
        *) BIN_NAME="periphery-x86_64" ;;
    esac

    BIN_PATH="/usr/local/bin/periphery"
    DL_URL="https://github.com/moghtech/komodo/releases/download/v1.17.2/$BIN_NAME"
    log_info "Downloading binary from $DL_URL..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DL_URL" -o "$BIN_PATH" || true
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$BIN_PATH" "$DL_URL" || true
    fi

    # Fallback to docker container if binary download fails or binary crashes
    if [ ! -s "$BIN_PATH" ] || ! chmod +x "$BIN_PATH" 2>/dev/null; then
        if command -v docker >/dev/null 2>&1 || command -v podman >/dev/null 2>&1; then
            log_warn "Binary unavailable; falling back to container deployment..."
            RUNTIME="docker"
            command -v podman >/dev/null 2>&1 && RUNTIME="podman"
            $RUNTIME rm -f komodo-periphery 2>/dev/null || true
            $RUNTIME run -d --name komodo-periphery --restart unless-stopped \
                -e PERIPHERY_CORE_ADDRESS="$CORE_ADDRESS" \
                -e PERIPHERY_CONNECT_AS="$CONNECT_AS" \
                -e PERIPHERY_STATS_POLLING_RATE="$POLLING_RATE" \
                -v /var/run/docker.sock:/var/run/docker.sock:ro \
                -v /proc:/proc:ro \
                -v "$ROOT_DIR:$ROOT_DIR" \
                "$DOCKER_IMAGE"
            exit 0
        fi
    fi

    # Systemd service file
    cat > /etc/systemd/system/periphery.service <<EOF
[Unit]
Description=Komodo Periphery Agent
After=network.target

[Service]
ExecStart=/usr/local/bin/periphery --config-path $CONFIG_FILE
Restart=always
RestartSec=5
TimeoutStartSec=0

[Install]
WantedBy=default.target
EOF

    systemctl daemon-reload
    systemctl enable periphery 2>/dev/null || true
    systemctl restart periphery
else
    # Generic container fallback
    if command -v docker >/dev/null 2>&1 || command -v podman >/dev/null 2>&1; then
        RUNTIME="docker"
        command -v podman >/dev/null 2>&1 && RUNTIME="podman"
        $RUNTIME rm -f komodo-periphery 2>/dev/null || true
        ONBOARD_ENV=""
        [ -n "$ONBOARDING_KEY" ] && ONBOARD_ENV="-e PERIPHERY_ONBOARDING_KEY=$ONBOARDING_KEY"
        $RUNTIME run -d --name komodo-periphery --network host --restart unless-stopped \
            -e PERIPHERY_CORE_ADDRESS="$CORE_ADDRESS" \
            -e PERIPHERY_CONNECT_AS="$CONNECT_AS" \
            $ONBOARD_ENV \
            -e PERIPHERY_STATS_POLLING_RATE="$POLLING_RATE" \
            -e PERIPHERY_INCLUDE_DISK_MOUNTS="$ROOT_DIR,/host,/" \
            -v /var/run/docker.sock:/var/run/docker.sock:ro \
            -v /proc:/proc:ro \
            -v "$KEYS_DIR:/config/keys" \
            -v "$ROOT_DIR:$ROOT_DIR" \
            "$DOCKER_IMAGE"
    fi
fi

log_success "=========================================================="
log_success "  SUCCESS: Komodo Periphery setup finished!"
log_success "  Node '$CONNECT_AS' configured to connect to '$CORE_ADDRESS'."
log_success "=========================================================="
