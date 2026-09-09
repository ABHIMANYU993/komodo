# Komodo Android Periphery — Installation Architecture & Lifecycle Specification

**Author**: ABHIMANYU993  
**Target Platform**: Android ARM64 (`aarch64-linux-android`), Android 10+ (API 29–35)  
**Root Framework**: Magisk (Tested on Magisk v30.7)  
**Reference Device**: Xiaomi Redmi Note 10 Pro (`M2101K6P`, `sweetin`, Android 13, API 33)

---

## 1. Overview & Architectural Decision

To achieve unattended boot persistence, privilege management, and clean lifecycle operations on rooted Android, two mechanisms were investigated and evaluated directly on physical hardware:

1. **Option A: Direct Magisk `service.d` Script** (`/data/adb/service.d/komodo-android-periphery.sh`)
2. **Option B: Standard Magisk Module** (`/data/adb/modules/komodo-android-periphery/` with `module.prop` and `service.sh`)

### Comparative Evaluation

| Criteria | Direct `service.d` | Standard Magisk Module | Selected Architecture |
| :--- | :--- | :--- | :--- |
| **Boot Execution Mode** | `late_start` service mode | `late_start` service mode (`service.sh`) | Tie |
| **SELinux Context** | `u:r:magisk:s0` (root) | `u:r:magisk:s0` (root) | Tie |
| **Magisk App UI Visibility** | Invisible (user cannot see status) | Visible in Modules list with version & description | **Magisk Module** |
| **Toggle Disable / Enable** | Manual shell script modification | Native toggle in Magisk Manager UI (`disable` flag) | **Magisk Module** |
| **Safe Mode Protection** | Unreliable across Magisk versions | Disabled automatically if device boots into Safe Mode | **Magisk Module** |
| **Official Installation API** | Manual file copy into `/data/adb/` | Supported CLI `magisk --install-module <ZIP>` | **Magisk Module** |
| **Update Staging** | Overwriting active running files | Handled via Magisk `modules_update/` atomic staging | **Magisk Module** |
| **Uninstall Hook** | Custom script only | Magisk App uninstall triggers `uninstall.sh` | **Magisk Module** |
| **Official Guidance** | Recommended for ad-hoc scripts only | Recommended by topjohnwu / Magisk documentation | **Magisk Module** |

### Decision: Standard Magisk Module (Canonical)

The **Standard Magisk Module** architecture is selected. It satisfies all operational requirements, provides native Magisk App visibility and disable switches, leverages Magisk's atomic `modules_update` staging mechanism, and ensures clean uninstallation without dangling files.

---

## 2. Directory Layout & Separation of Concerns

To guarantee that application upgrades, rollbacks, and module re-installations never destroy cryptographic identities or user configurations, state is strictly segregated into two separate directory trees:

```text
/data/adb/
├── modules/
│   └── komodo-android-periphery/           <-- IMMUTABLE PROGRAM FILES (Managed by Magisk)
│       ├── module.prop                      <-- Module ID, name, version, description
│       ├── komodo-android-periphery         <-- Native ARM64 ELF executable (PIE, 0755)
│       ├── service.sh                       <-- Magisk late_start boot execution script
│       ├── customize.sh                     <-- Magisk module installation hook
│       ├── uninstall.sh                     <-- Magisk module uninstallation hook
│       ├── komodo-control                   <-- CLI helper (status, start, stop, restart)
│       └── config.toml.example              <-- Reference configuration template
│
├── modules_update/
│   └── komodo-android-periphery/           <-- STAGED UPDATE (During in-flight installs)
│
└── komodo/                                 <-- MUTABLE DATA & SECRETS (Persistent across updates)
    ├── config.toml                          <-- Active daemon configuration (0600)
    ├── keys/                                <-- Cryptographic keys (0700)
    │   ├── periphery.key                    <-- Permanent X25519 private identity key (0600)
    │   └── periphery.pub                    <-- Base64 encoded public key (0644)
    ├── logs/                                <-- Runtime diagnostic logs
    │   ├── daemon.log                       <-- Supervisor & startup logs
    │   └── periphery.log                    <-- Komodo Periphery agent application log
    ├── state/                               <-- Daemon state & PID tracking
    │   └── komodo.pid                       <-- Active process ID file
    └── backups/                             <-- Rollback archives before upgrades
        └── backup_YYYYMMDD_HHMMSS/
            ├── config.toml
            ├── periphery.key
            └── periphery.pub
```

### Critical Rules:
1. **Never delete `/data/adb/komodo/keys/` during upgrade**: The agent's permanent X25519 identity key is recognized by Komodo Core. Regenerating keys breaks Core trust and requires re-onboarding.
2. **Never store user configuration in the module folder**: Everything under `/data/adb/modules/komodo-android-periphery/` may be replaced during an update or purged on module removal.

---

