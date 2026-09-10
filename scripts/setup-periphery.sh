#!/bin/sh
# Komodo Linux Periphery Installer & Lifecycle Manager
# Strictly installs Periphery as a native system service (systemd, OpenRC, SysVinit, runit, s6, dinit)
# NO container fallback (Docker/Podman). Zero host mutations.
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
VERSION="v2.4.2"
BIN_URL=""
BIN_PATH_OVERRIDE=""
GITHUB_REPO="ABHIMANYU993/komodo"

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
Strictly installs Komodo Periphery as a native host system service.

Usage:
  setup-periphery.sh [ACTION] [OPTIONS]

Actions:
  --install                 Install or configure Periphery (default)
  --reinstall               Fresh reinstall: wipe keys, re-download, and re-onboard
  --reconfig                Update configuration (Core address, node name) and restart
  --update, --upgrade       Update to latest periphery binary (preserves config & keys)
  --restart                 Restart running Periphery service
  --status                  Show current running status of the Periphery service
  --uninstall               Uninstall Periphery service and binary
  --purge                   Used with --uninstall to remove /etc/komodo completely

Options:
  --core-address=<url>      WebSocket URL of Komodo Core (e.g. ws://192.168.31.100:9120)
  --onboarding-key=<key>    One-time onboarding token
  --connect-as=<name>       Server identifier name (defaults to hostname)
  --polling-rate=<rate>     Stats polling rate (default: 1-sec)
  --root-directory=<path>   Periphery root directory (default: /etc/komodo)
  --version=<tag>           Release tag to install (default: v2.4.2)
  --binary-url=<url>        Direct URL to precompiled periphery binary
  --binary-path=<path>      Local file path to precompiled periphery binary
  --force                   Force reinstall even if already running
  -h, --help                Show this help message

Supported Service Managers:
  - systemd      (/etc/systemd/system/periphery.service)
  - OpenRC       (/etc/init.d/periphery)
  - SysVinit     (/etc/init.d/periphery)
  - runit        (/etc/sv/periphery -> /var/service/periphery)
  - s6 / s6-rc   (/var/service/periphery or /etc/s6/services/periphery)
  - dinit        (/etc/dinit.d/periphery)

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
        --update|--upgrade) ACTION="update" ;;
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
        --version=*) VERSION="${1#*=}" ;;
        --version) VERSION="$2"; shift ;;
        --binary-url=*) BIN_URL="${1#*=}" ;;
        --binary-url) BIN_URL="$2"; shift ;;
        --binary-path=*) BIN_PATH_OVERRIDE="${1#*=}" ;;
        --binary-path) BIN_PATH_OVERRIDE="$2"; shift ;;
        --force) FORCE=1 ;;
        -h|--help) print_help; exit 0 ;;
        *) log_err "Unknown option: $1"; print_help; exit 1 ;;
    esac
    shift
done

# Root check
if [ "$(id -u 2>/dev/null || echo 1)" -ne 0 ]; then
    log_err "Error: Root access is required to manage system services. Run as root or with sudo/doas."
    exit 1
fi

# Detect system service manager
detect_init() {
    if command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
        echo "systemd"
    elif command -v rc-service >/dev/null 2>&1 || [ -f /sbin/openrc-run ] || ([ -d /etc/init.d ] && [ -f /etc/inittab ] && grep -q "openrc" /etc/inittab 2>/dev/null); then
        echo "openrc"
    elif command -v sv >/dev/null 2>&1 || [ -d /etc/runit ] || [ -d /var/service ]; then
        echo "runit"
    elif command -v s6-svc >/dev/null 2>&1 || command -v s6-rc >/dev/null 2>&1; then
        echo "s6"
    elif command -v dinitctl >/dev/null 2>&1 || [ -d /etc/dinit.d ]; then
        echo "dinit"
    elif [ -d /etc/init.d ] && (command -v update-rc.d >/dev/null 2>&1 || command -v chkconfig >/dev/null 2>&1 || command -v service >/dev/null 2>&1); then
        echo "sysvinit"
    else
        echo "none"
    fi
}

INIT_SYS=$(detect_init)

if [ "$INIT_SYS" = "none" ]; then
    log_err "=========================================================================="
    log_err "ERROR: No supported system service manager detected on this host!"
    log_err "Supported service managers: systemd, OpenRC, SysVinit, runit, s6, dinit."
    log_err "Komodo Periphery must run as a native system service. Container fallback is disabled."
    log_err "=========================================================================="
    exit 1
fi

log_info "Detected Host Service Manager: $INIT_SYS"

