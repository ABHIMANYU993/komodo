use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermalCategory {
    Cpu,
    Gpu,
    Battery,
    Pmic,
    Skin,
    Modem,
    Npu,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ThermalZoneInfo {
    pub zone_id: u32,
    pub sensor_type: String,
    pub category: ThermalCategory,
    pub temp_c: f32,
}

pub struct ThermalCollector {
    zones: Vec<(u32, String, ThermalCategory)>,
}

impl ThermalCollector {
    pub fn new() -> Self {
        Self {
            zones: Self::discover_zones(),
        }
    }

    /// Dynamically enumerate `/sys/class/thermal/thermal_zone*` and classify sensor types.
    fn discover_zones() -> Vec<(u32, String, ThermalCategory)> {
        let mut list = Vec::new();
        let base = Path::new("/sys/class/thermal");
        let Ok(entries) = fs::read_dir(base) else {
            return list;
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            if name_str.starts_with("thermal_zone") {
                if let Ok(id) = name_str["thermal_zone".len()..].parse::<u32>() {
                    let type_path = entry.path().join("type");
                    let sensor_type = fs::read_to_string(type_path)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("zone{id}"));

                    let category = Self::classify_sensor(&sensor_type);
                    list.push((id, sensor_type, category));
                }
            }
        }

        list.sort_by_key(|z| z.0);
        list
    }

    /// Classify sensor type string into normalized hardware categories.
    fn classify_sensor(name: &str) -> ThermalCategory {
        let lower = name.to_lowercase();
        if lower.contains("cpu") || lower.contains("ap-") || lower.contains("cluster") || lower.contains("core") {
            ThermalCategory::Cpu
        } else if lower.contains("gpu") || lower.contains("kgsl") || lower.contains("mali") {
            ThermalCategory::Gpu
        } else if lower.contains("batt") || lower.contains("bms") {
            ThermalCategory::Battery
        } else if lower.contains("pmic") || lower.contains("charger") {
            ThermalCategory::Pmic
        } else if lower.contains("skin") || lower.contains("case") || lower.contains("chg") {
            ThermalCategory::Skin
        } else if lower.contains("modem") || lower.contains("pa") || lower.contains("cellular") {
            ThermalCategory::Modem
        } else if lower.contains("npu") || lower.contains("dsp") {
            ThermalCategory::Npu
        } else {
            ThermalCategory::Unknown
        }
    }

    /// Read temperatures for all discovered zones.
    pub fn collect(&self) -> Vec<ThermalZoneInfo> {
        let mut results = Vec::with_capacity(self.zones.len());

        for (id, sensor_type, category) in &self.zones {
            let temp_path = format!("/sys/class/thermal/thermal_zone{id}/temp");
            if let Ok(content) = fs::read_to_string(&temp_path) {
                if let Ok(raw_temp) = content.trim().parse::<f32>() {
                    // On most Android kernels, sysfs temp is in millidegrees C (e.g. 42000 -> 42.0 C)
                    // If already < 200, assume raw Celsius
                    let temp_c = if raw_temp > 200.0 { raw_temp / 1000.0 } else { raw_temp };
                    results.push(ThermalZoneInfo {
                        zone_id: *id,
                        sensor_type: sensor_type.clone(),
                        category: category.clone(),
                        temp_c,
                    });
                }
            }
        }

        results
    }
}
