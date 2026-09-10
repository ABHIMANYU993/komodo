# Komodo Linux Periphery (`komodo_periphery`) — v2.4.2

Komodo Periphery is the lightweight host telemetry agent and Docker/process manager for Linux environments. It connects outbound over WebSocket to Komodo Core, exposing real-time host metrics, container controls, and terminal sessions.

---

## Key Architecture & Optimizations

1. **Native Host Service Model**:
   - Installed as a native system service via [`scripts/setup-periphery.sh`](../../scripts/setup-periphery.sh) with support for **systemd, OpenRC, runit, s6, dinit, and SysVinit**.
   - Zero container fallback: runs natively with minimal resource consumption, no nested Docker overhead, and direct host kernel/cgroup access.
   - Statically-linked musl binaries (`periphery-x86_64-musl`) run seamlessly on Alpine Linux and all standard glibc distributions.
2. **Low-Overhead Telemetry (Cockpit / Beszel Design)**:
   - Process list collection is **strictly on-demand** via `ListSystemProcesses`. Idle nodes do not burn CPU traversing `/proc`.
   - Docker inspection derives Compose project metadata in-memory from container labels with **zero child subprocess forks** (`docker compose ls` eliminated).
   - Filesystem `statvfs` calls are throttled to eliminate disk I/O thrashing.
3. **Graceful Disconnection**:
   - Signal handlers (`SIGTERM`, `SIGINT`) cleanly close active WebSocket sessions, signaling normal closure (Code 1000) to Komodo Core before the process terminates.
   - Prevents stale connection locks and "Not OK" status when updating or restarting the agent.

---

## Deployment Commands

### Standard Installation:
```bash
# Via curl:
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Alpine_VM" \
  --onboarding-key="YOUR_ONBOARDING_KEY"

# Via wget:
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Ubuntu_Server" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

For full update, reinstall, restart, and status command examples, see [**`example-commands.md`**](../../example-commands.md).

---

## Building from Source

```bash
# Build dynamically linked host binary:
cargo build --release -p komodo_periphery -j $(nproc)

# Build statically linked musl binary (universal):
cargo build --release -p komodo_periphery --target x86_64-unknown-linux-musl -j $(nproc)
```
