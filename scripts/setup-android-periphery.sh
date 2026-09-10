#!/system/bin/sh
# Komodo Android Periphery Installer & Lifecycle Manager
# Canonical one-command installer for rooted Android (Magisk / KernelSU / APatch)
# Repository: https://github.com/ABHIMANYU993/komodo

set -e

VERSION_DEFAULT="latest"
GITHUB_REPO="ABHIMANYU993/komodo"
TMP_DIR="/data/local/tmp/komodo_install_$$"
KOMODO_DIR="/data/adb/komodo"
CONFIG_FILE="$KOMODO_DIR/config.toml"
KEYS_DIR="$KOMODO_DIR/keys"
LOG_FILE="$KOMODO_DIR/daemon.log"
MODULE_ID="komodo-android-periphery"
ACTIVE_MODDIR="/data/adb/modules/$MODULE_ID"
UPDATE_MODDIR="/data/adb/modules_update/$MODULE_ID"

# Expand PATH early to locate Magisk, KernelSU, APatch, and system utilities
for p in /data/adb/magisk /data/adb/ksu/bin /data/adb/ap/bin /sbin /system/bin /system/xbin /debug_ramdisk /system/bin/.magisk; do
    if [ -d "$p" ]; then
        case ":$PATH:" in
            *:"$p":*) ;;
            *) PATH="$PATH:$p" ;;
        esac
    fi
done
export PATH

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log_info() {
    printf "${BLUE}%s${NC}\n" "$1"
}

log_success() {
    printf "${GREEN}%s${NC}\n" "$1"
}

log_warn() {
    printf "${YELLOW}%s${NC}\n" "$1"
}

log_err() {
    printf "${RED}%s${NC}\n" "$1" >&2
}

