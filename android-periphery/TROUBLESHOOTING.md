# Troubleshooting Komodo Android Periphery

This guide provides diagnostic procedures and resolutions for common issues encountered during Android Periphery deployment and operation.

---

## 1. Quick Diagnostics CLI

Run the built-in diagnostic tool directly on the device:

```bash
/data/adb/modules/komodo-android-periphery/komodo-control diagnose
```

Sample output:
```text
=== Komodo Android Diagnostics ===
UID:            0 (expected 0)
SELinux Mode:   Enforcing
Android API:    33
Architecture:   arm64-v8a
Magisk Version: 30.7
Module Path:    /data/adb/modules/komodo-android-periphery
Binary Found:   /data/adb/modules/komodo-android-periphery/komodo-android-periphery
Config Found:   /data/adb/komodo/config.toml
Core Endpoint:  ws://192.168.31.80:9120
Network Reachability to 192.168.31.80:9120: REACHABLE
===================================
```

---

## 2. Common Issues & Solutions

### A. "Error: Magisk not found in PATH"
- **Cause**: The script was run in a non-root context where Magisk environment is not exposed, or the device is not rooted with Magisk.
- **Resolution**: Ensure the device is rooted with Magisk v24+ and that root permission was granted in the Magisk superuser prompt. Run `su` prior to executing the script.

### B. "Network Reachability UNREACHABLE"
- **Cause**: The Android device cannot route packets to the Core IP address or port 9120.
- **Resolution**:
  1. Test basic connectivity: `ping -c 3 <core-ip>` or `curl -I http://<core-ip>:9120/`.
  2. Verify that your host firewall allows ingress on port 9120 (`sudo ufw allow 9120/tcp`).
  3. Verify that both the phone and host are connected to the same Wi-Fi network (or VPN).

### C. "Onboarding Noise handshake failed"
- **Cause**: The onboarding token entered was expired, already consumed, or invalid.
- **Resolution**:
  1. In Komodo Core UI, create a new onboarding key for your server name.
  2. Rerun the installer with `--onboarding-key="NEW_KEY"`.

### D. "Server shows NotOk in Komodo Core"
- **Cause**: The agent process terminated, or network connection was dropped.
- **Resolution**:
  1. Inspect the agent logs:
     ```bash
     tail -n 50 /data/adb/komodo/daemon.log
     ```
  2. Restart the agent:
     ```bash
     /data/adb/modules/komodo-android-periphery/komodo-control restart
     ```

### E. Daemon does not start after reboot
- **Cause**: Magisk module was toggled off, or Magisk did not trigger `late_start`.
- **Resolution**:
  1. Verify module is enabled in Magisk App -> Modules.
  2. Check if the `disable` flag exists:
     ```bash
     ls -l /data/adb/modules/komodo-android-periphery/disable
     ```
     If it exists, remove it: `rm -f /data/adb/modules/komodo-android-periphery/disable`.
  3. Start the agent manually using `komodo-control start`.
