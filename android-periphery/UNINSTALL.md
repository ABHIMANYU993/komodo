# Uninstalling Komodo Android Periphery

This document details the uninstallation process for removing Komodo Android Periphery completely and cleanly from an Android device.

---

## Method 1: Canonical One-Command Uninstaller (Recommended)

Run directly from the Android root shell, SSH, or Termux:

```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/uninstall-android-periphery.sh | sh
```

### Unattended / Non-Interactive
To skip the destructive confirmation prompt:
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/uninstall-android-periphery.sh | sh -s -- -y
```

---

## What Gets Removed

The uninstaller purges **all** project-owned artifacts:
- The running daemon process (terminated gracefully with `SIGTERM`, falling back to `SIGKILL`).
- The Magisk module directory: `/data/adb/modules/komodo-android-periphery/`.
- Staged updates: `/data/adb/modules_update/komodo-android-periphery/`.
- Legacy service scripts: `/data/adb/service.d/komodo-android-periphery.sh`.
- Persistent state directory: `/data/adb/komodo/` (including `config.toml`, identity keys, logs, and backups).
- Binary symlinks: `/data/adb/komodo/bin/komodo-control`.

### What is Preserved
The uninstaller **never** touches:
- Magisk core files or unrelated Magisk modules.
- SSH / Dropbear / OpenSSH configurations or authorized keys.
- Termux or system packages.
- User files outside `/data/adb/komodo/`.
- The Komodo Server record in Core (must be deleted via Komodo UI or API if desired).

---

## Method 2: Magisk Manager App Uninstall

You can also remove the module through the Magisk app:
1. Open the **Magisk App** on the device.
2. Navigate to the **Modules** tab.
3. Locate **Komodo Android Periphery** and tap **Remove**.
4. Reboot the device.

*Note: Removing via Magisk App triggers the module's `uninstall.sh` to stop the daemon and delete module files. To also wipe `/data/adb/komodo` (keys and configs), run `rm -rf /data/adb/komodo` in a root shell.*
