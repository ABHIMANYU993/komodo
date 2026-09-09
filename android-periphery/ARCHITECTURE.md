# Architecture: Komodo Android Periphery

## 1. High-Level System Architecture

```
+-----------------------------------------------------------------------------------+
|                              Android Device (Root / UID 0)                        |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  |                komodo-android-periphery (Native Rust Daemon)                |  |
|  |                                                                             |  |
|  |  +-----------------------------------------------------------------------+  |  |
|  |  |                  Multi-Rate Telemetry Engine                          |  |  |
|  |  |  - FAST (1s):   /proc/stat (CPU), /proc/meminfo (RAM), /proc/net/dev   |  |  |
|  |  |  - MEDIUM (3s): /proc/[pid] (Procs), /sys/class/power_supply          |  |  |
|  |  |  - SLOW (30s):  statvfs (Disks), /sys/class/thermal, Device Info       |  |  |
|  |  +-----------------------------------+-----------------------------------+  |  |
|  |                                      |                                      |  |
|  |                                      v                                      |  |
|  |                     +---------------------------------+                     |  |
|  |                     |   Atomic In-Memory Metric Cache |                     |  |
|  |                     +---------------------------------+                     |  |
|  |                                      |                                      |  |
|  |                                      v (Zero lock wait)                     |  |
|  |  +-----------------------------------------------------------------------+  |  |
|  |  |              Protocol Dispatcher & Request Resolvers                  |  |  |
|  |  |  - PollStatus        - GetHealth           - GetVersion               |  |  |
|  |  |  - SystemProcesses   - Native PTY Terminal (PTY Master / /system/bin/sh) |  |
|  |  +-----------------------------------+-----------------------------------+  |  |
|  |                                      |                                      |  |
|  |  +-----------------------------------+-----------------------------------+  |  |
|  |  |              Outbound Secure WebSocket Client                         |  |  |
|  |  |  - Noise_XX_25519_ChaChaPoly_BLAKE2s (Prologue SHA-256)               |  |  |
|  |  |  - Auto-reconnect with exponential backoff                             |  |  |
|  |  |  - 5-second WebSocket ping/pong keepalive                              |  |  |
|  |  |  - 4-second "Pending" progress frames for long RPCs                    |  |  |
|  |  +-----------------------------------+-----------------------------------+  |  |
|  +--------------------------------------|--------------------------------------+  |
+-----------------------------------------|-----------------------------------------+
                                          |
                        Outbound TLS/WS (Port 8120/443)
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                               Komodo Core Host                                    |
|                                                                                   |
|  +---------------------------+  WebSocket   +----------------------------------+  |
|  | Komodo Core Daemon        |<============>| Periphery Connection Manager     |  |
|  | (bin/core)                |              | - Receives PollStatus telemetry  |  |
|  +-------------+-------------+              | - Routes Terminal I/O            |  |
|                |                            +----------------------------------+  |
|                v                                                                  |
|  +---------------------------+                                                    |
|  | MongoDB & Server Model    |                                                    |
|  +---------------------------+                                                    |
|                |                                                                  |
|                v                                                                  |
|  +---------------------------+                                                    |
|  | Komodo Web UI             |  (Shows live CPU, RAM, Disk, Procs, Root Shell)    |
|  +---------------------------+                                                    |
+-----------------------------------------------------------------------------------+
```

---

## 2. Telemetry Engine & Multi-Rate Collector Design

Executing heavy Android framework commands (`dumpsys`, `cmd`, `pm`) inside a high-frequency polling loop consumes excessive CPU, triggers garbage collection pauses in Android system servers, and drains battery.

To guarantee $< 0.5\%$ CPU overhead on mobile devices:
1. **Zero Subprocess Forking for Telemetry**:
   - Fast metrics are read directly from kernel pseudo-filesystems (`/proc` and `/sys`) using buffered I/O.
2. **Multi-Rate Scheduling**:
   - **Fast Tier (~1 second)**:
     - CPU aggregate and per-core utilization via differential `/proc/stat`.
     - CPU frequency scaling via `/sys/devices/system/cpu/cpufreq/policy*/scaling_cur_freq`.
     - Memory accounting via `/proc/meminfo` (`MemAvailable` vs `MemTotal`).
     - Network interface bandwidth deltas via `/proc/net/dev`.
   - **Medium Tier (~2–5 seconds)**:
     - Process table enumeration via dynamic `/proc/[pid]/stat` scan with graceful handling of disappearing processes (`ENOENT`).
     - Battery current and voltage via `/sys/class/power_supply/*`.
   - **Slow Tier (~30+ seconds)**:
     - Filesystem statistics using `statvfs` on mounted filesystems (`/data`, `/`).
     - Thermal zone inventory and classification.
     - Device metadata (OS build, kernel release, hardware board).
3. **Decoupled Metric Cache**:
   - The RPC handler for `PollStatus` never invokes collectors synchronously.
   - It clones the latest snapshot from an `ArcSwap` or read-optimized atomic cache, completing in $< 50\,\mu\text{s}$.

---

## 3. Native PTY Root Terminal Architecture

The terminal subprotocol establishes an interactive terminal session inside Komodo UI:
1. Allocates a pseudo-terminal (PTY master/slave pair) via libc `openpty` / `/dev/ptmx`.
2. Sets raw mode and configures window dimensions (default 80x24, dynamically resized via ioctl `TIOCSWINSZ` on resize frames).
3. Forks and executes the discovered shell (preferring `/system/bin/sh` or `/system/bin/bash`) with root UID 0 and GID 0.
4. Streams PTY output asynchronously into binary frames tagged with `TransportMessageVariant::Terminal` (`0x03`).
5. Ingests stdin input frames from Core and writes them into the master PTY descriptor.
6. Automatically reaps child processes and cleans up channel routing upon session termination.
