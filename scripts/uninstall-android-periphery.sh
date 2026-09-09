#!/system/bin/sh
# Komodo Android Periphery Uninstaller
# Canonical uninstaller for rooted Android (Magisk)
# Repository: https://github.com/ABHIMANYU993/komodo

set -e

MODULE_ID="komodo-android-periphery"
ACTIVE_MODDIR="/data/adb/modules/$MODULE_ID"
UPDATE_MODDIR="/data/adb/modules_update/$MODULE_ID"
LEGACY_SERVICE="/data/adb/service.d/komodo-android-periphery.sh"
KOMODO_DIR="/data/adb/komodo"

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

# Check Root Escalation
CURRENT_UID=$(id -u 2>/dev/null || echo 1)
if [ "$CURRENT_UID" -ne 0 ]; then
    printf "${YELLOW}Process is not root (UID %s). Escalating via su...${NC}\n" "$CURRENT_UID"
    TMP_SCRIPT="/data/local/tmp/komodo_uninstaller_run.sh"
    if [ -f "$0" ] && [ "$0" != "sh" ] && [ "$0" != "/system/bin/sh" ]; then
        SU_EXEC="$0"
    else
        cat > "$TMP_SCRIPT"
        chmod 700 "$TMP_SCRIPT"
        SU_EXEC="$TMP_SCRIPT"
    fi
    exec su -c "$SU_EXEC" "$@"
fi

# Parse Options
NON_INTERACTIVE=0
while [ $# -gt 0 ]; do
    case "$1" in
        -y|--yes|--non-interactive)
            NON_INTERACTIVE=1
            ;;
        -h|--help)
            cat <<EOF
Usage: uninstall-android-periphery.sh [OPTIONS]

Options:
  -y, --yes, --non-interactive   Skip interactive confirmation
  -h, --help                     Show this help message
EOF
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
    shift
done

# Destructive Confirmation Prompt
if [ $NON_INTERACTIVE -eq 0 ] && [ -t 0 ]; then
    printf "\n${RED}${BOLD}============================================================\n"
    printf "DANGER\n"
    printf "This will completely remove Komodo Android Periphery,\n"
    printf "including its permanent identity keys and local configuration.\n"
    printf "The Komodo Server record in Core will NOT automatically be\n"
    printf "deleted unless explicitly requested.\n"
    printf "Continue? [y/N]${NC}\n"
    printf "${RED}${BOLD}============================================================${NC}\n"
    printf "Selection (default N): "
    read -r CONFIRM || CONFIRM="N"
    case "$CONFIRM" in
        y*|Y*)
            ;;
        *)
            echo "Uninstallation cancelled by user."
            exit 0
            ;;
    esac
fi

echo "1. Stopping Komodo Android Periphery daemon..."
for pid in $(pgrep -x "komodo-android-periphery" 2>/dev/null || true) $(pgrep -x "komodo-android-" 2>/dev/null || true) $(pgrep -f "/data/adb/modules.*/komodo-android-periphery" 2>/dev/null || true); do
    if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
        kill -TERM "$pid" 2>/dev/null || true
    fi
done
COUNT=0
while [ $COUNT -lt 5 ]; do
    STILL_RUNNING=0
    for pid in $(pgrep -x "komodo-android-periphery" 2>/dev/null || true) $(pgrep -x "komodo-android-" 2>/dev/null || true) $(pgrep -f "/data/adb/modules.*/komodo-android-periphery" 2>/dev/null || true); do
        if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
            STILL_RUNNING=1
            break
        fi
    done
    [ $STILL_RUNNING -eq 0 ] && break
    sleep 1
    COUNT=$((COUNT + 1))
done

for pid in $(pgrep -x "komodo-android-periphery" 2>/dev/null || true) $(pgrep -x "komodo-android-" 2>/dev/null || true) $(pgrep -f "/data/adb/modules.*/komodo-android-periphery" 2>/dev/null || true); do
    if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
        kill -KILL "$pid" 2>/dev/null || true
    fi
done

echo "2. Removing Magisk module files..."
# If active module exists, create remove flag for Magisk or remove directory
if [ -d "$ACTIVE_MODDIR" ]; then
    touch "$ACTIVE_MODDIR/remove" 2>/dev/null || true
    rm -rf "$ACTIVE_MODDIR"
fi

# Remove pending update directory if present
if [ -d "$UPDATE_MODDIR" ]; then
    rm -rf "$UPDATE_MODDIR"
fi

# Remove legacy service.d script if present
if [ -f "$LEGACY_SERVICE" ]; then
    rm -f "$LEGACY_SERVICE"
fi

echo "3. Removing persistent configuration, keys, and logs..."
if [ -d "$KOMODO_DIR" ]; then
    rm -rf "$KOMODO_DIR"
fi

# Verify no running daemon remains
if pgrep -f "komodo-android-periphery" >/dev/null 2>&1; then
    printf "${RED}Warning: Lingering process detected. Forcing termination...${NC}\n"
    pkill -9 -f "komodo-android-periphery" 2>/dev/null || true
fi

printf "\n${GREEN}${BOLD}============================================================\n"
printf "SUCCESS: Komodo Android Periphery uninstalled cleanly.\n"
printf "Local Android agent removed. Komodo Server registration remains.\n"
printf "============================================================${NC}\n"

exit 0