print_help() {
    cat <<EOF
Komodo Android Periphery Installer & Lifecycle Manager

Usage:
  setup-android-periphery.sh [ACTION] [OPTIONS]

Actions:
  --install                 Install or configure Android Periphery (default)
  --reinstall               Fresh reinstall: wipe keys and re-onboard
  --reconfig                Update configuration (Core address, node name) and restart
  --update, --upgrade       Download and install latest module version
  --restart                 Restart running daemon process
  --status                  Show current daemon running status and config
  --uninstall               Uninstall Magisk module and stop daemon
  --purge                   Used with --uninstall to also delete all keys and configs

Required Options (for install / reconfig):
  --core-address=<url>      WebSocket URL of Komodo Core (e.g. ws://192.168.31.100:9120)
  --onboarding-key=<key>    One-time onboarding token (required for initial registration)

Optional Parameters:
  --connect-as=<name>       Server identifier name (defaults to device hostname)
  --version=<version>       Specify release version (e.g. v2.4.0, defaults to latest)
  --artifact-url=<url>      Direct URL to prebuilt komodo-android-periphery.zip
  --artifact-file=<path>    Local path to prebuilt komodo-android-periphery.zip
  --polling-rate=<rate>     Telemetry polling interval (e.g. 1-sec, 2-sec, default: 1-sec)
  --non-interactive         Disable interactive prompts
  --verbose                 Enable detailed logging
  --force                   Bypass confirmation prompts and force execution
  -h, --help                Show this help message

Examples:
  # Initial Install / Connection via curl:
  curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \\
    --core-address="ws://192.168.31.100:9120" \\
    --connect-as="Redmi_Note_7_Pro" \\
    --onboarding-key="YOUR_ONBOARDING_KEY"

  # Initial Install / Connection via wget:
  wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \\
    --core-address="ws://192.168.31.100:9120" \\
    --connect-as="Realme_7_Pro" \\
    --onboarding-key="YOUR_ONBOARDING_KEY"

  # Fresh Reinstall (reset keys):
  sh setup-android-periphery.sh --reinstall \\
    --core-address="ws://192.168.31.100:9120" \\
    --connect-as="Redmi_Note_7_Pro" \\
    --onboarding-key="YOUR_NEW_KEY"

  # Check Status:
  sh setup-android-periphery.sh --status

  # Clean Uninstall:
  sh setup-android-periphery.sh --uninstall --purge
EOF
}

# Parse Arguments
ACTION="install"
CORE_ADDRESS=""
CONNECT_AS=""
ONBOARDING_KEY=""
VERSION="$VERSION_DEFAULT"
ARTIFACT_URL=""
ARTIFACT_FILE=""
NON_INTERACTIVE=0
VERBOSE=0
FORCE=0
PURGE=0
POLLING_RATE="1-sec"

while [ $# -gt 0 ]; do
    case "$1" in
        --install)
            ACTION="install"
            ;;
        --reinstall)
            ACTION="reinstall"
            ;;
        --reconfig|--update-config)
            ACTION="reconfig"
            ;;
        --update|--upgrade)
            ACTION="update"
            ;;
        --restart)
            ACTION="restart"
            ;;
        --status)
            ACTION="status"
            ;;
        --uninstall)
            ACTION="uninstall"
            ;;
        --purge)
            PURGE=1
            ;;
        --core-address=*|--core=*)
            CORE_ADDRESS="${1#*=}"
            ;;
        --core-address|--core)
            CORE_ADDRESS="$2"
            shift
            ;;
        --connect-as=*|--name=*)
            CONNECT_AS="${1#*=}"
            ;;
        --connect-as|--name)
            CONNECT_AS="$2"
            shift
            ;;
        --onboarding-key=*|--token=*)
            ONBOARDING_KEY="${1#*=}"
            ;;
        --onboarding-key|--token)
            ONBOARDING_KEY="$2"
            shift
            ;;
        --polling-rate=*|--stats-polling-rate=*)
            POLLING_RATE="${1#*=}"
            ;;
        --polling-rate|--stats-polling-rate)
            POLLING_RATE="$2"
            shift
            ;;
        --version=*)
            VERSION="${1#*=}"
            ;;
        --version)
            VERSION="$2"
            shift
            ;;
        --artifact-url=*)
            ARTIFACT_URL="${1#*=}"
            ;;
        --artifact-url)
            ARTIFACT_URL="$2"
            shift
            ;;
        --artifact-file=*)
            ARTIFACT_FILE="${1#*=}"
            ;;
        --artifact-file)
            ARTIFACT_FILE="$2"
            shift
            ;;
        --non-interactive)
            NON_INTERACTIVE=1
            ;;
        --verbose)
            VERBOSE=1
            ;;
        --force)
            FORCE=1
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            log_err "Unknown option: $1"
            print_help
            exit 1
            ;;
    esac
    shift
done

# Root Escalation Check
CURRENT_UID=$(id -u 2>/dev/null || echo 1)
if [ "$CURRENT_UID" -ne 0 ]; then
    log_warn "Current process is not root (UID $CURRENT_UID). Attempting escalation via su..."
    if command -v su >/dev/null 2>&1; then
        TMP_SCRIPT="/data/local/tmp/komodo_installer_run.sh"
        if [ -f "$0" ] && [ "$0" != "sh" ] && [ "$0" != "/system/bin/sh" ] && [ "$0" != "bash" ]; then
            SU_EXEC="$0"
        else
            cat > "$TMP_SCRIPT"
            chmod 700 "$TMP_SCRIPT"
            SU_EXEC="$TMP_SCRIPT"
        fi
        exec su -c "$SU_EXEC" "$@"
    else
        log_err "Error: Root access is required. 'su' command not found."
        exit 1
    fi
fi

