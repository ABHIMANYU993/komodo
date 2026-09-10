# Komodo Android Periphery (`komodo-android-periphery`) — v2.4.2

A native, standalone Android Periphery daemon that connects rooted Android devices (**Magisk**, **KernelSU**, or **APatch**) as high-performance, low-overhead monitored server nodes to Komodo Core.

---

## Architecture & Design Highlights

- **Native `aarch64-linux-android` Binary**: Compiled against Android Bionic libc with low memory footprint and execution as `UID 0` (root).
- **Cockpit/Beszel Low-Overhead Telemetry**:
  - Background 1s procfs scanning loops eliminated.
  - Process queries (780+ Android processes) run **on-demand** when requested by the UI, with an **800ms coalescing cache**. Idle CPU utilization stays at **~0.0%**.
- **Snapdragon / big.LITTLE Utilization Clamping**: Dynamic hotplug detection and total percentage clamping (`0.0..=100.0%`) to eliminate artificial multi-thousand percent CPU utilization spikes.
- **Noise XX Mutual Authentication**: Wire-level zero-trust encrypted WebSocket client (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).
- **Graceful Lifecycle Management**: Auto-disconnects running daemon instances before modifying keys or modules to eliminate key collisions and "Not OK" state in Core.
- **Root Web Terminal**: Interactive `/system/bin/sh` shell session inside the Komodo Core dashboard.

---

## One-Command Deployment

Execute directly on the Android device via **Termux** (`su`), **ADB root shell** (`adb shell su`), or **SSH**:

### Install via `curl`:
```bash
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

### Install via `wget`:
```bash
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

---

## Lifecycle Commands

| Operation | Command |
|---|---|
| **Check Status** | `sh setup-android-periphery.sh --status` |
| **Restart Daemon** | `sh setup-android-periphery.sh --restart` |
| **Update Module** | `sh setup-android-periphery.sh --update` |
| **Fresh Reinstall** | `sh setup-android-periphery.sh --reinstall --core-address="ws://..." --onboarding-key="NEW_KEY"` |
| **Reconfigure** | `sh setup-android-periphery.sh --reconfig --core-address="ws://..."` |
| **Uninstall** | `sh setup-android-periphery.sh --uninstall` |
| **Purge (Wipe All)** | `sh setup-android-periphery.sh --uninstall --purge` |

👉 *For more command options and examples, see [**`example-commands.md`**](../example-commands.md).*

---

## Filesystem Layout

```
/data/adb/
├── komodo/
│   ├── config.toml           # Core URL, node name, polling rate
│   ├── keys/                 # Ed25519 authentication keys
│   │   ├── periphery.key     # Private key (chmod 600)
│   │   └── periphery.pub     # Public key
│   ├── daemon.log            # Daemon standard output and error log
│   └── daemon.pid            # Supervisor background process PID
└── modules/
    └── komodo-android-periphery/
        ├── module.prop       # Magisk module metadata
        ├── service.sh        # late_start service watchdog
        ├── customize.sh      # Magisk installation script
        ├── uninstall.sh      # Magisk module removal script
        └── komodo-android-periphery  # Stripped aarch64 native binary
```

---

## Documentation Directory

- [INSTALL.md](INSTALL.md): In-depth setup options and manual installation.
- [INSTALLATION-ARCHITECTURE.md](INSTALLATION-ARCHITECTURE.md): Magisk module architecture, boot persistence, and security model.
- [UPGRADE.md](UPGRADE.md): Upgrade workflow and rollback instructions.
- [UNINSTALL.md](UNINSTALL.md): Clean uninstallation and purge steps.
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md): Diagnostic CLI and troubleshooting steps.
- [PROTOCOL-CONFORMANCE.md](PROTOCOL-CONFORMANCE.md): Wire protocol and Noise encryption state machine.
- [ARCHITECTURE.md](ARCHITECTURE.md): Multi-rate scheduler and telemetry collectors.
- [DEVICE-CAPABILITIES.md](DEVICE-CAPABILITIES.md): Dynamic discovery engine and capability registry.
- [Example Commands Guide](../example-commands.md): Quick reference for `curl` and `wget` commands.
