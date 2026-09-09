use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct BatteryInfo {
    pub capacity_percent: Option<u32>,
    pub status: Option<String>,
    pub health: Option<String>,
    pub current_now_ua: Option<i64>,
    pub voltage_now_uv: Option<u64>,
    pub temp_deci_c: Option<i32>,
    pub cycle_count: Option<u32>,
    pub charge_type: Option<String>,
}

pub struct BatteryCollector {
    battery_dir: Option<PathBuf>,
}

impl BatteryCollector {
    pub fn new() -> Self {
        Self {
            battery_dir: Self::discover_battery_path(),
        }
    }

    /// Dynamically discover battery power_supply path under `/sys/class/power_supply/`
    fn discover_battery_path() -> Option<PathBuf> {
        let base = Path::new("/sys/class/power_supply");
        let Ok(entries) = fs::read_dir(base) else {
            return None;
        };

        let mut candidate = None;

        for entry in entries.flatten() {
            let path = entry.path();
            let type_path = path.join("type");
            if let Ok(content) = fs::read_to_string(&type_path) {
                if content.trim().eq_ignore_ascii_case("battery") {
                    return Some(path);
                }
            }

            // Secondary check: name contains "batt" or "bms"
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("battery") || name.contains("batt") || name.contains("bms") {
                candidate = Some(path);
            }
        }

        candidate
    }

    /// Read raw battery statistics directly from sysfs.
    pub fn collect(&self) -> BatteryInfo {
        let Some(ref dir) = self.battery_dir else {
            return BatteryInfo::default();
        };

        let read_u32 = |name: &str| -> Option<u32> {
            fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
        };

        let read_i32 = |name: &str| -> Option<i32> {
            fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
        };

        let read_i64 = |name: &str| -> Option<i64> {
            fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
        };

        let read_u64 = |name: &str| -> Option<u64> {
            fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
        };

        let read_str = |name: &str| -> Option<String> {
            fs::read_to_string(dir.join(name)).ok().map(|s| s.trim().to_string())
        };

        BatteryInfo {
            capacity_percent: read_u32("capacity"),
            status: read_str("status"),
            health: read_str("health"),
            current_now_ua: read_i64("current_now"),
            voltage_now_uv: read_u64("voltage_now"),
            temp_deci_c: read_i32("temp"),
            cycle_count: read_u32("cycle_count"),
            charge_type: read_str("charge_type"),
        }
    }
}
