# Komodo Android Periphery (`komodo-android-periphery`)

A native, standalone Android Periphery daemon that allows rooted Android devices (running Magisk) to connect as managed `Server` nodes to an unmodified [Komodo](https://github.com/moghtech/komodo) Core deployment.

---

## Primary Installation Experience (One-Command Deployment)

Run the following command directly on the Android device via **SSH**, **Android root shell**, or **Termux**:

```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.80:9120" \
  --connect-as="$(hostname)" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

The installer automatically:
1. Detects Android ARM64 architecture.
2. Checks root and Magisk CLI availability.
3. Downloads the prebuilt release artifact (`komodo-android-periphery.zip`).
4. Verifies the SHA-256 cryptographic checksum.
5. Installs the Magisk module via `magisk --install-module`.
6. Generates runtime configuration (`/data/adb/komodo/config.toml`).
7. Starts the background supervisor daemon.
8. Verifies authenticated connection with Komodo Core.
9. Purges the one-time onboarding secret from disk upon successful registration.
10. Offers an optional reboot prompt to validate boot persistence.

---

## What This Is
- **Native Android ARM64 Agent**: Compiled against Bionic libc (`aarch64-linux-android`) running with root privileges (`UID 0`).
- **Official Komodo Protocol**: Outbound WebSocket client implementing Noise XX mutual authentication (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).
- **Real-Time Telemetry**: Multi-rate collectors for CPU, Memory, Disk, Network, Process (780+ Android processes), Battery, and Thermal status.
- **Root Web Terminal**: Sentinel-framed interactive `/system/bin/sh` session inside Komodo web dashboard.
- **Magisk Module Lifecycle**: Managed via Magisk App with `late_start` boot persistence, disable switches, and clean uninstall hooks.

## What This Is Not
- **NEVER builds on the phone**: The phone downloads prebuilt release artifacts; no Rust, Cargo, Python, NDK, or compilers are needed on device.
- **Zero ADB dependency**: ADB is used strictly during development. The installer runs 100% on-device.
- **Zero Core or Linux Periphery modifications**: Komodo Core and Linux Periphery remain 100% untouched.

---

## Documentation Directory
- [INSTALL.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/INSTALL.md): Step-by-step one-command installation guide and options.
- [INSTALLATION-ARCHITECTURE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/INSTALLATION-ARCHITECTURE.md): Magisk module vs `service.d` architectural evaluation, filesystem layout, and security model.
- [UPGRADE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/UPGRADE.md): Seamless upgrades, snapshot backups, and rollback instructions.
- [UNINSTALL.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/UNINSTALL.md): Complete uninstallation workflow.
- [TROUBLESHOOTING.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/TROUBLESHOOTING.md): Diagnostic CLI, log analysis, and network verification.
- [PHASE1_REPORT.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/PHASE1_REPORT.md): Validation report with live physical telemetry from Xiaomi Redmi Note 10 Pro.
- [PROTOCOL-CONFORMANCE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/PROTOCOL-CONFORMANCE.md): Formal wire protocol and Noise state machine specifications.
- [ARCHITECTURE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/ARCHITECTURE.md): Multi-rate scheduler, lock-free cache, and subprotocols.
- [DEVICE-CAPABILITIES.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/DEVICE-CAPABILITIES.md): Dynamic discovery engine and capability registry.
