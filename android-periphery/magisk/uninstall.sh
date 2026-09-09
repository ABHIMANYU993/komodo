#!/system/bin/sh
# Magisk Module Uninstallation Script
# Executed by Magisk when the module is removed via Magisk App

# 1. Terminate running daemon processes gracefully
pkill -TERM -f "komodo-android-periphery" 2>/dev/null || true
sleep 1
pkill -KILL -f "komodo-android-periphery" 2>/dev/null || true

# 2. Log uninstallation event
if [ -d "/data/adb/komodo" ]; then
    echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Magisk module uninstalled." >> /data/adb/komodo/daemon.log 2>/dev/null || true
fi