# Cleanup on exit
cleanup() {
    rm -rf "$TMP_DIR" 2>/dev/null || true
    rm -f "/data/local/tmp/komodo_installer_run.sh" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

mkdir -p "$TMP_DIR"

# Stage 1: Detecting Android Environment
log_info "[1/8] Detecting Android environment..."
SDK_VER=$(getprop ro.build.version.sdk 2>/dev/null || echo "0")
OS_REL=$(getprop ro.build.version.release 2>/dev/null || echo "unknown")
CPU_ABI=$(getprop ro.product.cpu.abi 2>/dev/null || uname -m 2>/dev/null || echo "unknown")

if [ "$SDK_VER" -eq 0 ] && [ ! -f /system/bin/linker64 ] && [ ! -d /system ]; then
    log_err "Error: This installer is intended strictly for Android systems."
    exit 1
fi

case "$CPU_ABI" in
    arm64*|aarch64*)
        [ $VERBOSE -eq 1 ] && echo "Detected compatible 64-bit ARM architecture ($CPU_ABI)"
        ;;
    *)
        log_err "Error: Unsupported CPU architecture: $CPU_ABI. Android Periphery requires ARM64 (aarch64-linux-android)."
        exit 1
        ;;
esac
[ $VERBOSE -eq 1 ] && echo "Android Release: $OS_REL (API $SDK_VER)"

# Stage 2: Checking Root and Magisk/KernelSU/APatch
log_info "[2/8] Checking root and module manager availability..."
MAGISK_BIN=""
if command -v magisk >/dev/null 2>&1; then
    MAGISK_BIN="magisk"
else
    for candidate in /data/adb/magisk/magisk /sbin/magisk /system/bin/magisk /system/xbin/magisk /debug_ramdisk/magisk /system/bin/.magisk; do
        if [ -x "$candidate" ]; then
            MAGISK_BIN="$candidate"
            break
        fi
    done
fi

ROOT_PROVIDER="Generic Root"
if [ -n "$MAGISK_BIN" ]; then
    MAGISK_VER=$("$MAGISK_BIN" -v 2>/dev/null || echo "detected")
    ROOT_PROVIDER="Magisk ($MAGISK_VER)"
elif command -v ksud >/dev/null 2>&1; then
    ROOT_PROVIDER="KernelSU ($(ksud -V 2>/dev/null || echo "detected"))"
elif command -v apd >/dev/null 2>&1; then
    ROOT_PROVIDER="APatch ($(apd -V 2>/dev/null || echo "detected"))"
elif [ -d "/data/adb/modules" ]; then
    ROOT_PROVIDER="Magisk/KernelSU compatible (/data/adb/modules)"
else
    # Create /data/adb/modules directory if root is active
    mkdir -p /data/adb/modules
    ROOT_PROVIDER="Android Root Directory (/data/adb)"
fi
log_success "Root provider verified: $ROOT_PROVIDER"

# Helper: stop all running daemon instances
stop_daemon() {
    log_info "Gracefully stopping running periphery daemon instances..."
    if [ -f "$KOMODO_DIR/daemon.pid" ]; then
        SUPERVISOR_PID=$(cat "$KOMODO_DIR/daemon.pid" 2>/dev/null || true)
        if [ -n "$SUPERVISOR_PID" ] && [ "$SUPERVISOR_PID" != "$$" ]; then
            kill -TERM "$SUPERVISOR_PID" 2>/dev/null || true
            kill -9 "$SUPERVISOR_PID" 2>/dev/null || true
        fi
        rm -f "$KOMODO_DIR/daemon.pid" 2>/dev/null || true
    fi

    for pid in $(pgrep -x "komodo-android-periphery" 2>/dev/null || true) \
               $(pgrep -x "komodo-android-" 2>/dev/null || true) \
               $(pgrep -f "komodo-android-periphery" 2>/dev/null || true); do
        if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
            kill -TERM "$pid" 2>/dev/null || true
        fi
    done

    # Wait up to 4s for daemon to terminate and close WebSocket connection
    for _i in 1 2 3 4; do
        if ! pgrep -f "komodo-android-periphery" >/dev/null 2>&1; then
            break
        fi
        sleep 1
    done

    # Force kill if still running
    if pgrep -f "komodo-android-periphery" >/dev/null 2>&1; then
        for pid in $(pgrep -f "komodo-android-periphery" 2>/dev/null || true); do
            if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        done
    fi
    sleep 1
}

