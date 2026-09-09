#!/system/bin/sh
# Komodo Android Periphery Installer
# Canonical one-command installer for rooted Android (Magisk)
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

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

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
Komodo Android Periphery Installer

Usage:
  setup-android-periphery.sh [OPTIONS]

Required Options:
  --core-address=<url>     WebSocket URL of Komodo Core (e.g. ws://192.168.31.80:9120)
  --onboarding-key=<key>   One-time onboarding token (required for initial registration)

Optional Parameters:
  --connect-as=<name>      Server identifier name (defaults to device hostname)
  --version=<version>      Specify release version (e.g. v0.1.0, defaults to latest)
  --artifact-url=<url>     Direct URL to prebuilt komodo-android-periphery.zip
  --artifact-file=<path>   Local path to prebuilt komodo-android-periphery.zip
  --polling-rate=<rate>    Telemetry polling interval (e.g. 1-sec, 2-sec, default: 1-sec)
  --non-interactive        Disable interactive prompts (e.g. reboot prompt)
  --verbose                Enable detailed logging
  --force                  Reinstall even if already installed and healthy
  -h, --help               Show this help message

Example:
  curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \\
    --core-address="ws://192.168.31.80:9120" \\
    --connect-as="\$(hostname)" \\
    --onboarding-key="YOUR_ONBOARDING_KEY"
EOF
}

# Parse Arguments
CORE_ADDRESS=""
CONNECT_AS=""
ONBOARDING_KEY=""
VERSION="$VERSION_DEFAULT"
ARTIFACT_URL=""
ARTIFACT_FILE=""
NON_INTERACTIVE=0
VERBOSE=0
FORCE=0
POLLING_RATE="1-sec"

while [ $# -gt 0 ]; do
    case "$1" in
        --core-address=*)
            CORE_ADDRESS="${1#*=}"
            ;;
        --core-address)
            CORE_ADDRESS="$2"
            shift
            ;;
        --connect-as=*)
            CONNECT_AS="${1#*=}"
            ;;
        --connect-as)
            CONNECT_AS="$2"
            shift
            ;;
        --onboarding-key=*)
            ONBOARDING_KEY="${1#*=}"
            ;;
        --onboarding-key)
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

# Check Root Escalation (Task 6)
CURRENT_UID=$(id -u 2>/dev/null || echo 1)
if [ "$CURRENT_UID" -ne 0 ]; then
    log_warn "Current process is not root (UID $CURRENT_UID). Escalating via su..."
    # If script was piped through stdin, write it out to temp location for su execution
    TMP_SCRIPT="/data/local/tmp/komodo_installer_run.sh"
    if [ -f "$0" ] && [ "$0" != "sh" ] && [ "$0" != "/system/bin/sh" ]; then
        SU_EXEC="$0"
    else
        cat > "$TMP_SCRIPT"
        chmod 700 "$TMP_SCRIPT"
        SU_EXEC="$TMP_SCRIPT"
    fi

    # Pass all arguments to su
    exec su -c "$SU_EXEC" "$@"
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

if [ "$SDK_VER" -eq 0 ] && [ ! -f /system/bin/linker64 ]; then
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

# Stage 2: Checking Root and Magisk Availability
log_info "[2/8] Checking root and Magisk availability..."
if ! command -v magisk >/dev/null 2>&1; then
    log_err "Error: Magisk not found in PATH. A rooted Android system with Magisk is required."
    exit 1
fi
MAGISK_VER=$(magisk -v 2>/dev/null || echo "Unknown")
[ $VERBOSE -eq 1 ] && echo "Magisk Version: $MAGISK_VER"

if [ ! -d "/data/adb" ]; then
    log_err "Error: /data/adb directory not found. Magisk environment corrupted or inaccessible."
    exit 1
fi

# Resolve defaults
if [ -z "$CONNECT_AS" ]; then
    CONNECT_AS="$(hostname 2>/dev/null || getprop net.hostname 2>/dev/null || echo "android-node")"
fi

# Validation
if [ -z "$CORE_ADDRESS" ]; then
    log_err "Error: Missing required parameter --core-address."
    print_help
    exit 1
fi

# Check Idempotency (Task 11)
IS_UPGRADE=0
if [ -d "$ACTIVE_MODDIR" ] && [ $FORCE -eq 0 ]; then
    INSTALLED_VER=$(grep '^version=' "$ACTIVE_MODDIR/module.prop" 2>/dev/null | cut -d'=' -f2 || echo "")
    if pgrep -f "komodo-android-periphery" >/dev/null 2>&1; then
        if [ "$INSTALLED_VER" = "$VERSION" ] || [ "$VERSION" = "latest" ]; then
            # Verify if keys exist and daemon is healthy
            if [ -f "$KEYS_DIR/periphery.key" ]; then
                log_success "=========================================================="
                log_success "  ALREADY INSTALLED / HEALTHY"
                log_success "  Node '$CONNECT_AS' is running and registered."
                log_success "  Module Version: $INSTALLED_VER"
                log_success "=========================================================="
                exit 0
            fi
        fi
    fi
    IS_UPGRADE=1
    log_info "Existing installation detected (v$INSTALLED_VER). Proceeding with upgrade..."
