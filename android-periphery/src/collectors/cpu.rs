use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Context, anyhow};

#[derive(Debug, Clone, Default)]
pub struct CpuRawSnapshot {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuRawSnapshot {
    pub fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }

    pub fn busy(&self) -> u64 {
        self.user + self.nice + self.system + self.irq + self.softirq + self.steal
    }
}

#[derive(Debug, Clone, Default)]
pub struct CpuState {
    pub aggregate: CpuRawSnapshot,
    pub per_core: Vec<CpuRawSnapshot>,
}

pub struct CpuCollector {
    prev_state: Option<CpuState>,
}

#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    pub total_percentage: f32,
    pub per_core_percentage: Vec<f32>,
    pub frequencies_khz: Vec<u64>,
}

impl CpuCollector {
    pub fn new() -> Self {
        Self { prev_state: None }
    }

    /// Read `/proc/stat` and return the raw state for aggregate and individual cores.
    pub fn read_stat() -> anyhow::Result<CpuState> {
        let file = File::open("/proc/stat").context("Failed to open /proc/stat")?;
        let reader = BufReader::new(file);

        let mut aggregate = CpuRawSnapshot::default();
        let mut per_core = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("cpu ") {
                aggregate = Self::parse_cpu_line(&line)?;
            } else if line.starts_with("cpu") {
                // Individual core e.g. cpu0, cpu1
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 && parts[0][3..].parse::<usize>().is_ok() {
                    let core_stat = Self::parse_cpu_line(&line)?;
                    per_core.push(core_stat);
                }
            }
        }

        Ok(CpuState { aggregate, per_core })
    }

    fn parse_cpu_line(line: &str) -> anyhow::Result<CpuRawSnapshot> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(anyhow!("Malformed cpu line in /proc/stat: {line}"));
        }

        let user = parts[1].parse().unwrap_or(0);
        let nice = parts[2].parse().unwrap_or(0);
        let system = parts[3].parse().unwrap_or(0);
        let idle = parts[4].parse().unwrap_or(0);
        let iowait = parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0);
        let irq = parts.get(6).and_then(|v| v.parse().ok()).unwrap_or(0);
        let softirq = parts.get(7).and_then(|v| v.parse().ok()).unwrap_or(0);
        let steal = parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0);

        Ok(CpuRawSnapshot {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
        })
    }

    /// Compute differential percentage between two snapshots with safety bounds against hotplug resets.
    pub fn calculate_utilization(prev: &CpuRawSnapshot, current: &CpuRawSnapshot) -> f32 {
        let prev_total = prev.total();
        let curr_total = current.total();
        if curr_total <= prev_total {
            return 0.0;
        }
        let total_delta = curr_total.saturating_sub(prev_total);

        let prev_busy = prev.busy();
        let curr_busy = current.busy();
        let busy_delta = curr_busy.saturating_sub(prev_busy);

        if total_delta == 0 {
            0.0
        } else {
            let pct = (busy_delta as f64 / total_delta as f64) * 100.0;
            (pct.clamp(0.0, 100.0)) as f32
        }
    }

    /// Read dynamic CPU frequencies from `/sys/devices/system/cpu/cpufreq/policy*` or `/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq`.
    pub fn read_frequencies() -> Vec<u64> {
        let mut freqs = Vec::new();
        let mut core_idx = 0;

        loop {
            let path_str = format!("/sys/devices/system/cpu/cpu{core_idx}/cpufreq/scaling_cur_freq");
            let path = Path::new(&path_str);
            if !path.exists() {
                break;
            }
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(freq) = content.trim().parse::<u64>() {
                    freqs.push(freq);
                }
            }
            core_idx += 1;
        }

        freqs
    }

    /// Collect differential metrics across successive calls.
    pub fn collect(&mut self) -> anyhow::Result<CpuMetrics> {
        let current = Self::read_stat()?;
        let freqs = Self::read_frequencies();

        let metrics = if let Some(prev) = &self.prev_state {
            let total_perc = Self::calculate_utilization(&prev.aggregate, &current.aggregate);
            let mut per_core_perc = Vec::with_capacity(current.per_core.len());

            for (i, curr_core) in current.per_core.iter().enumerate() {
                if let Some(prev_core) = prev.per_core.get(i) {
                    per_core_perc.push(Self::calculate_utilization(prev_core, curr_core));
                } else {
                    per_core_perc.push(0.0);
                }
            }

            CpuMetrics {
                total_percentage: total_perc,
                per_core_percentage: per_core_perc,
                frequencies_khz: freqs,
            }
        } else {
            // First read: establish baseline, return 0%
            let core_count = current.per_core.len();
            CpuMetrics {
                total_percentage: 0.0,
                per_core_percentage: vec![0.0; core_count],
                frequencies_khz: freqs,
            }
        };

        self.prev_state = Some(current);
        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_utilization_calculation() {
        let prev = CpuRawSnapshot {
            user: 100,
            nice: 0,
            system: 50,
            idle: 850,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        }; // total 1000, busy 150

        let curr = CpuRawSnapshot {
            user: 120, // +20
            nice: 0,
            system: 60, // +10
            idle: 920, // +70
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        }; // total 1100 (+100), busy 180 (+30) -> 30%

        let util = CpuCollector::calculate_utilization(&prev, &curr);
        assert!((util - 30.0).abs() < 0.1);
    }
}