# ACTION: Status
if [ "$ACTION" = "status" ]; then
    log_info "=== Komodo Android Periphery Status ==="
    echo "Module directory: $ACTIVE_MODDIR"
    if [ -f "$ACTIVE_MODDIR/module.prop" ]; then
        cat "$ACTIVE_MODDIR/module.prop"
    else
        echo "Module: NOT INSTALLED"
    fi
    echo ""
    echo "Config file: $CONFIG_FILE"
    if [ -f "$CONFIG_FILE" ]; then
        cat "$CONFIG_FILE"
    else
        echo "Config: NOT FOUND"
    fi
    echo ""
    RUNNING_PID=$(pgrep -f "komodo-android-periphery" 2>/dev/null || true)
    if [ -n "$RUNNING_PID" ]; then
        log_success "Daemon: RUNNING (PID: $RUNNING_PID)"
    else
        log_warn "Daemon: STOPPED"
    fi
    if [ -f "$KEYS_DIR/periphery.pub" ]; then
        echo "Public Key: $(cat "$KEYS_DIR/periphery.pub" 2>/dev/null)"
    fi
    exit 0
fi

# ACTION: Uninstall
if [ "$ACTION" = "uninstall" ]; then
    log_info "Uninstalling Komodo Android Periphery..."
    stop_daemon
    rm -rf "$ACTIVE_MODDIR" "$UPDATE_MODDIR" 2>/dev/null || true
    if [ "$PURGE" -eq 1 ]; then
        rm -rf "$KOMODO_DIR" 2>/dev/null || true
        log_success "Uninstalled successfully. All configuration and keys purged."
    else
        log_success "Module removed. Configuration and keys preserved in $KOMODO_DIR (use --purge to delete)."
    fi
    exit 0
fi

# ACTION: Restart
if [ "$ACTION" = "restart" ]; then
    stop_daemon
    if [ -x "$ACTIVE_MODDIR/komodo-android-periphery" ]; then
        TARGET_BIN="$ACTIVE_MODDIR/komodo-android-periphery"
    elif [ -x "$UPDATE_MODDIR/komodo-android-periphery" ]; then
        TARGET_BIN="$UPDATE_MODDIR/komodo-android-periphery"
    else
        log_err "Daemon binary not found."
        exit 1
    fi
    (
        echo "$$" > "$KOMODO_DIR/daemon.pid"
        while true; do
            if [ -x "$TARGET_BIN" ] && [ -f "$CONFIG_FILE" ]; then
                "$TARGET_BIN" --config "$CONFIG_FILE" >> "$LOG_FILE" 2>&1
            fi
            sleep 5
        done
    ) &
    log_success "Periphery daemon restarted."
    exit 0
fi

# Inspect Existing Installation
EXISTING_INSTALLED=0
EXISTING_CORE=""
EXISTING_NAME=""
if [ -d "$ACTIVE_MODDIR" ] || [ -f "$CONFIG_FILE" ]; then
    EXISTING_INSTALLED=1
    if [ -f "$CONFIG_FILE" ]; then
        EXISTING_CORE=$(grep '^core_url' "$CONFIG_FILE" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || true)
        EXISTING_NAME=$(grep '^connect_as' "$CONFIG_FILE" 2>/dev/null | cut -d'=' -f2 | tr -d ' "' || true)
    fi
fi

