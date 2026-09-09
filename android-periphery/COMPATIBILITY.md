# Compatibility Matrix: Komodo Android Periphery

## 1. Android OS & API Levels

| Android Version | API Level | Support Status | Notes |
|---|---|---|---|
| Android 10 (Q)  | 29 | Fully Supported | Baseline Bionic target |
| Android 11 (R)  | 30 | Fully Supported | Scoped storage active |
| Android 12 (S)  | 31/32 | Fully Supported | Phantom process limits (mitigated by Magisk UID 0) |
| Android 13 (T)  | 33 | **Primary Target** | Verified on Xiaomi Redmi Note 10 Pro test device |
| Android 14 (U)  | 34 | Fully Supported | Verified libc compatibility |
| Android 15 (V)  | 35 | Fully Supported | 16KB page alignment verified |

---

## 2. Hardware Platforms & SoCs

The agent utilizes **runtime capability discovery** rather than hard-coded vendor profiles.

| Vendor | Platform / SoC Family | Telemetry Adapter | Support Level |
|---|---|---|---|
| **Qualcomm** | Snapdragon (SM7150, SM8250, etc.) | `AdrenoKgslCollector` (`/sys/class/kgsl/kgsl-3d0`) | Full (Utilization, Frequency, Throttle) |
| **MediaTek** | Dimensity / Helio | `MaliMtkCollector` (`/proc/gpufreq`, `/sys/class/misc/mali0`) | Dynamic Frequency & Thermal |
| **Samsung** | Exynos | `MaliExynosCollector` | Dynamic Frequency |
| **Generic** | Any ARM64 SoC | Generic Fallback (`/proc/stat`, `/proc/meminfo`) | Full CPU, RAM, Storage, Network |

---

## 3. SELinux & Execution Context

### Diagnose First, Policy Second
Per implementation rules, we do not ship broad `sepolicy.rule` blindly.
1. The daemon runs under Magisk's `service.sh` at `late_start`.
2. On modern Magisk (v25+), processes spawned from `service.sh` inherit the Magisk root domain (`u:r:magisk:s0`).
3. During startup, the daemon records `id`, `getenforce`, and its active context.
4. Any AVC denial observed in `logcat -b events` or `dmesg` is captured and audited before introducing targeted rules into `magisk/sepolicy.rule`.
