# Development & Cross-Compilation Guide

## 1. Prerequisites
- **Rust Toolchain**: `rustc 1.98.0+` with target `aarch64-linux-android`.
  ```bash
  rustup target add aarch64-linux-android
  ```
- **Android NDK**: r26d or newer.
  Location: `/home/icebyte/work/android-ndk-r26d`
- **Connected Device**: Rooted Android device accessible via `adb devices`.

---

## 2. Cargo Cross-Compilation Configuration

In `android-periphery/.cargo/config.toml`:
```toml
[target.aarch64-linux-android]
linker = "/home/icebyte/work/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang"
ar = "/home/icebyte/work/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-z", "-C", "link-arg=max-page-size=16384"
]
```

---

## 3. Build & Test Commands

### Run Unit Tests on Host
```bash
cargo test --package komodo-android-periphery
```

### Build Android Release Binary
```bash
cargo build --package komodo-android-periphery --target aarch64-linux-android --release
```

### Push to Physical Device via ADB
```bash
adb push target/aarch64-linux-android/release/komodo-android-periphery /data/adb/komodo/bin/
adb shell "su -c 'chmod 755 /data/adb/komodo/bin/komodo-android-periphery'"
```