# Detect Libc
detect_libc() {
    if [ -f /etc/alpine-release ] || ldd --version 2>&1 | grep -iq musl || ls /lib/ld-musl-*.so* >/dev/null 2>&1 || ls /lib64/ld-musl-*.so* >/dev/null 2>&1; then
        echo "musl"
    else
        echo "gnu"
    fi
}

LIBC=$(detect_libc)
log_info "Detected C Library (libc): $LIBC"

CONFIG_FILE="$ROOT_DIR/periphery.config.toml"
KEYS_DIR="$ROOT_DIR/keys"
BIN_INSTALL_PATH="/usr/local/bin/periphery"

# Download helper using curl or wget (never mutating host packages)
download_to() {
    _url="$1"
    _dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$_url" -o "$_dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$_dest" "$_url"
    elif command -v busybox >/dev/null 2>&1 && busybox wget --help >/dev/null 2>&1; then
        busybox wget -qO "$_dest" "$_url"
    else
        log_err "Error: Neither curl nor wget is available on this host."
        log_err "Please ensure curl or wget is installed."
        return 1
    fi
}

# ACTION: Status
if [ "$ACTION" = "status" ]; then
    log_info "=== Komodo Periphery Service Status ($INIT_SYS) ==="
    case "$INIT_SYS" in
        systemd)
            systemctl status periphery --no-pager || true
            ;;
        openrc)
            rc-service periphery status || true
            ;;
        runit)
            sv status periphery || true
            ;;
        s6)
            s6-svstat /var/service/periphery 2>/dev/null || s6-svstat /etc/s6/services/periphery || true
            ;;
        dinit)
            dinitctl status periphery || true
            ;;
        sysvinit)
            /etc/init.d/periphery status || true
            ;;
    esac

    echo ""
    log_info "=== Binary & Config ==="
    if [ -x "$BIN_INSTALL_PATH" ]; then
        echo "Binary: $BIN_INSTALL_PATH ($("$BIN_INSTALL_PATH" --help 2>&1 | head -n 1 || echo "executable"))"
    else
        echo "Binary: NOT FOUND at $BIN_INSTALL_PATH"
    fi
    if [ -f "$CONFIG_FILE" ]; then
        echo "Config: $CONFIG_FILE"
        cat "$CONFIG_FILE"
    else
        echo "Config: NOT FOUND at $CONFIG_FILE"
    fi
    exit 0
fi

# ACTION: Stop Service
stop_service() {
    log_info "Gracefully stopping running periphery service/process..."
    case "$INIT_SYS" in
        systemd)
            systemctl stop periphery 2>/dev/null || true
            ;;
        openrc)
            rc-service periphery stop 2>/dev/null || true
            ;;
        runit)
            sv stop periphery 2>/dev/null || true
            ;;
        s6)
            s6-svc -d /var/service/periphery 2>/dev/null || s6-svc -d /etc/s6/services/periphery 2>/dev/null || true
            ;;
        dinit)
            dinitctl stop periphery 2>/dev/null || true
            ;;
        sysvinit)
            /etc/init.d/periphery stop 2>/dev/null || true
            ;;
    esac

    # Ensure all lingering periphery processes terminate gracefully and close sockets
    for pid in $(pgrep -x "periphery" 2>/dev/null || true); do
        if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
            kill -TERM "$pid" 2>/dev/null || true
        fi
    done

    # Wait up to 4s for process termination and clean socket closure
    for _i in 1 2 3 4; do
        if ! pgrep -x "periphery" >/dev/null 2>&1; then
            break
        fi
        sleep 1
    done

    # Force kill if still hung
    if pgrep -x "periphery" >/dev/null 2>&1; then
        kill -9 $(pgrep -x "periphery" 2>/dev/null) 2>/dev/null || true
    fi
    sleep 1
}

# ACTION: Start Service
start_service() {
    case "$INIT_SYS" in
        systemd)
            systemctl daemon-reload
            systemctl enable periphery 2>/dev/null || true
            systemctl restart periphery
            ;;
        openrc)
            chmod 755 /etc/init.d/periphery
            rc-update add periphery default 2>/dev/null || true
            rc-service periphery restart
            ;;
        runit)
            chmod +x /etc/sv/periphery/run
            if [ -d /var/service ] && [ ! -e /var/service/periphery ]; then
                ln -sf /etc/sv/periphery /var/service/periphery
            elif [ -d /run/runit/service ] && [ ! -e /run/runit/service/periphery ]; then
                ln -sf /etc/sv/periphery /run/runit/service/periphery
            fi
            sv restart periphery 2>/dev/null || sv start periphery
            ;;
        s6)
            s6-svc -u /var/service/periphery 2>/dev/null || s6-svc -u /etc/s6/services/periphery 2>/dev/null || true
            ;;
        dinit)
            dinitctl start periphery 2>/dev/null || dinitctl restart periphery
            ;;
        sysvinit)
            chmod 755 /etc/init.d/periphery
            update-rc.d periphery defaults 2>/dev/null || chkconfig --add periphery 2>/dev/null || true
            /etc/init.d/periphery restart
            ;;
    esac
}