# Interactive Menu for existing installations (when no explicit action/args passed in terminal)
if [ $EXISTING_INSTALLED -eq 1 ] && [ $NON_INTERACTIVE -eq 0 ] && [ -t 0 ] && [ "$ACTION" = "install" ] && [ -z "$CORE_ADDRESS" ] && [ $FORCE -eq 0 ]; then
    INSTALLED_VER=$(grep '^version=' "$ACTIVE_MODDIR/module.prop" 2>/dev/null | cut -d'=' -f2 || echo "unknown")
    printf "\n${BOLD}==========================================================\n"
    printf "  Existing Komodo Android Periphery Detected\n"
    printf "  Installed Version: v%s | Node: '%s'\n" "$INSTALLED_VER" "$EXISTING_NAME"
    printf "  Current Core URL:  %s\n" "$EXISTING_CORE"
    printf "==========================================================${NC}\n"
    printf "Select an operation:\n"
    printf "  1) Reconfigure Core address & re-register with new Core\n"
    printf "  2) Fresh Reinstall (reset keys and re-onboard)\n"
    printf "  3) Upgrade / Update to latest version\n"
    printf "  4) Restart Periphery daemon\n"
    printf "  5) Uninstall Komodo Android Periphery\n"
    printf "  6) Exit\n"
    printf "Choice [1-6]: "
    read -r CHOICE || CHOICE="6"
    case "$CHOICE" in
        1)
            ACTION="reconfig"
            printf "Enter new Komodo Core address (e.g. ws://192.168.31.100:9120): "
            read -r CORE_ADDRESS
            printf "Enter Onboarding Key (leave empty to reuse existing keys): "
            read -r ONBOARDING_KEY
            printf "Enter Server name (default '%s'): " "$EXISTING_NAME"
            read -r CONNECT_AS
            ;;
        2)
            ACTION="reinstall"
            printf "Enter Komodo Core address (e.g. ws://192.168.31.100:9120): "
            read -r CORE_ADDRESS
            printf "Enter Onboarding Key: "
            read -r ONBOARDING_KEY
            ;;
        3)
            ACTION="update"
            ;;
        4)
            ACTION="restart"
            ;;
        5)
            ACTION="uninstall"
            ;;
        *)
            echo "Operation cancelled."
            exit 0
            ;;
    esac
fi

# Non-interactive intelligent detection:
# If user provided a new core-address or onboarding key, automatically reconfigure or reinstall!
if [ $EXISTING_INSTALLED -eq 1 ] && [ "$ACTION" = "install" ] && [ $FORCE -eq 0 ]; then
    CONFIG_CHANGED=0
    if [ -n "$CORE_ADDRESS" ] && [ "$CORE_ADDRESS" != "$EXISTING_CORE" ]; then
        CONFIG_CHANGED=1
    fi
    if [ -n "$CONNECT_AS" ] && [ "$CONNECT_AS" != "$EXISTING_NAME" ]; then
        CONFIG_CHANGED=1
    fi
    if [ -n "$ONBOARDING_KEY" ]; then
        CONFIG_CHANGED=1
    fi

    if [ $CONFIG_CHANGED -eq 1 ]; then
        log_info "Configuration change or new onboarding key detected. Reconfiguring node..."
        ACTION="reconfig"
    else
        INSTALLED_VER=$(grep '^version=' "$ACTIVE_MODDIR/module.prop" 2>/dev/null | cut -d'=' -f2 || echo "")
        if pgrep -f "komodo-android-periphery" >/dev/null 2>&1 && [ -f "$KEYS_DIR/periphery.key" ]; then
            log_success "=========================================================="
            log_success "  ALREADY RUNNING & HEALTHY"
            log_success "  Node '$EXISTING_NAME' is active on Core '$EXISTING_CORE'."
            log_success "  Version: $INSTALLED_VER"
            log_success "  (To point to a different Core, provide --core-address or use --reinstall)"
            log_success "=========================================================="
            exit 0
        fi
    fi
fi

# Resolve defaults
if [ -z "$CONNECT_AS" ]; then
    if [ -n "$EXISTING_NAME" ]; then
        CONNECT_AS="$EXISTING_NAME"
    else
        CONNECT_AS="$(hostname 2>/dev/null || getprop net.hostname 2>/dev/null || echo "android-node")"
    fi
