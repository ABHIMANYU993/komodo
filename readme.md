# Komodo 🦎 — Real-Time Telemetry & Native Multi-Platform Infrastructure (v2.4.2)

> **High-performance, low-overhead monitoring and container management fork of [moghtech/komodo](https://github.com/moghtech/komodo)** with native system service agents for Linux, native ARM64 agents for rooted Android, on-demand telemetry sampling, and dynamic real-time UI polling rate controls.

---

## What's New in v2.4.2

| Feature | Detail |
|---|---|
| 🎛️ **Dynamic Polling Rate Controls** | Live UI dropdown selectors on **Current Stats** (default `1-sec`), **Containers** (default `15-sec`), and **Processes** (default `5-sec`), selectable from `1s` to `1d`. Dynamic changes instantly drive Periphery polling without page refreshes. |
| 🪶 **Cockpit / Beszel Telemetry Architecture** | Zero background procfs polling storms. Process inspection and container stats run **on-demand** with an 800ms coalescing cache, dropping idle daemon CPU to **~0.0%**. |
| 🐧 **Native Linux Host Periphery Service** | One-command installer [`setup-periphery.sh`](scripts/setup-periphery.sh) with native support for **systemd, OpenRC, runit, s6, dinit, SysVinit**. Zero container overhead; static musl support for Alpine and glibc Linux. |
| 🤖 **Native Android Periphery** | Standalone `aarch64` daemon via [`setup-android-periphery.sh`](scripts/setup-android-periphery.sh) for rooted Android devices (**Magisk v26+, KernelSU, APatch**). Zero-trust Noise XX encryption. |
| 🛑 **Graceful Lifecycle & Re-onboarding** | Installers automatically detect active processes and gracefully terminate WebSocket connections before updating configs or keys, eliminating key conflicts and "Not OK" state. |
| 🐳 **Optimized Container Inspection** | Zero child process spawning: Docker Compose project names and state are derived in-memory from container labels with no `docker compose ls` subprocess forks. |
| 📱 **Qualcomm Snapdragon Core Normalization** | Hotplug detection and CPU metric clamping (`0.0..=100.0%`) to eliminate erroneous multi-thousand percent CPU utilization spikes on big.LITTLE architectures. |
| 🚀 **Local Multi-Core Build Automation** | Parallelized local builds utilizing all available host CPU cores (`nproc`) to build binaries, package Magisk modules, and assemble multi-tagged Docker images. |

---

## ⚡ Quick Start & Helping Commands

For a full reference of commands, see the dedicated [**Example Commands Guide (`example-commands.md`)**](example-commands.md).

### 1. Deploy Linux Periphery (Host System Service)

Runs as a native system service (systemd, OpenRC, runit, s6, dinit, SysVinit):

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

### 2. Deploy Android Periphery (Rooted Android / Magisk / KernelSU / APatch)

Run in **Termux** (`su`), **ADB root shell**, or **SSH**:

```bash
# Via curl:
curl -fsSL https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"

# Via wget:
wget -qO- https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/scripts/setup-android-periphery.sh | sh -s -- \
  --core-address="ws://192.168.31.100:9120" \
  --connect-as="Redmi_Note_10_Pro" \
  --onboarding-key="YOUR_ONBOARDING_KEY"
```

### 3. Essential Lifecycle Commands

| Action | Linux Periphery (`setup-periphery.sh`) | Android Periphery (`setup-android-periphery.sh`) |
|---|---|---|
| **Check Status** | `sh setup-periphery.sh --status` | `sh setup-android-periphery.sh --status` |
| **Restart** | `sh setup-periphery.sh --restart` | `sh setup-android-periphery.sh --restart` |
| **Update Binary** | `sh setup-periphery.sh --update` | `sh setup-android-periphery.sh --update` |
| **Fresh Reinstall** | `sh setup-periphery.sh --reinstall --core-address=... --onboarding-key=...` | `sh setup-android-periphery.sh --reinstall --core-address=... --onboarding-key=...` |
| **Reconfigure** | `sh setup-periphery.sh --reconfig --core-address=...` | `sh setup-android-periphery.sh --reconfig --core-address=...` |
| **Uninstall** | `sh setup-periphery.sh --uninstall` | `sh setup-android-periphery.sh --uninstall` |
| **Full Purge** | `sh setup-periphery.sh --uninstall --purge` | `sh setup-android-periphery.sh --uninstall --purge` |

👉 *For more copy-pasteable commands and advanced options, read [`example-commands.md`](example-commands.md).*

---

## 🐳 Docker Deployment (Komodo Core Server)

### Prerequisites
- Docker / Podman + Compose
- Linux server (`x86_64` or `aarch64`)

### Quick Start

```bash
# 1. Clone repository
git clone https://github.com/ABHIMANYU993/komodo.git
cd komodo

# 2. Configure environment
cp compose/compose.env compose/.env.local

# 3. Deploy Core with MongoDB
docker compose -f compose/mongo.compose.yaml --env-file compose/compose.env up -d
```

Dashboard will be available at **http://your-server-ip:9120**.

**Default Credentials**:
- Username: `admin`
- Password: `changeme`

### Container Images

| Service | Image | Description |
|---|---|---|
| **Core** | `ghcr.io/abhimanyu993/komodo-core:latest` (`:2.4.2`, `:2`) | Core server, API resolvers, Web UI bundle |
| **Periphery** | `ghcr.io/abhimanyu993/komodo-periphery:latest` (`:2.4.2`, `:2`) | Linux container periphery agent |
| **Android Periphery** | `ghcr.io/abhimanyu993/komodo-android-periphery:latest` (`:2.4.2`, `:2`) | Distributable artifact container |

---

## 📐 Architecture Overview

```
┌──────────────────────────────────────┐
│       Rooted Android Device          │
│   (Magisk / KernelSU / APatch)       │
│    komodo-android-periphery (aarch64)│
│    - Noise XX mutual auth            │
│    - On-demand proc & battery stats  │
└──────────────────┬───────────────────┘
                   │
                   │ Outbound WebSocket (Encrypted)
                   ▼
┌──────────────────────────────────────┐            ┌──────────────────────────────────────┐
│          Komodo Core (v2.4.2)        │            │        Linux Host / VM Node          │
│   ghcr.io/abhimanyu993/komodo-core   │◀───────────│   (Alpine, Debian, Ubuntu, Arch)     │
│   - Port 9120 (HTTP / WS)            │  WebSocket │   periphery (systemd/openrc service) │
│   - Dynamic polling rate engine      │            │   - Zero container fallback          │
│   - On-demand coalescing cache (800ms│            │   - Native cgroups & procfs          │
│   - MongoDB backend                  │            └──────────────────────────────────────┘
└──────────────────┬───────────────────┘
                   │
                   │ Web UI (Vite / React 19 / Mantine)
                   ▼
┌──────────────────────────────────────┐
│         Browser Dashboard            │
│   - Current Stats dropdown (1s-1d)   │
│   - Containers dropdown (1s-1d)      │
│   - Processes dropdown (1s-1d)       │
└──────────────────────────────────────┘
```

---

## 🔨 Multi-Core Building from Source

All binaries and images can be built locally using all available CPU threads:

```bash
# Build release binaries (core, periphery, km, static musl) and container images
./scripts/build-release-local.sh v2.4.2

# Build Android Periphery & Magisk package only
./android-periphery/package.sh
```

---

## 🔗 Documentation Links

- [Example Commands Guide (`example-commands.md`)](example-commands.md)
- [Linux Periphery Installer Documentation](scripts/readme.md)
- [Android Periphery Documentation](android-periphery/README.md)
- [Core Architecture Guide](bin/core/README.md)
- [Periphery Architecture Guide](bin/periphery/README.md)
- [Upstream Komodo Documentation](https://komo.do)

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
