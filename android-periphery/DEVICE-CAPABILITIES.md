# Android Device Capabilities Registry

## 1. Dynamic Discovery Engine

The daemon detects all hardware components and kernel features dynamically during startup without hard-coding device paths:

```
                      +-----------------------------+
                      |   Runtime Discovery Pass    |
                      +--------------+--------------+
                                     |
         +---------------------------+---------------------------+
         |                           |                           |
         v                           v                           v
+-------------------+       +-------------------+       +-------------------+
|  CPU Topology     |       | Power & Battery   |       | Thermal Zones     |
| - /proc/stat      |       | - /sys/class/     |       | - /sys/class/     |
| - policy0..N      |       |   power_supply/*  |       |   thermal/        |
| - scaling_cur_freq|       | - capacity/temp   |       |   thermal_zone*   |
+-------------------+       +-------------------+       +-------------------+
         |                           |                           |
         v                           v                           v
+-------------------+       +-------------------+       +-------------------+
|  GPU Subsystem    |       | Filesystems       |       | Network           |
| - Adreno (KGSL)   |       | - /proc/mounts    |       | - /proc/net/dev   |
| - Mali (sysfs)    |       | - statvfs API     |       | - RX / TX deltas  |
+-------------------+       +-------------------+       +-------------------+
```

---

## 2. Capabilities Table

| Capability Key | Description | Fast Kernel Source | Android Framework Fallback |
|---|---|---|---|
| `cpu.total` | Aggregate CPU utilization % | `/proc/stat` delta | N/A |
| `cpu.per_core` | Per-core CPU utilization % | `/proc/stat` delta | N/A |
| `cpu.frequency` | Per-core current frequency (kHz) | `/sys/devices/system/cpu/cpufreq/policy*/scaling_cur_freq` | N/A |
| `memory.total` | Total system RAM | `/proc/meminfo: MemTotal` | N/A |
| `memory.available` | Available system RAM | `/proc/meminfo: MemAvailable` | N/A |
| `storage.mounts` | Mounted partitions capacity/usage | `statvfs` on `/data`, `/` | N/A |
| `network.interfaces` | Network interfaces bandwidth & drops | `/proc/net/dev` delta | N/A |
| `processes.list` | Running process table | `/proc/[pid]/stat`, `/proc/[pid]/status` | N/A |
| `battery.level` | Battery state of charge % | `/sys/class/power_supply/*/capacity` | `cmd battery get level` |
| `battery.temperature` | Battery temperature (0.1°C) | `/sys/class/power_supply/*/temp` | `cmd battery get temp` |
| `thermal.zones` | Thermal sensors and temperature | `/sys/class/thermal/thermal_zone*/temp` | `dumpsys thermalservice` |
| `gpu.utilization` | GPU bus / core busy % | `/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage` | Dynamic vendor adapter |
| `terminal.pty` | Native interactive root shell | `/dev/ptmx` -> `/system/bin/sh` | N/A |