fi

if [ -z "$CORE_ADDRESS" ]; then
    if [ -n "$EXISTING_CORE" ]; then
        CORE_ADDRESS="$EXISTING_CORE"
    else
        log_err "Error: Missing required parameter --core-address."
        print_help
        exit 1
    fi
fi

# Format core address to ws:// or wss:// if provided as http
case "$CORE_ADDRESS" in
    http://*)
        CORE_ADDRESS="ws://${CORE_ADDRESS#http://}"
        ;;
    https://*)
        CORE_ADDRESS="wss://${CORE_ADDRESS#https://}"
        ;;
    ws://*|wss://*)
        ;;
    *)
        CORE_ADDRESS="ws://${CORE_ADDRESS}"
        ;;
esac

# Gracefully stop running periphery daemon before modifying keys, config, or module files
stop_daemon

# Fresh reinstall resets keys
if [ "$ACTION" = "reinstall" ]; then
    log_warn "Fresh reinstall requested. Purging previous authentication keys..."
    rm -rf "$KEYS_DIR"/* 2>/dev/null || true
    if [ -z "$ONBOARDING_KEY" ]; then
        log_err "Error: --onboarding-key is required for a fresh reinstall."
        exit 1
    fi
fi

# If onboarding key provided during reconfig, clear old keys to force zero-trust onboarding
if [ -n "$ONBOARDING_KEY" ]; then
    rm -rf "$KEYS_DIR"/* 2>/dev/null || true
fi

# Normalize polling rate
case "$POLLING_RATE" in
    [0-9]*)
        if ! echo "$POLLING_RATE" | grep -q -- "-sec"; then
            POLLING_RATE="${POLLING_RATE}-sec"
        fi
        ;;
esac

# Stage: Write Config
write_config_file() {
    mkdir -p "$KOMODO_DIR" "$KEYS_DIR" "$KOMODO_DIR/logs" "$KOMODO_DIR/backups"
    chmod 700 "$KOMODO_DIR" "$KEYS_DIR" "$KOMODO_DIR/logs" "$KOMODO_DIR/backups"

    cat > "$CONFIG_FILE" <<EOF
core_url = "$CORE_ADDRESS"
connect_as = "$CONNECT_AS"
log_level = "info"
keys_dir = "$KEYS_DIR"
stats_polling_rate = "$POLLING_RATE"
EOF
    if [ -n "$ONBOARDING_KEY" ]; then
        echo "onboarding_key = \"$ONBOARDING_KEY\"" >> "$CONFIG_FILE"
    fi
    chmod 600 "$CONFIG_FILE"
}

# If only reconfiguring, update config and restart daemon immediately
if [ "$ACTION" = "reconfig" ] && [ -d "$ACTIVE_MODDIR" ]; then
    log_info "Applying new configuration: Core = $CORE_ADDRESS, Node = $CONNECT_AS..."
    write_config_file
    stop_daemon

    TARGET_BIN="$ACTIVE_MODDIR/komodo-android-periphery"
    (
        while true; do
            if [ -x "$TARGET_BIN" ] && [ -f "$CONFIG_FILE" ]; then
                echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Starting komodo-android-periphery..." >> "$LOG_FILE"
                "$TARGET_BIN" --config "$CONFIG_FILE" >> "$LOG_FILE" 2>&1
                EXIT_CODE=$?
                echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Agent exited ($EXIT_CODE). Restarting in 5s..." >> "$LOG_FILE"
            fi
            sleep 5
        done
    ) &

    log_info "Verifying connection to Komodo Core..."
    CONNECTED=0
    for i in $(seq 1 12); do
        sleep 1
        if grep -E "Authenticated successfully as|Entering message loop|Onboarding for.*completed successfully" "$LOG_FILE" 2>/dev/null | tail -n 5 >/dev/null 2>&1; then
            CONNECTED=1
            break
        fi
    done

    if [ $CONNECTED -eq 1 ]; then
        if [ -n "$ONBOARDING_KEY" ]; then
            grep -v '^onboarding_key' "$CONFIG_FILE" > "${CONFIG_FILE}.tmp" 2>/dev/null && mv "${CONFIG_FILE}.tmp" "$CONFIG_FILE"
            chmod 600 "$CONFIG_FILE"
        fi
        log_success "=========================================================="
        log_success "  SUCCESS: Reconfigured and connected!"
        log_success "  Node '$CONNECT_AS' is now communicating with '$CORE_ADDRESS'."
        log_success "=========================================================="
    else
        log_warn "Daemon restarted with new config. Awaiting WebSocket handshake..."
        log_info "Check log: $LOG_FILE"
    fi
    exit 0
fi

# Stage 3: Downloading Release Artifact
log_info "[3/8] Downloading release artifact..."
ZIP_DEST="$TMP_DIR/komodo-android-periphery.zip"
SHA_DEST="$TMP_DIR/komodo-android-periphery.zip.sha256"

if [ -n "$ARTIFACT_FILE" ]; then
    if [ ! -f "$ARTIFACT_FILE" ]; then
        log_err "Error: Specified artifact file does not exist: $ARTIFACT_FILE"
        exit 1
    fi
    log_info "Using local artifact file: $ARTIFACT_FILE"
    cp "$ARTIFACT_FILE" "$ZIP_DEST"
    [ -f "${ARTIFACT_FILE}.sha256" ] && cp "${ARTIFACT_FILE}.sha256" "$SHA_DEST"
else
    if [ -n "$ARTIFACT_URL" ]; then
        ZIP_URL="$ARTIFACT_URL"
        SHA_URL="${ARTIFACT_URL}.sha256"
    elif [ "$VERSION" = "latest" ]; then
        ZIP_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/komodo-android-periphery.zip"
        SHA_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/komodo-android-periphery.zip.sha256"
    else
        ZIP_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/komodo-android-periphery.zip"
        SHA_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/komodo-android-periphery.zip.sha256"
    fi

    log_info "Fetching artifact from: $ZIP_URL"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$ZIP_URL" -o "$ZIP_DEST"
        curl -fsSL "$SHA_URL" -o "$SHA_DEST" 2>/dev/null || true
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$ZIP_URL" -O "$ZIP_DEST"
        wget -q "$SHA_URL" -O "$SHA_DEST" 2>/dev/null || true
    elif [ -x /data/adb/magisk/busybox ]; then
        /data/adb/magisk/busybox wget -q "$ZIP_URL" -O "$ZIP_DEST"
        /data/adb/magisk/busybox wget -q "$SHA_URL" -O "$SHA_DEST" 2>/dev/null || true
    else
        log_err "Error: Neither curl nor wget is available on this device."
        exit 1
    fi
fi

# Stage 4: Verifying Artifact Integrity
log_info "[4/8] Verifying artifact integrity..."
if [ ! -s "$ZIP_DEST" ]; then
    log_err "Error: Downloaded package is empty or failed to download."
    exit 1
fi

if [ -s "$SHA_DEST" ]; then
    EXPECTED_SHA=$(awk '{print $1}' "$SHA_DEST" | head -n 1)
    ACTUAL_SHA=$(sha256sum "$ZIP_DEST" 2>/dev/null | awk '{print $1}' || echo "")
    if [ -n "$ACTUAL_SHA" ] && [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
        log_err "Error: SHA-256 mismatch! Expected: $EXPECTED_SHA, Got: $ACTUAL_SHA"
        exit 1
    fi
    [ $VERBOSE -eq 1 ] && echo "SHA-256 verified: $ACTUAL_SHA"
fi

# Stage 5: Installing Magisk/KernelSU/APatch Module
log_info "[5/8] Installing module package..."
stop_daemon

INSTALLED_OK=0
if [ -n "$MAGISK_BIN" ]; then
    if "$MAGISK_BIN" --install-module "$ZIP_DEST" 2>/dev/null; then
        INSTALLED_OK=1
    fi
elif command -v ksud >/dev/null 2>&1; then
    if ksud module install "$ZIP_DEST" 2>/dev/null; then
        INSTALLED_OK=1
    fi
elif command -v apd >/dev/null 2>&1; then
    if apd module install "$ZIP_DEST" 2>/dev/null; then
        INSTALLED_OK=1
    fi
fi

# Universal Fallback: Extract directly to /data/adb/modules/$MODULE_ID
if [ $INSTALLED_OK -eq 0 ]; then
    log_info "Using direct module extraction fallback to $ACTIVE_MODDIR..."
    mkdir -p "$ACTIVE_MODDIR"
    unzip -o "$ZIP_DEST" -d "$ACTIVE_MODDIR" >/dev/null 2>&1 || {
        log_err "Error: Failed to extract module package."
        exit 1
    }
    chmod 755 "$ACTIVE_MODDIR"/*.sh "$ACTIVE_MODDIR/komodo-android-periphery" 2>/dev/null || true
fi

# Stage 6: Configuring Periphery Service
log_info "[6/8] Configuring Periphery service..."
write_config_file

# Stage 7: Starting Periphery Agent
log_info "[7/8] Starting Periphery agent..."
if [ -x "$ACTIVE_MODDIR/komodo-android-periphery" ]; then
    TARGET_BIN="$ACTIVE_MODDIR/komodo-android-periphery"
elif [ -x "$UPDATE_MODDIR/komodo-android-periphery" ]; then
    TARGET_BIN="$UPDATE_MODDIR/komodo-android-periphery"
else
    log_err "Error: Could not locate installed komodo-android-periphery binary."
    exit 1
fi

(
    echo "$$" > "$KOMODO_DIR/daemon.pid"
    while true; do
        if [ -x "$TARGET_BIN" ] && [ -f "$CONFIG_FILE" ]; then
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Starting komodo-android-periphery..." >> "$LOG_FILE"
            "$TARGET_BIN" --config "$CONFIG_FILE" >> "$LOG_FILE" 2>&1
            EXIT_CODE=$?
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Agent exited ($EXIT_CODE). Restarting in 5s..." >> "$LOG_FILE"
        fi
        sleep 5
    done
) &

# Stage 8: Verifying Komodo Connection
log_info "[8/8] Verifying Komodo connection..."
CONNECTED=0
COUNT=0
TIMEOUT=15

while [ $COUNT -lt $TIMEOUT ]; do
    sleep 1
    COUNT=$((COUNT + 1))
    if [ -f "$LOG_FILE" ]; then
        if grep -E "Authenticated successfully as|Entering message loop|Onboarding for.*completed successfully" "$LOG_FILE" >/dev/null 2>&1; then
            CONNECTED=1
            break
        fi
    fi
done

if [ $CONNECTED -eq 1 ]; then
    if [ -n "$ONBOARDING_KEY" ] && [ -f "$CONFIG_FILE" ]; then
        grep -v '^onboarding_key' "$CONFIG_FILE" > "${CONFIG_FILE}.tmp" 2>/dev/null && mv "${CONFIG_FILE}.tmp" "$CONFIG_FILE"
        chmod 600 "$CONFIG_FILE"
    fi
    log_success "=========================================================="
    log_success "  SUCCESS: Android Periphery installed and connected!"
    log_success "  Node '$CONNECT_AS' is now communicating with Komodo Core."
    log_success "  Verify the Server shows OK in the Komodo UI."
    log_success "=========================================================="
else
    log_warn "=========================================================="
    log_warn "  Daemon started. Awaiting connection handshake..."
    log_warn "  Check log file: $LOG_FILE"
    log_warn "=========================================================="
fi

exit 0
