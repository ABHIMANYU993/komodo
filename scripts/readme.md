# Komodo Host & Device Periphery Installer Scripts

This directory contains the canonical, production-ready POSIX shell installers for deploying and managing Komodo Periphery across Linux servers and rooted Android devices.

---

## Scripts Overview

| Script | Target Platform | Architecture | Supported Init / Supervison |
|---|---|---|---|
| [`setup-periphery.sh`](setup-periphery.sh) | Linux Hosts & VMs | `x86_64`, `aarch64`, `armv7` | systemd, OpenRC, runit, s6, dinit, SysVinit |
| [`setup-android-periphery.sh`](setup-android-periphery.sh) | Rooted Android | `aarch64` | Magisk (v26+), KernelSU, APatch |
| [`build-release-local.sh`](build-release-local.sh) | Local Development Host | Multi-core build runner | Host native + cross-compilation |
| [`auto-release.sh`](auto-release.sh) | Git Hooks Runner | Post-commit automation | Local release pipeline |

---

## 1. `setup-periphery.sh` (Linux Host System Service)

A zero-dependency POSIX shell installer that installs Komodo Periphery as a **native host system service**.

> [!IMPORTANT]
> **Zero Container Fallback**: This script strictly avoids falling back to Docker or Podman on VM/host nodes. Periphery runs directly on the host system as a native daemon managed by the host's init system.

### Features
- **Auto-Detects Init Systems**:
  - `systemd`: Registers `/etc/systemd/system/periphery.service`
  - `OpenRC`: Registers `/etc/init.d/periphery` (e.g. Alpine Linux)
  - `runit`: Registers `/etc/sv/periphery` (e.g. Void Linux)
  - `s6 / s6-rc`: Registers `/var/service/periphery`
  - `dinit`: Registers `/etc/dinit.d/periphery` (e.g. Chimera Linux)
  - `SysVinit`: Registers `/etc/init.d/periphery`
- **Libc Compatibility**: Detects musl vs glibc and deploys static musl binaries on musl hosts.
- **Graceful Lifecycle Management**: Rerunning the installer or executing `--reinstall`, `--update`, or `--restart` gracefully stops running processes and closes WebSocket connections before modifying keys or binaries, preventing connection lockouts.

### Quick Usage

```bash
# Install / Onboard
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Alpine_VM" \
  --onboarding-key="YOUR_ONBOARDING_KEY"

# Check Status
sh setup-periphery.sh --status

# Update Binary (Preserving Keys and Config)
sh setup-periphery.sh --update

# Fresh Reinstall (Purging Keys)
sh setup-periphery.sh --reinstall --core-address="ws://192.168.31.100:9120" --onboarding-key="NEW_KEY"
```

---

## 2. `setup-android-periphery.sh` (Rooted Android)

A standalone installer for rooted Android devices running Magisk, KernelSU, or APatch.

### Features
- **Direct On-Device Execution**: Runs directly inside Termux (`su`), ADB root shell (`adb shell su`), or on-device SSH.
- **Module Packaging**: Automatically verifies SHA256 integrity and installs via `magisk --install-module`, `ksud module install`, or direct filesystem staging into `/data/adb/modules/komodo-android-periphery`.
- **Background Watchdog**: Launches a resilient background daemon that survives network reconnects and deep sleep.
- **Graceful Lifecycle**: Stops running daemons and closes WebSocket connections before modifying keys.

### Quick Usage

```bash
# Install / Onboard
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"

# Check Daemon Status
sh setup-android-periphery.sh --status

# Update to Latest Release
sh setup-android-periphery.sh --update

# Complete Purge
sh setup-android-periphery.sh --uninstall --purge
```

---

## 3. Action Reference

| Action Flag | Description |
|---|---|
| `--install` | Standard installation and registration (default) |
| `--reinstall` | Fresh installation: gracefully disconnects, purges previous keys, and onboards with new key |
| `--reconfig` | Updates Core address, node name, or polling interval without deleting keys |
| `--update`, `--upgrade` | Upgrades binary or module to latest version, keeping configs and keys intact |
| `--restart` | Gracefully terminates running daemon and brings service back up |
| `--status` | Displays native init/daemon status, running PID, and active config |
| `--uninstall` | Removes service/module and binary; preserves `/etc/komodo` or `/data/adb/komodo` |
| `--purge` | Used with `--uninstall` to completely delete configuration and keys |

---

## 4. Comprehensive Command Reference

For extensive copy-pasteable `curl` and `wget` commands covering all options, see:  
👉 [**`example-commands.md`**](../example-commands.md)