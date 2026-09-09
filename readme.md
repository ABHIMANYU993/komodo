# Komodo 🦎 — ABHIMANYU993 Fork (v2.4.0)

> **This is a custom fork of [moghtech/komodo](https://github.com/moghtech/komodo)** with major enhancements for **real-time monitoring** and **native Android deployment** via Magisk.

---

## What's New in v2.4.0

| Feature | Detail |
|---|---|
| 🤖 **Android Periphery** | Native `aarch64` binary. Runs on rooted Android via Magisk — no Docker needed |
| ⚡ **1-Second Real-Time Stats** | CPU, RAM, disk, load average, and network all update every second |
| 📊 **1s / 2s / 3s Historical Graphs** | Selectable granularity for telemetry history charts |
| 🔄 **Process Refresh Dropdown** | Processes table update interval matches the dropdown you select (1s – 1day) |
| 🌐 **Public IP in Header** | Android device's public/local IP is displayed in the dashboard header |
| 🗄️ **Auto DB Retention (7 days)** | MongoDB stats and alerts auto-pruned weekly — no manual cleanup |
| 📦 **Custom Docker Images** | `ghcr.io/abhimanyu993/komodo-core` and `ghcr.io/abhimanyu993/komodo-periphery` built from this repo |

---

## 🐳 Docker Deployment (Server / Linux)

### Prerequisites
- Docker + Docker Compose v2
- A Linux server (x86-64 or arm64)

### Quick Start

```bash
# 1. Clone this repo
git clone https://github.com/ABHIMANYU993/komodo.git
cd komodo

# 2. Copy and configure the env file
cp compose/compose.env compose/.env.local
# Edit .env.local — set passwords, KOMODO_HOST, etc.

# 3. Deploy with MongoDB
docker compose -f compose/mongo.compose.yaml --env-file compose/compose.env up -d
```

The core dashboard will be available at **http://your-server-ip:9120**.

**Default login** (change immediately!):
- Username: `admin`
- Password: `changeme`

### Images Used

| Service | Image |
|---|---|
| Core | `ghcr.io/abhimanyu993/komodo-core:2` |
| Periphery | `ghcr.io/abhimanyu993/komodo-periphery:2` |
| MongoDB | `mongo` (official) |

Images are automatically built and pushed to GHCR on every push to `main` and on every GitHub Release via the [docker-build-push workflow](.github/workflows/docker-build-push.yaml).

### ENV File Reference

The main env file is [`compose/compose.env`](compose/compose.env). Key variables:

| Variable | Default | Description |
|---|---|---|
| `COMPOSE_KOMODO_IMAGE_TAG` | `2` | Docker image tag to pull |
| `KOMODO_HOST` | `https://example.komodo.com` | Your public URL (used for OAuth / webhooks) |
| `KOMODO_INIT_ADMIN_USERNAME` | `admin` | Initial admin username |
| `KOMODO_INIT_ADMIN_PASSWORD` | `changeme` | Initial admin password — **change this!** |
| `KOMODO_DATABASE_USERNAME` | `admin` | MongoDB username |
| `KOMODO_DATABASE_PASSWORD` | `admin` | MongoDB password — **change this!** |
| `KOMODO_MONITORING_INTERVAL` | `1-sec` | How often Core polls servers (1-sec for real-time) |
| `PERIPHERY_STATS_POLLING_RATE` | `1-sec` | How often Periphery samples system stats |
| `KOMODO_KEEP_STATS_FOR_DAYS` | `7` | Auto-prune stats older than N days |
| `KOMODO_KEEP_ALERTS_FOR_DAYS` | `7` | Auto-prune alerts older than N days |
| `KOMODO_JWT_SECRET` | `a_random_jwt_secret` | **Set a strong random value!** |
| `KOMODO_WEBHOOK_SECRET` | `a_random_secret` | **Set a strong random value!** |

### Upgrade

```bash
# Pull latest images and restart
docker compose -f compose/mongo.compose.yaml --env-file compose/compose.env pull
docker compose -f compose/mongo.compose.yaml --env-file compose/compose.env up -d
```

---

## 🤖 Android Periphery Deployment (Magisk)

The Android Periphery is a standalone native daemon that connects your **rooted Android phone** to Komodo Core as a monitored server.

### Prerequisites
- Rooted Android device (Magisk v26+)
- Root shell access (`adb shell su` or Termux with root)
- Network access from the phone to your Komodo Core server

### Install via Magisk Module

```bash
# Download the latest release package
curl -L https://github.com/ABHIMANYU993/komodo/releases/latest/download/komodo-android-periphery.zip \
     -o /sdcard/komodo-android-periphery.zip

# Install via Magisk (preferred — survives updates cleanly)
su -c "magisk --install-module /sdcard/komodo-android-periphery.zip"

# Reboot to activate
su -c "reboot"
```

### Post-Install Configuration

After reboot, edit the config file:

```bash
su -c "nano /data/adb/modules/komodo-android-periphery/config/komodo-android-periphery.toml"
```

Minimum config:

```toml
core_address = "ws://192.168.x.x:9120"      # Your Komodo Core WS address (e.g. ws://192.168.1.100:9120)
server_name  = "Android-Device"            # Name shown in Komodo UI (e.g. Pixel-7 or Android-Device)
```

Reload the daemon:

```bash
su -c "komodo-control restart"
```

### Control Commands

```bash
su -c "komodo-control status"    # Check if daemon is running
su -c "komodo-control start"     # Start daemon
su -c "komodo-control stop"      # Stop daemon
su -c "komodo-control restart"   # Restart daemon
su -c "komodo-control logs"      # View recent logs
su -c "komodo-control uninstall" # Remove module + all files
```

### What is Monitored

| Metric | Update Rate |
|---|---|
| CPU usage % | 1 second |
| RAM used / total | 1 second |
| Disk usage | 1 second |
| Load average (1m/5m/15m) | 1 second |
| Network ingress / egress | 1 second |
| Process list (CPU/Mem per process) | Configurable (1s–1day) |
| Public / local IP | Every 15 minutes |

---

## 🔨 Building from Source

### Android Periphery

```bash
# Install Rust + Android target
rustup target add aarch64-linux-android

# Setup NDK (set your NDK path)
export ANDROID_NDK_HOME=/path/to/android-ndk

# Build + package
./android-periphery/package.sh
# Output: android-periphery/dist/komodo-android-periphery.zip
```

### Docker Images (CI handles this automatically)
Images are built and pushed to GHCR automatically via the GitHub Actions workflow. For local builds:

```bash
# Build core image locally
docker build -f bin/core/aio.Dockerfile -t ghcr.io/abhimanyu993/komodo-core:local .

# Build periphery image locally
docker build -f bin/periphery/aio.Dockerfile -t ghcr.io/abhimanyu993/komodo-periphery:local .
```

---

## 📐 Architecture

```
┌─────────────────────────────┐      WebSocket
│  Android Device (Magisk)    │─────────────────────────────────────┐
│  komodo-android-periphery   │                                     │
│  (aarch64 native binary)    │                                     ▼
└─────────────────────────────┘              ┌────────────────────────────┐
                                             │   Komodo Core              │
┌─────────────────────────────┐   WebSocket  │   ghcr.io/abhimanyu993/   │
│  Linux Server (Docker)      │─────────────▶│   komodo-core:2            │
│  komodo-periphery container │             │                            │
└─────────────────────────────┘             │   Port 9120 (HTTP + WS)    │
                                             │   MongoDB backend          │
                                             └────────────────────────────┘
```

---

## 🔗 Links

- [Upstream Komodo docs](https://komo.do)
- [Upstream GitHub (moghtech/komodo)](https://github.com/moghtech/komodo)
- [This fork's releases](https://github.com/ABHIMANYU993/komodo/releases)

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
