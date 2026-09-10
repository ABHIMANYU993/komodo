# Komodo Deployment & Lifecycle Example Commands

Comprehensive reference guide providing copy-pasteable `curl` and `wget` commands for deploying, managing, updating, and monitoring both **Linux Host Periphery** and **Android Periphery** agents.

---

## Table of Contents
1. [Linux Periphery Commands (`setup-periphery.sh`)](#1-linux-periphery-commands)
   - [Standard Installation](#linux-standard-installation)
   - [Fresh Reinstallation (Reset Keys)](#linux-fresh-reinstallation)
   - [Update / Upgrade to Latest Binary](#linux-update--upgrade)
   - [Reconfigure Node](#linux-reconfigure)
   - [Service Restart](#linux-service-restart)
   - [Service Status & Diagnostics](#linux-service-status)
   - [Uninstallation](#linux-uninstallation)
   - [Complete Purge](#linux-complete-purge)
2. [Android Periphery Commands (`setup-android-periphery.sh`)](#2-android-periphery-commands)
   - [Standard Installation](#android-standard-installation)
   - [Fresh Reinstallation (Reset Keys)](#android-fresh-reinstallation)
   - [Update / Upgrade to Latest Module](#android-update--upgrade)
   - [Reconfigure Node](#android-reconfigure)
   - [Daemon Restart](#android-daemon-restart)
   - [Daemon Status & Diagnostics](#android-daemon-status)
   - [Uninstallation](#android-uninstallation)
   - [Complete Purge](#android-complete-purge)
3. [Parameter & Flag Reference](#3-parameter--flag-reference)

---

<a name="1-linux-periphery-commands"></a>
## 1. Linux Periphery Commands (`setup-periphery.sh`)

Installs and manages Komodo Periphery as a **native host system service**.  
Automatically detects and registers with:
- **systemd** (`/etc/systemd/system/periphery.service`)
- **OpenRC** (`/etc/init.d/periphery` — e.g. Alpine Linux)
- **runit** (`/etc/sv/periphery` — e.g. Void Linux)
- **s6 / s6-rc** (`/var/service/periphery`)
- **dinit** (`/etc/dinit.d/periphery` — e.g. Chimera Linux)
- **SysVinit** (`/etc/init.d/periphery` — Debian/Ubuntu legacy)

*(Zero container fallback; uses statically-linked musl binaries on musl distros or standard glibc).*

---

<a name="linux-standard-installation"></a>
### Linux: Standard Installation

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Alpine_VM" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Ubuntu_Server" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

---

<a name="linux-fresh-reinstallation"></a>
### Linux: Fresh Reinstallation (Reset Keys)

Gracefully stops the active service, closes WebSocket connections, purges previous authentication keys, and registers anew with a fresh onboarding key:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --reinstall \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Alpine_VM" \
  --onboarding-key="YOUR_NEW_ONBOARDING_KEY"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --reinstall \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Ubuntu_Server" \
  --onboarding-key="YOUR_NEW_ONBOARDING_KEY"
```

---

<a name="linux-update--upgrade"></a>
### Linux: Update / Upgrade to Latest Binary

Downloads and replaces the binary at `/usr/local/bin/periphery` while keeping your existing configuration and keys intact:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --update
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --update
```

---

<a name="linux-reconfigure"></a>
### Linux: Reconfigure Node

Updates target Core address, polling interval, or node name and restarts service:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --reconfig \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Production_Node_01" \
  --polling-rate="1-sec"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --reconfig \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Production_Node_01" \
  --polling-rate="1-sec"
```

---

<a name="linux-service-restart"></a>
### Linux: Service Restart

Gracefully terminates the active daemon, waits for socket release, and brings the service back up:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --restart
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --restart
```

---

<a name="linux-service-status"></a>
### Linux: Service Status & Diagnostics

Displays native init status, binary location, version info, and current configuration:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --status
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --status
```

---

<a name="linux-uninstallation"></a>
### Linux: Uninstallation

Stops service, removes binary and init service unit, keeping `/etc/komodo` configuration:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --uninstall
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --uninstall
```

---

<a name="linux-complete-purge"></a>
### Linux: Complete Purge

Stops service, uninstalls binary, and completely wipes `/etc/komodo` keys and configurations:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --uninstall --purge
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- --uninstall --purge
```

---

<a name="2-android-periphery-commands"></a>
## 2. Android Periphery Commands (`setup-android-periphery.sh`)

Installs and manages Komodo Android Periphery as a **native ARM64 Magisk / KernelSU / APatch module**.  
Supports Android 8.0 through Android 15+. Requires root (`su`).

---

<a name="android-standard-installation"></a>
### Android: Standard Installation

Execute via **Termux** (`su`), **ADB root shell** (`adb shell su`), or **SSH**:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

---

<a name="android-fresh-reinstallation"></a>
### Android: Fresh Reinstallation (Reset Keys)

Gracefully stops the background daemon, waits for socket release, purges existing cryptographic keys in `/data/adb/komodo/keys`, and registers with the new key:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --reinstall \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_NEW_ONBOARDING_KEY"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --reinstall \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_NEW_ONBOARDING_KEY"
```

---

<a name="android-update--upgrade"></a>
### Android: Update / Upgrade to Latest Module

Downloads the latest version zip, verifies SHA256 checksum, updates the module files, and reloads the daemon:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --update
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --update
```

---

<a name="android-reconfigure"></a>
### Android: Reconfigure Node

Updates target Core address, polling interval, or node name without losing existing authentication keys:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --reconfig \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Android_Server_01" \
  --polling-rate="1-sec"
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --reconfig \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Android_Server_01" \
  --polling-rate="1-sec"
```

---

<a name="android-daemon-restart"></a>
### Android: Daemon Restart

Gracefully restarts the background watchdog and agent daemon:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --restart
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --restart
```

---

<a name="android-daemon-status"></a>
### Android: Daemon Status & Diagnostics

Displays module installation status, running PID, public key, and active configuration:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --status
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --status
```

---

<a name="android-uninstallation"></a>
### Android: Uninstallation

Removes Magisk module and terminates running processes, preserving `/data/adb/komodo` configuration:

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --uninstall
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --uninstall
```

---

<a name="android-complete-purge"></a>
### Android: Complete Purge

Stops daemon, uninstalls module, and completely wipes `/data/adb/komodo` directory (all keys and configurations):

**Via `curl`:**
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --uninstall --purge
```

**Via `wget`:**
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- --uninstall --purge
```

---

<a name="3-parameter--flag-reference"></a>
## 3. Parameter & Flag Reference

| Flag / Option | Description | Default |
|---|---|---|
| `--core-address=<url>` | WebSocket URL of Komodo Core (e.g. `ws://192.168.31.100:9120`) | *Required for install* |
| `--connect-as=<name>` | Server node identifier in Komodo UI | Hostname |
| `--onboarding-key=<key>` | One-time onboarding token generated by Core | *Required for first registration* |
| `--polling-rate=<rate>` | System stats polling rate (`1-sec`, `5-sec`, `15-sec`, etc.) | `1-sec` |
| `--version=<tag>` | Specific release tag to install | `v2.4.2` |
| `--reinstall` | Force fresh setup & purge previous authentication keys | `false` |
| `--reconfig` | Update Core address, node name, or polling rate without resetting keys | `false` |
| `--update`, `--upgrade` | Upgrade binary / module to latest release while keeping configuration | `false` |
| `--restart` | Gracefully restart daemon / service | `false` |
| `--status` | Display running status, PID, config file, and public keys | `false` |
| `--uninstall` | Remove service / module and binary | `false` |
| `--purge` | Used alongside `--uninstall` to delete all keys and configs | `false` |
| `--non-interactive` | Bypass interactive confirmation menus (Android) | `false` |
| `--force` | Force execution without interactive prompts | `false` |
