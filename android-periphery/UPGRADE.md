# Upgrading Komodo Android Periphery

This document explains how updates and rollbacks work in the Komodo Android Periphery deployment system.

---

## Zero-Downtime Upgrade

To upgrade an existing Komodo Android Periphery installation to the latest release or a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.80:9120"
```

Notice:
- You do **NOT** need to supply `--onboarding-key` during upgrades.
- The agent's permanent identity key (`/data/adb/komodo/keys/periphery.key`) is automatically preserved.
- The Core server registration remains intact.

---

## Upgrade Safety & Rollback Mechanism

When `setup-android-periphery.sh` detects an existing installation:

1. **Integrity Validation**: The new release archive and its SHA-256 checksum are downloaded and validated before any local changes are made.
2. **Automatic Snapshot**: A timestamped backup is saved to `/data/adb/komodo/backups/`:
   ```text
   /data/adb/komodo/backups/backup_YYYYMMDD_HHMMSS/
   ├── config.toml
   ├── periphery.key
   ├── periphery.pub
   └── module.prop
   ```
3. **Graceful Termination**: The running daemon is stopped with `SIGTERM`.
4. **Magisk Staging**: The new package is installed via `magisk --install-module`.
5. **Connection Verification**: The new binary is started and monitored. If it fails to authenticate with Komodo Core within 15 seconds, the installer alerts you and preserves the rollback snapshot.

---

## Manual Rollback Procedure

If you ever need to manually revert to a previous version:

```bash
# 1. Stop the current daemon
/data/adb/modules/komodo-android-periphery/komodo-control stop

# 2. View available backups
ls -la /data/adb/komodo/backups/

# 3. Restore desired configuration and keys
LATEST_BACKUP=$(ls -td /data/adb/komodo/backups/backup_* | head -n 1)
cp "$LATEST_BACKUP/config.toml" /data/adb/komodo/config.toml
cp -a "$LATEST_BACKUP/"*.key /data/adb/komodo/keys/ 2>/dev/null || true

# 4. Restart daemon
/data/adb/modules/komodo-android-periphery/komodo-control start
```
