# Security Architecture & Threat Model

## 1. Privilege & Process Model
The `komodo-android-periphery` daemon runs as `UID 0 (root)` and `GID 0 (root)` on Android devices.
- **No Listening Ports**: The daemon connects **outbound** to Komodo Core. It binds no local TCP or UDP ports on the phone, completely eliminating inbound attack vectors from the local network, Wi-Fi hotspots, or cellular interfaces.
- **Isolated State Storage**: All configuration, cryptographic keys, and runtime state reside in `/data/adb/komodo/` (mode `0700`, owner `root:root`).

---

## 2. Cryptographic Authentication & Zero Trust
1. **Asymmetric Identity**:
   - Every agent generates and maintains an X25519 keypair (`periphery_key.pkcs8`).
   - The public key is in SubjectPublicKeyInfo (SPKI) PEM format.
2. **Noise XX Handshake**:
   - `Noise_XX_25519_ChaChaPoly_BLAKE2s` mutual authentication.
   - Core and Periphery authenticate each other without transmitting private keys.
3. **Zero-Trust Prologue Binding**:
   - Handshake messages are cryptographically bound to the WebSocket session parameters:
     `SHA-256("noise-wss-v1|" + host + "|" + query + "|" + accept + "|" + nonce)`.
   - Prevents connection spoofing, man-in-the-middle downgrade attacks, or cross-connection replay.

---

## 3. Root Terminal Security
1. The browser terminal executes `/system/bin/sh` with root permissions.
2. Terminal sessions are multiplexed strictly over the authenticated Noise WebSocket channel.
3. When the browser disconnects or the channel is closed, the daemon reaps the child process via `SIGHUP` and `SIGKILL`, releasing PTY file descriptors immediately.
