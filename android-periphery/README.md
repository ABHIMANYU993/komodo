# Komodo Android Periphery (`komodo-android-periphery`)

A native, standalone Android Periphery daemon that allows rooted Android devices (running Magisk) to connect as managed `Server` nodes to an unmodified [Komodo](https://github.com/moghtech/komodo) Core deployment.

## What This Is
- A native compiled Rust daemon (`aarch64-linux-android`) running as root (`UID 0`) on Android.
- An outbound WebSocket client implementing the official Komodo Periphery protocol (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).
- A dynamic telemetry engine providing real-time CPU, Memory, Disk, Network, Process, Battery, and Thermal metrics into Komodo's web dashboard.
- A native PTY root terminal server providing an interactive `/system/bin/sh` session inside the browser.
- Packaged as a standard Magisk module (`late_start` service).

## What This Is Not
- **Not a Komodo redesign**: Komodo Core and the official Komodo UI remain 100% untouched.
- **Not a modification of the Linux Periphery**: `bin/periphery` remains reference code.
- **Not dependent on Termux**: The daemon runs natively under Magisk root userspace.
- **Not dependent on ADB or SSH**: ADB is strictly for development and debugging; the production daemon connects via outbound WebSocket directly to Core.
- **Not hard-coded to any phone**: Dynamically discovers CPU clusters, frequencies, thermal zones, battery paths, and network interfaces.

## Quick Start

### 1. Build
```bash
# Set Android NDK toolchain path
export ANDROID_NDK_HOME=/home/icebyte/work/android-ndk-r26d
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

# Build for aarch64-linux-android
cargo build --package komodo-android-periphery --target aarch64-linux-android --release
```

### 2. Configure
Create `/data/adb/komodo/config.toml` on the device:
```toml
# Komodo Core WebSocket URL
core_url = "ws://192.168.1.50:8120"

# Node identity in Komodo
connect_as = "Android_Redmi_Note_10_Pro"

# Optional onboarding key (required only for initial registration)
# onboarding_key = "your_onboarding_key_here"

# Log level (trace, debug, info, warn, error)
log_level = "info"
```

### 3. Deploy
```bash
# Push binary
adb push target/aarch64-linux-android/release/komodo-android-periphery /data/adb/komodo/bin/
adb shell "su -c 'chmod 755 /data/adb/komodo/bin/komodo-android-periphery'"

# Test run
adb shell "su -c '/data/adb/komodo/bin/komodo-android-periphery --config /data/adb/komodo/config.toml'"
```

---

## Documentation Directory
- [IMPLEMENTATION_BASELINE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/IMPLEMENTATION_BASELINE.md): Git baseline, toolchain, and commit pins.
- [PROTOCOL-CONFORMANCE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/PROTOCOL-CONFORMANCE.md): Formal wire protocol and Noise state machine specifications.
- [ARCHITECTURE.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/ARCHITECTURE.md): Multi-rate scheduler, lock-free cache, and subprotocols.
- [COMPATIBILITY.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/COMPATIBILITY.md): Android versions, SoC vendor adapters, and SELinux considerations.
- [SECURITY.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/SECURITY.md): Threat model, key storage, permissions, and audit logging.
- [DEVELOPMENT.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/DEVELOPMENT.md): Build targets, cross-compilation setup, and test runner.
- [PROTOCOL.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/PROTOCOL.md): Framing layout, discriminators, and channel UUID multiplexing.
- [DEVICE-CAPABILITIES.md](file:///home/icebyte/Projects/Personal/Android/Redmi_Note_10_Pro/komodo/android-periphery/DEVICE-CAPABILITIES.md): Dynamic discovery engine and capability registry.
