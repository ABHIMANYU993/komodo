#!/system/bin/sh
# Komodo Android Periphery Service Script
# Executed by Magisk in late_start service mode

MODDIR="${0%/*}"
KOMODO_DIR="/data/adb/komodo"
CONFIG_FILE="$KOMODO_DIR/config.toml"
DAEMON_BIN="$MODDIR/komodo-android-periphery"
LOG_FILE="$KOMODO_DIR/daemon.log"

# Check if module is disabled via Magisk
if [ -f "$MODDIR/disable" ]; then
    exit 0
fi

# Wait for Android system boot completion (max 60 seconds)
# Do NOT block indefinitely or wait for network routes
BOOT_TIMEOUT=60
COUNT=0
while [ "$(getprop sys.boot_completed)" != "1" ]; do
    sleep 1
    COUNT=$((COUNT + 1))
    if [ $COUNT -ge $BOOT_TIMEOUT ]; then
        break
    fi
done

# Ensure configuration directories exist with strict permissions
mkdir -p "$KOMODO_DIR"
mkdir -p "$KOMODO_DIR/keys"
mkdir -p "$KOMODO_DIR/logs"
chmod 700 "$KOMODO_DIR"
chmod 700 "$KOMODO_DIR/keys"
chmod 700 "$KOMODO_DIR/logs"

# Seed default config if absent
if [ ! -f "$CONFIG_FILE" ]; then
    if [ -f "$MODDIR/config.toml.example" ]; then
        cp "$MODDIR/config.toml.example" "$CONFIG_FILE"
        chmod 600 "$CONFIG_FILE"
    fi
fi

# Ensure daemon binary is executable
if [ -f "$DAEMON_BIN" ]; then
    chmod 755 "$DAEMON_BIN"
fi

# Prevent duplicate supervisors
if pgrep -f "$DAEMON_BIN" >/dev/null 2>&1; then
    exit 0
fi

# Launch daemon in background with supervisor watchdog
# Daemon handles network delays, route changes, and reconnects asynchronously
(
    echo "$$" > "$KOMODO_DIR/daemon.pid"
    while true; do
        # Exit supervisor if module was disabled
        if [ -f "$MODDIR/disable" ]; then
            pkill -TERM -f "$DAEMON_BIN" 2>/dev/null || true
            exit 0
        fi

        if [ -x "$DAEMON_BIN" ] && [ -f "$CONFIG_FILE" ]; then
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Starting komodo-android-periphery daemon..." >> "$LOG_FILE"
            "$DAEMON_BIN" --config "$CONFIG_FILE" >> "$LOG_FILE" 2>&1
            EXIT_CODE=$?
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] komodo-android-periphery exited with code $EXIT_CODE. Restarting in 5s..." >> "$LOG_FILE"
        else
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Binary or config missing. Sleeping 10s..." >> "$LOG_FILE"
            sleep 10
        fi
        sleep 5
    done
) &