# ACTION: Uninstall
if [ "$ACTION" = "uninstall" ]; then
    log_info "Uninstalling Komodo Periphery native service..."
    stop_service
    case "$INIT_SYS" in
        systemd)
            systemctl disable periphery 2>/dev/null || true
            rm -f /etc/systemd/system/periphery.service
            systemctl daemon-reload 2>/dev/null || true
            ;;
        openrc)
            rc-update del periphery default 2>/dev/null || true
            rm -f /etc/init.d/periphery
            ;;
        runit)
            rm -f /var/service/periphery /run/runit/service/periphery
            rm -rf /etc/sv/periphery
            ;;
        s6)
            rm -rf /var/service/periphery /etc/s6/services/periphery
            ;;
        dinit)
            rm -f /etc/dinit.d/periphery
            ;;
        sysvinit)
            update-rc.d -f periphery remove 2>/dev/null || chkconfig --del periphery 2>/dev/null || true
            rm -f /etc/init.d/periphery
            ;;
    esac

    rm -f "$BIN_INSTALL_PATH"

    if [ $PURGE -eq 1 ]; then
        rm -rf "$ROOT_DIR"
        log_success "Komodo Periphery completely purged from system."
    else
        log_success "Periphery service and binary removed. Configuration preserved in $ROOT_DIR."
    fi
    exit 0
fi

# ACTION: Restart
if [ "$ACTION" = "restart" ]; then
    log_info "Restarting Komodo Periphery service ($INIT_SYS)..."
    stop_service
    start_service
    log_success "Periphery service restarted."
    exit 0
fi

# If updating or existing config present, auto-populate CORE_ADDRESS if omitted
if [ -f "$CONFIG_FILE" ] && [ -z "$CORE_ADDRESS" ]; then
    CORE_ADDRESS=$(grep '^core_address' "$CONFIG_FILE" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || true)
    if [ -z "$CONNECT_AS" ]; then
        CONNECT_AS=$(grep '^connect_as' "$CONFIG_FILE" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || true)
    fi
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

# Gracefully stop running periphery service before modifying keys, binary, or config
stop_service

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

# Binary Installation
ARCH_RAW=$(uname -m)
case "$ARCH_RAW" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    armv7l|armhf) ARCH="armv7" ;;
    *) ARCH="x86_64" ;;
esac

install_binary() {
    mkdir -p /usr/local/bin

    if [ -n "$BIN_PATH_OVERRIDE" ] && [ -f "$BIN_PATH_OVERRIDE" ]; then
        log_info "Installing binary from local path: $BIN_PATH_OVERRIDE"
        cp -f "$BIN_PATH_OVERRIDE" "$BIN_INSTALL_PATH"
        chmod +x "$BIN_INSTALL_PATH"
        return 0
    fi

    if [ -n "$BIN_URL" ]; then
        log_info "Downloading binary from explicit URL: $BIN_URL"
        download_to "$BIN_URL" "$BIN_INSTALL_PATH"
        chmod +x "$BIN_INSTALL_PATH"
        return 0
    fi

    # Determine binary candidates based on libc and architecture
    # Static musl works everywhere (both Alpine and standard glibc Linux distros)
    if [ "$LIBC" = "musl" ]; then
        CANDIDATES="periphery-${ARCH}-musl periphery-${ARCH} periphery"
    else
        CANDIDATES="periphery-${ARCH} periphery-${ARCH}-musl periphery"
    fi

    INSTALLED=0
    for CANDIDATE in $CANDIDATES; do
        URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${CANDIDATE}"
        log_info "Attempting to download binary: $URL"
        if download_to "$URL" "$BIN_INSTALL_PATH" 2>/dev/null && [ -s "$BIN_INSTALL_PATH" ]; then
            chmod +x "$BIN_INSTALL_PATH"
            # Verify execution
            if "$BIN_INSTALL_PATH" --help >/dev/null 2>&1; then
                log_success "Successfully installed and verified binary ($CANDIDATE)."
                INSTALLED=1
                break
            else
                log_warn "Binary $CANDIDATE failed compatibility check (missing symbols/dynamic linker). Trying alternative..."
                rm -f "$BIN_INSTALL_PATH"
            fi
        fi
    done

    # Fallback to latest tag if versioned release download failed
    if [ $INSTALLED -eq 0 ]; then
        for CANDIDATE in $CANDIDATES; do
            URL="https://github.com/${GITHUB_REPO}/releases/latest/download/${CANDIDATE}"
            log_info "Attempting fallback download: $URL"
            if download_to "$URL" "$BIN_INSTALL_PATH" 2>/dev/null && [ -s "$BIN_INSTALL_PATH" ]; then
                chmod +x "$BIN_INSTALL_PATH"
                if "$BIN_INSTALL_PATH" --help >/dev/null 2>&1; then
                    log_success "Successfully installed and verified binary ($CANDIDATE) from latest."
                    INSTALLED=1
                    break
                else
                    rm -f "$BIN_INSTALL_PATH"
                fi
            fi
        done
    fi

    if [ $INSTALLED -eq 0 ] && [ -x "$BIN_INSTALL_PATH" ]; then
        log_info "Retaining currently installed binary at $BIN_INSTALL_PATH."
        return 0
    fi

    if [ ! -x "$BIN_INSTALL_PATH" ]; then
        log_err "Error: Failed to download or verify a compatible Komodo Periphery binary for $ARCH ($LIBC)."
        log_err "You can specify a direct binary with --binary-url or --binary-path."
        exit 1
    fi
}