fi

# On fresh install, onboarding key is strictly required
if [ ! -f "$KEYS_DIR/periphery.key" ] && [ -z "$ONBOARDING_KEY" ]; then
    log_err "Error: Missing required parameter --onboarding-key for initial device onboarding."
    exit 1
fi

# Stage 3: Downloading Release Artifact (Task 8 / User Correction 3)
log_info "[3/8] Downloading release artifact..."
ZIP_DEST="$TMP_DIR/komodo-android-periphery.zip"
SHA_DEST="$TMP_DIR/komodo-android-periphery.zip.sha256"

# Select downloader
if command -v curl >/dev/null 2>&1; then
    FETCH_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    FETCH_CMD="wget -qO-"
elif [ -x /data/adb/magisk/busybox ]; then
    FETCH_CMD="/data/adb/magisk/busybox wget -qO-"
else
    log_err "Error: Neither curl nor wget was found on this device."
    exit 1
fi

if [ -n "$ARTIFACT_FILE" ]; then
    if [ ! -f "$ARTIFACT_FILE" ]; then
        log_err "Error: Specified artifact file does not exist: $ARTIFACT_FILE"
        exit 1
    fi
    log_info "Using local artifact file: $ARTIFACT_FILE"
    cp "$ARTIFACT_FILE" "$ZIP_DEST"
    if [ -f "${ARTIFACT_FILE}.sha256" ]; then
        cp "${ARTIFACT_FILE}.sha256" "$SHA_DEST"
    fi
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
    else
        wget -q "$ZIP_URL" -O "$ZIP_DEST"
        wget -q "$SHA_URL" -O "$SHA_DEST" 2>/dev/null || true
    fi
fi

# Stage 4: Verifying Artifact Integrity (Task 8)
log_info "[4/8] Verifying artifact integrity..."
if [ ! -s "$ZIP_DEST" ]; then
    log_err "Error: Downloaded package is empty or failed to download."
    exit 1
fi

# Verify checksum if .sha256 was retrieved
if [ -s "$SHA_DEST" ]; then
    EXPECTED_SHA=$(awk '{print $1}' "$SHA_DEST" | head -n 1)
    ACTUAL_SHA=$(sha256sum "$ZIP_DEST" | awk '{print $1}')
    if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
        log_err "Error: SHA-256 mismatch!"
        log_err "Expected: $EXPECTED_SHA"
        log_err "Actual:   $ACTUAL_SHA"
        exit 1
    fi
    [ $VERBOSE -eq 1 ] && echo "SHA-256 verified: $ACTUAL_SHA"
fi

# Verify ZIP archive structure
if ! unzip -t "$ZIP_DEST" >/dev/null 2>&1; then
    log_err "Error: Invalid or corrupted ZIP archive."
    exit 1
fi

# Stage 5: Installing Magisk Module (User Correction 1)
log_info "[5/8] Installing Magisk module via official CLI..."

# If upgrading, backup state before touching installation
if [ $IS_UPGRADE -eq 1 ]; then
    BACKUP_DIR="$KOMODO_DIR/backups/backup_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$BACKUP_DIR"
    cp -a "$CONFIG_FILE" "$BACKUP_DIR/" 2>/dev/null || true
    cp -a "$KEYS_DIR" "$BACKUP_DIR/" 2>/dev/null || true
    cp -a "$ACTIVE_MODDIR/module.prop" "$BACKUP_DIR/" 2>/dev/null || true
    log_info "Created rollback backup at $BACKUP_DIR"
fi

# Stop running daemon before updating module (avoid self-kill)
for pid in $(pgrep -x "komodo-android-periphery" 2>/dev/null || true) $(pgrep -x "komodo-android-" 2>/dev/null || true) $(pgrep -f "/data/adb/modules.*/komodo-android-periphery" 2>/dev/null || true); do
    if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
        kill -TERM "$pid" 2>/dev/null || true
    fi
done
sleep 1

# Supported Magisk installation command
magisk --install-module "$ZIP_DEST"

# Ensure persistent directory structure exists
mkdir -p "$KOMODO_DIR" "$KEYS_DIR" "$KOMODO_DIR/logs" "$KOMODO_DIR/backups"
chmod 700 "$KOMODO_DIR" "$KEYS_DIR" "$KOMODO_DIR/logs" "$KOMODO_DIR/backups"

# Install helper utility for komodo-control
mkdir -p "$KOMODO_DIR/bin"
if [ -f "$ACTIVE_MODDIR/komodo-control" ]; then
    cp "$ACTIVE_MODDIR/komodo-control" "$KOMODO_DIR/bin/komodo-control"
    chmod 755 "$KOMODO_DIR/bin/komodo-control"
