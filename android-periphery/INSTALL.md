# Installing Komodo Android Periphery

This guide describes how to deploy the native Android Periphery agent to any rooted Android device (ARM64) running Magisk.

---

## Prerequisites

- **Root Access**: Magisk v24+ (Tested up to Magisk v30.7).
- **Architecture**: ARM64 (`aarch64-linux-android`, Android 10 / API 29 through Android 14+ / API 35).
- **Network**: Direct network connectivity from the phone to Komodo Core (e.g. WiFi, VPN, or Tailscale).
- **Standard Utilities**: Stock `/system/bin/sh`, Magisk, and `curl` (present on modern Android devices).
- **ZERO Compilation on Phone**: The phone does NOT require Python, Rust, Cargo, Android NDK, Docker, or ADB.

---

## One-Command Installation (Direct on Device)

Run the following command directly on the Android device via **SSH**, **Android root shell**, or **Termux**:

```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.80:9120" \
  --connect-as="android-102" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

### CLI Parameters

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--core-address` | **Required** | — | WebSocket endpoint of your Komodo Core instance (e.g. `ws://192.168.1.100:9120`). |
| `--onboarding-key` | **Required** (1st time) | — | One-time bootstrap token generated in Komodo Core (`CreateOnboardingKey`). Purged after successful onboarding. |
| `--connect-as` | Optional | `$(hostname)` | Unique server name to appear in Komodo Core. |
| `--version` | Optional | `latest` | Specify a release tag, e.g. `v0.1.0`. |
| `--artifact-url` | Optional | GitHub Releases | Custom URL to prebuilt `komodo-android-periphery.zip`. |
| `--non-interactive` | Optional | `false` | Run without interactive prompts (skips reboot prompt). |
| `--verbose` | Optional | `false` | Enable verbose diagnostic logging during installation. |
| `--force` | Optional | `false` | Force reinstall even if already installed and healthy. |

---

## Installation Lifecycle & Stages

When executed, the installer advances through 8 automated stages:

```text
[1/8] Detecting Android environment...
      Identifies Android OS release, API level, and verifies ARM64 CPU compatibility.
[2/8] Checking root and Magisk availability...
      Verifies root UID and checks that Magisk CLI is operational.
[3/8] Downloading release artifact...
      Downloads prebuilt komodo-android-periphery.zip and .sha256 from GitHub Releases.
[4/8] Verifying artifact integrity...
      Validates cryptographic SHA-256 checksum and tests archive integrity.
[5/8] Installing Magisk module via official CLI...
      Invokes `magisk --install-module` to stage files cleanly into Magisk.
[6/8] Configuring Periphery service...
      Writes `/data/adb/komodo/config.toml` (0600) with core URL and onboarding token.
[7/8] Starting Periphery agent...
      Launches the supervisor process to initiate background connectivity.
[8/8] Verifying Komodo connection...
      Monitors logs until authenticated Noise XX session is established.
      Safely purges the temporary onboarding key from config.toml upon verification.
```

---

## Post-Installation & Reboot

After completing stage 8, the installer outputs:
```text
==========================================================
  SUCCESS: Android Periphery installed and connected!
  Node 'android-102' is now communicating with Komodo Core.
  Verify the Server shows OK in the Komodo UI.
==========================================================
```

### Reboot Recommendation
The installer will ask:
```text
============================================================
WARNING
The Android Periphery service will start automatically after
reboot.
Reboot is recommended to validate boot persistence.
Current SSH session will be disconnected by reboot.
Reboot now? [y/N]
============================================================
```
- **N (Default)**: Returns cleanly. The agent is already running in memory and connected. You can reboot at your convenience.
- **Y**: Initiates a clean system reboot to immediately verify Magisk boot persistence.

---

## Managing the Service

Use the bundled control utility:

```bash
# Check service status & active PID
/data/adb/modules/komodo-android-periphery/komodo-control status

# Restart daemon
/data/adb/modules/komodo-android-periphery/komodo-control restart

# Stop daemon
/data/adb/modules/komodo-android-periphery/komodo-control stop

# Start daemon
/data/adb/modules/komodo-android-periphery/komodo-control start

# View real-time logs
/data/adb/modules/komodo-android-periphery/komodo-control logs

# Run local system & network diagnostics
/data/adb/modules/komodo-android-periphery/komodo-control diagnose
```
*(Also accessible at `/data/adb/komodo/bin/komodo-control`)*
