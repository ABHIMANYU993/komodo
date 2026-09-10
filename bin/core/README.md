# Komodo Core (`komodo_core`) — v2.4.2

Komodo Core is the central management server for the Komodo monitoring and management platform. It aggregates telemetry from Linux and Android Periphery nodes, exposes the REST and WebSocket APIs, provides alerting, and serves the Vite/React web dashboard.

---

## Architecture & Responsibilities

1. **On-Demand Dynamic Telemetry Aggregation**:
   - `GetSystemStats`: Serves system metrics (CPU, RAM, Disks, Load Average, Network) on-demand to the web dashboard. Features an 800ms coalescing cache that queries the Periphery agent's `PollStatus { include_stats: true }` dynamically according to the user's selected UI interval (from 1s to 1d).
   - `ListContainers`: Inspects container statistics on-demand with an 800ms coalescing cache (`PollStatus { include_docker: true }`), updating dynamically based on the Containers dropdown.
   - `ListSystemProcesses`: Proxies process table queries on-demand, completely avoiding continuous procfs iterations on idle nodes.
2. **WebSocket Communication Engine**:
   - Maintains encrypted, bidirectional WebSocket sessions with Periphery agents.
   - Handles onboarding tokens, key exchange, and state synchronization.
   - Instantly detects clean WebSocket closure when an agent is stopped, avoiding dangling sessions and false "Not OK" reports.
3. **Web Dashboard Integration**:
   - Built with React 19, Mantine UI, Vite, and TanStack React Query.
   - Integrated dynamic dropdowns on **Current Stats** (default `1-sec`), **Containers** (default `15-sec`), and **Processes** (default `5-sec`), driving live background refetch intervals without browser reload.
   - Bundled directly into the Core container image under `/app/ui`.

---

## Building from Source

```bash
# Build release binary locally with all cores:
cargo build --release -p komodo_core -j $(nproc)

# Build Docker container image locally:
docker build -f bin/core/single-arch.Dockerfile \
  --build-arg "BINARIES_IMAGE=ghcr.io/abhimanyu993/komodo-binaries:2" \
  -t ghcr.io/abhimanyu993/komodo-core:latest \
  -t ghcr.io/abhimanyu993/komodo-core:2.4.2 .
```

---

## Configuration

Core is configured via `/config/core.config.toml` or environment variables:

| Variable | Description | Default |
|---|---|---|
| `KOMODO_DATABASE_ADDRESS` | MongoDB connection address | `mongo:27017` |
| `KOMODO_PORT` | HTTP/WS listen port | `9120` |
| `KOMODO_KEEP_STATS_FOR_DAYS` | Database telemetry retention window | `7` |
| `KOMODO_KEEP_ALERTS_FOR_DAYS` | Alert history retention window | `7` |
| `KOMODO_MONITORING_INTERVAL` | Background heartbeat check interval | `5-sec` |