elif [ -f "$UPDATE_MODDIR/komodo-control" ]; then
    cp "$UPDATE_MODDIR/komodo-control" "$KOMODO_DIR/bin/komodo-control"
    chmod 755 "$KOMODO_DIR/bin/komodo-control"
fi

# Stage 6: Configuring Periphery Service (Task 9, 10)
log_info "[6/8] Configuring Periphery service..."

# Normalize polling rate to ensure format like "1-sec"
case "$POLLING_RATE" in
    [0-9]*)
        if ! echo "$POLLING_RATE" | grep -q -- "-sec"; then
            POLLING_RATE="${POLLING_RATE}-sec"
        fi
        ;;
esac

# Write secure config.toml (mode 0600)
cat > "$CONFIG_FILE" <<EOF
core_url = "$CORE_ADDRESS"
connect_as = "$CONNECT_AS"
log_level = "info"
keys_dir = "$KEYS_DIR"
stats_polling_rate = "$POLLING_RATE"
EOF

# Include onboarding key only if provided (initial onboarding)
if [ -n "$ONBOARDING_KEY" ]; then
    echo "onboarding_key = \"$ONBOARDING_KEY\"" >> "$CONFIG_FILE"
fi
chmod 600 "$CONFIG_FILE"

# Stage 7: Starting Periphery Agent (Task 14, 15)
log_info "[7/8] Starting Periphery agent..."

# Find daemon binary: check active module or staged update
if [ -x "$ACTIVE_MODDIR/komodo-android-periphery" ]; then
    TARGET_BIN="$ACTIVE_MODDIR/komodo-android-periphery"
elif [ -x "$UPDATE_MODDIR/komodo-android-periphery" ]; then
    TARGET_BIN="$UPDATE_MODDIR/komodo-android-periphery"
else
    log_err "Error: Could not locate installed komodo-android-periphery binary."
    exit 1
fi

# Start daemon supervisor in background
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

# Stage 8: Verifying Komodo Connection (User Correction 4)
log_info "[8/8] Verifying Komodo connection..."
CONNECTED=0
COUNT=0
TIMEOUT=15

while [ $COUNT -lt $TIMEOUT ]; do
    sleep 1
    COUNT=$((COUNT + 1))
    
    # Check if process is running
    if ! pgrep -f "$TARGET_BIN" >/dev/null 2>&1; then
        continue
    fi

    # Check daemon log for successful authentication / entering message loop
    if [ -f "$LOG_FILE" ]; then
        if grep -E "Authenticated successfully as|Entering message loop|Onboarding for.*completed successfully" "$LOG_FILE" >/dev/null 2>&1; then
            CONNECTED=1
            break
        fi
    fi
done

if [ $CONNECTED -eq 1 ]; then
    # Onboarding secret sanitation (Task 10 / User Correction 6)
    if [ -n "$ONBOARDING_KEY" ] && [ -f "$CONFIG_FILE" ]; then
        # Remove onboarding_key line safely
        grep -v '^onboarding_key' "$CONFIG_FILE" > "${CONFIG_FILE}.tmp"
        mv "${CONFIG_FILE}.tmp" "$CONFIG_FILE"
        chmod 600 "$CONFIG_FILE"
        [ $VERBOSE -eq 1 ] && echo "Purged bootstrap onboarding secret from configuration."
    fi

    log_success "=========================================================="
    log_success "  SUCCESS: Android Periphery installed and connected!"
    log_success "  Node '$CONNECT_AS' is now communicating with Komodo Core."
    log_success "  Verify the Server shows OK in the Komodo UI."
    log_success "=========================================================="
else
    log_err "=========================================================="
    log_err "  WARNING: Daemon started but connection not confirmed yet."
    log_err "  Check log file: $LOG_FILE"
    log_err "=========================================================="
    if [ $IS_UPGRADE -eq 1 ] && [ -d "$BACKUP_DIR" ]; then
        log_warn "Upgrade did not verify cleanly within timeout. Rollback available at $BACKUP_DIR."
    fi
fi

# Reboot Prompt (User Correction 2)
if [ $NON_INTERACTIVE -eq 0 ] && [ -t 0 ]; then
    printf "\n${RED}${BOLD}============================================================\n"
    printf "WARNING\n"
    printf "The Android Periphery service will start automatically after\n"
    printf "reboot.\n"
    printf "Reboot is recommended to validate boot persistence.\n"
    printf "Current SSH session will be disconnected by reboot.\n"
    printf "Reboot now? [y/N]${NC}\n"
    printf "${RED}${BOLD}============================================================${NC}\n"
    printf "Selection (default N): "
    read -r ANSWER || ANSWER="N"
    case "$ANSWER" in
        y*|Y*)
            echo "Rebooting now. Reconnect over SSH after the device returns."
            /system/bin/reboot 2>/dev/null || reboot
            ;;
        *)
            echo "Module installed successfully. Reboot at your convenience to validate boot persistence."
            ;;
    esac
else
    echo "Module installed successfully. Reboot recommended for normal Magisk boot activation."
fi

exit 0