## 3. Installation & Upgrade Lifecycle

### Official Installation Flow (`magisk --install-module`)

Rather than copying files manually into `/data/adb/modules/` (which can corrupt running modules and bypass Magisk internal state tracking), the installer invokes:

```bash
magisk --install-module /path/to/komodo-android-periphery.zip
```

1. **Magisk Staging**: Magisk unzips the archive, executes `customize.sh`, and stages files into `/data/adb/modules_update/komodo-android-periphery/`.
2. **Immediate Activation (Pre-Reboot)**:
   - The installer sets up `/data/adb/komodo/config.toml` with the Core URL and temporary onboarding key.
   - The installer locates the executable binary (checking `/data/adb/modules/` or `/data/adb/modules_update/`).
   - The supervisor starts the daemon in background.
   - The daemon authenticates with Komodo Core and establishes its encrypted Noise XX session.
   - Upon verified registration, the installer purges `onboarding_key` from `config.toml`.
3. **Boot Persistence (Post-Reboot)**:
   - On the next Android reboot, Magisk's `post-fs-data` stage automatically promotes `/data/adb/modules_update/komodo-android-periphery` to `/data/adb/modules/komodo-android-periphery`.
   - In `late_start` service mode, Magisk automatically invokes `service.sh`.
   - `service.sh` waits for `sys.boot_completed=1` and launches the daemon supervisor.

### Upgrade & Rollback Workflow

When an upgrade is performed:
1. **Download & Checksum Verification**: The new `komodo-android-periphery.zip` is downloaded and its SHA-256 is validated.
2. **Backup**: A timestamped snapshot of `/data/adb/komodo/` (`config.toml` and `keys/`) is saved to `/data/adb/komodo/backups/`.
3. **Daemon Stop**: The active daemon is stopped gracefully via `SIGTERM`.
4. **Magisk Install**: `magisk --install-module komodo-android-periphery.zip` stages the new version.
5. **Health Verification**: The new agent binary is launched. If it fails to connect within 15 seconds, the installer automatically restores the previous binary and config from the backup.

---

## 4. Boot Execution & Supervisor (`service.sh`)

Magisk executes `service.sh` in the background during the `late_start` boot phase.

```bash
#!/system/bin/sh
MODDIR="${0%/*}"
KOMODO_DIR="/data/adb/komodo"
CONFIG_FILE="$KOMODO_DIR/config.toml"
DAEMON_BIN="$MODDIR/komodo-android-periphery"
LOG_FILE="$KOMODO_DIR/logs/daemon.log"

# Wait for system boot completion (max 60 seconds)
COUNT=0
while [ "$(getprop sys.boot_completed)" != "1" ]; do
    sleep 1
    COUNT=$((COUNT + 1))
    [ $COUNT -ge 60 ] && break
done

# Launch supervisor loop
(
    while true; do
        if [ -x "$DAEMON_BIN" ] && [ -f "$CONFIG_FILE" ]; then
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Starting komodo-android-periphery..." >> "$LOG_FILE"
            "$DAEMON_BIN" --config "$CONFIG_FILE" >> "$LOG_FILE" 2>&1
            EXIT_CODE=$?
            echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Agent exited ($EXIT_CODE). Restarting in 5s..." >> "$LOG_FILE"
        fi
        sleep 5
    done
) &
```

- **Non-blocking Boot**: Never blocks `init` or waits synchronously for network connectivity.
- **Auto-Recovery**: Asynchronous supervisor restarts the daemon if it ever terminates or if the network temporarily drops.

---

## 5. Security & Onboarding Token Sanitation

The `onboarding_key` is a one-time bootstrap secret issued by Komodo Core.

1. **Write**: The installer writes `onboarding_key = "..."` into `/data/adb/komodo/config.toml` (permissions `0600`).
2. **Handshake**: The agent performs the `OnboardingFlow(true)` exchange, establishing its permanent X25519 identity key (`periphery.key`).
3. **Sanitize**: The installer verifies successful registration, then removes the `onboarding_key` entry from `config.toml`.
4. **Never Log**: Neither the installer nor the agent prints the onboarding secret or private identity keys.

---

## 6. Uninstallation Workflow

Uninstallation is supported through two pathways:

1. **Standalone Uninstaller Script** (`scripts/uninstall-android-periphery.sh`):
   - Gracefully terminates running processes (`SIGTERM` -> `SIGKILL`).
   - Removes `/data/adb/modules/komodo-android-periphery`.
   - Removes `/data/adb/modules_update/komodo-android-periphery`.
   - Removes `/data/adb/service.d/komodo-android-periphery.sh` (legacy).
   - Removes `/data/adb/komodo/` (configs, keys, logs, backups).
   - Verifies zero running processes remain.
2. **Magisk App Uninstall**:
   - Magisk triggers `uninstall.sh` inside the module, terminating the daemon.
