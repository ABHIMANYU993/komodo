# Android Komodo Periphery Implementation Baseline

## 1. Repository Identifiers
- **Fork URL**: `https://github.com/ABHIMANYU993/komodo` (origin)
- **Upstream URL**: `https://github.com/moghtech/komodo.git` (upstream)
- **Git Branch**: `main`
- **Git Commit SHA**: `780ac68b992094a9fccd5fffb760e0c84fd3c3d1`
- **Git Tag / Describe**: `v2.3.3`
- **Commit Date**: `Tue Sep 1 14:34:49 2026 -0700`
- **Baseline Alignment**: Aligned exactly with official upstream release tag `v2.3.3`.

## 2. Toolchain Baseline
- **Rustc Version**: `rustc 1.98.1 (48a229cea 2026-09-01)`
- **Cargo Version**: `cargo 1.98.1`
- **Primary Production Target**: `aarch64-linux-android` (Android NDK r26d+ / Bionic userspace, API level 29-33+)
- **Secondary Fallback Target**: `aarch64-unknown-linux-musl` (Static binary with lld)
- **Host System**: Arch Linux x86_64 (`clang 18+`, `lld`, `llvm`)

## 3. Versioning Strategy
- **Android Agent Version**: `0.1.0` (independent package version)
- **Protocol Compatibility Specification**: Komodo v2.3.3 wire protocol
- **Upstream Compatibility Commit**: `780ac68b992094a9fccd5fffb760e0c84fd3c3d1`
- `GetVersion` returns `0.1.0` (Android Agent version).
- System information telemetry reports device Android release, SDK level, kernel release, and hardware platform.

## 4. Upstream Protocol Source Files Traced
- `lib/transport/src/auth.rs`:
  - Noise XX handshake initiator & responder state machine
  - Zero-trust identifier prologue calculation (`SHA-256("noise-wss-v1|" + host + "|" + query + "|" + accept + "|" + nonce)`)
  - WebSocket accept hash calculation (`258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
- `client/periphery/rs/src/transport/mod.rs`:
  - `TransportMessageVariant` (`Login=0`, `Request=1`, `Response=2`, `Terminal=3`)
  - `TransportMessage` wire encoding & decoding
  - `RequestMessage`, `ResponseMessage`, `TerminalMessage`
- `client/periphery/rs/src/transport/login.rs`:
  - `LoginMessage` (`Success=0`, `Nonce=1`, `Handshake=2`, `OnboardingFlow=3`, `PublicKey=4`)
- `client/periphery/rs/src/api/mod.rs`:
  - `PeripheryRequest` envelope (`PollStatus`, `GetHealth`, `GetVersion`, `GetSystemProcesses`, `terminal::*`)
- `client/periphery/rs/src/api/poll.rs`:
  - `PollStatus`, `PollStatusResponse`, `PeripheryInformation`, `SystemInformation`, `SystemStats`, `DockerLists`
- `bin/core/src/connection/server.rs`:
  - Inbound WebSocket upgrade `/ws/periphery?server=<connect_as>`
  - Initial `LoginMessage::OnboardingFlow(bool)` dispatch
  - Registration of SPKI public key in MongoDB
- `bin/periphery/src/connection/client.rs`:
  - Outbound WebSocket client loop and retry backoff (`CONNECTION_RETRY_SECONDS = 5`)
  - Onboarding fallback recovery
- `bin/periphery/src/connection/mod.rs`:
  - 4-second `Pending` keepalive loop (`send_in_progress`) to satisfy Core 10s timeout
  - Terminal subprotocol message routing

## 5. Protocol Compatibility Assumptions
1. **Outbound Direction Only**: The Android agent is the WebSocket client (`Periphery -> Core`). No inbound listening ports are exposed on Android.
2. **WebSocket Binary Frames**: Only binary frames are accepted and emitted.
3. **Pure Rust Cryptography**: `snow` with `Noise_XX_25519_ChaChaPoly_BLAKE2s` matches Komodo's `mogh_pki::MutualNoiseHandshake` byte-for-byte.
4. **Key Format**: SPKI X25519 public keys (base64 encoded string), PKCS#8 private keys.
5. **Docker Optional**: `docker: None` in `PollStatusResponse` is completely valid in Komodo Core and renders clean empty UI tables without errors.
6. **Graceful Degradation**: Optional hardware metrics (battery, GPU, thermal) do not cause failures if absent on a specific device.
