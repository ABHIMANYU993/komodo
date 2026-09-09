use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub vendor: String,
    pub model: Option<String>,
    pub utilization_percent: Option<f32>,
    pub cur_freq_hz: Option<u64>,
    pub max_freq_hz: Option<u64>,
}

pub enum GpuAdapter {
    /// Qualcomm Adreno via KGSL driver (`/sys/class/kgsl/kgsl-3d0`)
    QualcommKgsl(PathBuf),
    /// ARM Mali (`/sys/class/misc/mali0`, `/sys/devices/platform/*mali*`)
    ArmMali(PathBuf),
    /// Device has no discovered GPU sysfs telemetry node
    None,
}

pub struct GpuCollector {
    adapter: GpuAdapter,
}

impl GpuCollector {
    pub fn new() -> Self {
        Self {
            adapter: Self::detect_gpu_subsystem(),
        }
    }

    fn detect_gpu_subsystem() -> GpuAdapter {
        // 1. Check Qualcomm KGSL
        let kgsl_path = PathBuf::from("/sys/class/kgsl/kgsl-3d0");
        if kgsl_path.exists() {
            return GpuAdapter::QualcommKgsl(kgsl_path);
        }

        // 2. Check ARM Mali
        for mali_candidate in ["/sys/class/misc/mali0", "/sys/devices/platform/mali"] {
            let path = PathBuf::from(mali_candidate);
            if path.exists() {
                return GpuAdapter::ArmMali(path);
            }
        }

        GpuAdapter::None
    }

    pub fn collect(&self) -> GpuMetrics {
        match &self.adapter {
            GpuAdapter::QualcommKgsl(dir) => {
                let read_u64 = |name: &str| -> Option<u64> {
                    fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
                };

                let read_str = |name: &str| -> Option<String> {
                    fs::read_to_string(dir.join(name)).ok().map(|s| s.trim().to_string())
                };

                // Qualcomm exposes `gpu_busy_percentage` or `gpubusy`
                let utilization = if let Some(pct) = read_u64("gpu_busy_percentage") {
                    Some(pct as f32)
                } else if let Some(raw) = read_str("gpubusy") {
                    // gpubusy format: "busy_cycles total_cycles"
                    let parts: Vec<&str> = raw.split_whitespace().collect();
                    if parts.len() == 2 {
                        let busy: f64 = parts[0].parse().unwrap_or(0.0);
                        let total: f64 = parts[1].parse().unwrap_or(0.0);
                        if total > 0.0 {
                            Some(((busy / total) * 100.0) as f32)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let cur_freq = read_u64("gpuclk").or_else(|| read_u64("devfreq/cur_freq"));
                let max_freq = read_u64("max_gpuclk").or_else(|| read_u64("devfreq/max_freq"));
                let model = read_str("gpu_model");

                GpuMetrics {
                    vendor: "Qualcomm".to_string(),
                    model,
                    utilization_percent: utilization,
                    cur_freq_hz: cur_freq,
                    max_freq_hz: max_freq,
                }
            }
            GpuAdapter::ArmMali(dir) => {
                let read_u64 = |name: &str| -> Option<u64> {
                    fs::read_to_string(dir.join(name)).ok().and_then(|s| s.trim().parse().ok())
                };

                let cur_freq = read_u64("clock").or_else(|| read_u64("cur_freq"));

                GpuMetrics {
                    vendor: "ARM".to_string(),
                    model: Some("Mali".to_string()),
                    utilization_percent: None,
                    cur_freq_hz: cur_freq,
                    max_freq_hz: None,
                }
            }
            GpuAdapter::None => GpuMetrics {
                vendor: "Unknown".to_string(),
                model: None,
                utilization_percent: None,
                cur_freq_hz: None,
                max_freq_hz: None,
            },
        }
    }
}
