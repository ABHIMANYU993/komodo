# Protocol Specification: Komodo Periphery Wire Format

## 1. Frame Structure

All WebSocket messages are transmitted as binary frames. The final byte in every frame designates the `TransportMessageVariant`:

```
+------------------------------------------+-----------------------+
| Body / Payload (variable length)         | Variant Byte (1 byte) |
+------------------------------------------+-----------------------+
```

| Variant | Value | Direction | Purpose |
|---|---|---|---|
| `Login` | `0x00` | Bidirectional | Handshake, nonce, public key exchange, success/error |
| `Request` | `0x01` | Core -> Periphery | RPC execution (PollStatus, GetHealth, etc.) |
| `Response`| `0x02` | Periphery -> Core | RPC result (Ok, Err, or Pending) |
| `Terminal`| `0x03` | Bidirectional | Interactive PTY stream |

---

## 2. Response Status Encoding

Response messages (`0x02`) contain a status byte immediately preceding the Channel UUID:

```
+------------------+-----------------------+--------------------+---------------------+
| Data / Serror    | Response Status (1B)  | Channel UUID (16B) | Variant Byte (0x02) |
+------------------+-----------------------+--------------------+---------------------+
```

- `0x00`: `Ok` (Payload contains JSON-serialized result).
- `0x01`: `Err` (Payload contains JSON-serialized `mogh_error::Serror`).
- `0x02`: `Pending` (Keepalive sent every 4 seconds to reset Core's 10-second timeout; payload is empty).

---

## 3. Terminal Protocol Encoding

Terminal messages (`0x03`) use raw binary streams for low-latency interactive shells:

```
+------------------+-----------------------+--------------------+---------------------+
| Raw PTY Bytes    | Response Status (1B)  | Channel UUID (16B) | Variant Byte (0x03) |
+------------------+-----------------------+--------------------+---------------------+
```

- Core transmits keyboard stdin input as `Ok(bytes)`.
- Periphery writes stdin to `/dev/ptmx` and transmits PTY stdout/stderr output frames back as `Ok(bytes)`.