install_binary

# Stop any running instances prior to service update
stop_service

# Generate Service Definitions
log_info "Registering service for $INIT_SYS..."

case "$INIT_SYS" in
    systemd)
        cat > /etc/systemd/system/periphery.service <<EOF
[Unit]
Description=Komodo Periphery Agent
After=network.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$BIN_INSTALL_PATH --config-path $CONFIG_FILE
Restart=always
RestartSec=5
LimitNOFILE=65536
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
EOF
        ;;

    openrc)
        cat > /etc/init.d/periphery <<EOF
#!/sbin/openrc-run
name="Komodo Periphery"
description="Komodo Periphery Agent"

command="$BIN_INSTALL_PATH"
command_args="--config-path $CONFIG_FILE"
command_background="yes"
pidfile="/run/periphery.pid"
output_log="/var/log/periphery.log"
error_log="/var/log/periphery.err"

depend() {
    need net
    after firewall
}
EOF
        ;;

    runit)
        mkdir -p /etc/sv/periphery
        cat > /etc/sv/periphery/run <<EOF
#!/bin/sh
exec 2>&1
exec $BIN_INSTALL_PATH --config-path $CONFIG_FILE
EOF
        ;;

    s6)
        mkdir -p /var/service/periphery
        cat > /var/service/periphery/run <<EOF
#!/bin/sh
exec 2>&1
exec $BIN_INSTALL_PATH --config-path $CONFIG_FILE
EOF
        chmod +x /var/service/periphery/run
        ;;

    dinit)
        mkdir -p /etc/dinit.d
        cat > /etc/dinit.d/periphery <<EOF
type = process
command = $BIN_INSTALL_PATH --config-path $CONFIG_FILE
restart = yes
smooth-recovery = yes
logfile = /var/log/periphery.log
EOF
        ;;

    sysvinit)
        cat > /etc/init.d/periphery <<EOF
#!/bin/sh
### BEGIN INIT INFO
# Provides:          periphery
# Required-Start:    \$network \$local_fs \$remote_fs
# Required-Stop:     \$network \$local_fs \$remote_fs
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: Komodo Periphery Agent
### END INIT INFO

DAEMON=$BIN_INSTALL_PATH
DAEMON_ARGS="--config-path $CONFIG_FILE"
PIDFILE=/run/periphery.pid

case "\$1" in
    start)
        echo "Starting Komodo Periphery..."
        start-stop-daemon --start --background --make-pidfile --pidfile "\$PIDFILE" --exec "\$DAEMON" -- \$DAEMON_ARGS
        ;;
    stop)
        echo "Stopping Komodo Periphery..."
        start-stop-daemon --stop --pidfile "\$PIDFILE" --retry 5
        rm -f "\$PIDFILE"
        ;;
    restart)
        \$0 stop
        sleep 1
        \$0 start
        ;;
    status)
        start-stop-daemon --status --pidfile "\$PIDFILE" && echo "Running" || echo "Stopped"
        ;;
    *)
        echo "Usage: \$0 {start|stop|restart|status}"
        exit 1
        ;;
esac
EOF
        ;;
esac

# Start Service
start_service

log_success "=========================================================="
log_success "  SUCCESS: Komodo Periphery setup finished!"
log_success "  Service Manager: $INIT_SYS"
log_success "  Node '$CONNECT_AS' configured to connect to '$CORE_ADDRESS'."
log_success "=========================================================="
