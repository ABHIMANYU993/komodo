use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::Context;

#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub mem_free_kb: u64,
    pub cached_kb: u64,
    pub sreclaimable_kb: u64,
    pub buffers_kb: u64,
    pub shmem_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemorySnapshot {
    /// In Android systems, `MemFree` only accounts for unallocated pages, falsely reporting
    /// 95-98% RAM usage if used as `MemTotal - MemFree`.
    ///
    /// The authoritative Android memory utilization metric is:
    /// `used_kb = mem_total_kb - mem_available_kb`
    pub fn used_kb(&self) -> u64 {
        self.mem_total_kb.saturating_sub(self.mem_available_kb)
    }

    pub fn total_gb(&self) -> f64 {
        self.mem_total_kb as f64 / (1024.0 * 1024.0)
    }

    pub fn used_gb(&self) -> f64 {
        self.used_kb() as f64 / (1024.0 * 1024.0)
    }

    pub fn free_gb(&self) -> f64 {
        self.mem_free_kb as f64 / (1024.0 * 1024.0)
    }

    pub fn buff_cache_gb(&self) -> f64 {
        (self.buffers_kb + self.cached_kb + self.sreclaimable_kb) as f64 / (1024.0 * 1024.0)
    }

    pub fn swap_total_gb(&self) -> f64 {
        self.swap_total_kb as f64 / (1024.0 * 1024.0)
    }

    pub fn swap_used_gb(&self) -> f64 {
        self.swap_total_kb.saturating_sub(self.swap_free_kb) as f64 / (1024.0 * 1024.0)
    }
}

pub struct MemoryCollector;

impl MemoryCollector {
    /// Collect current memory statistics by reading `/proc/meminfo`.
    pub fn collect() -> anyhow::Result<MemorySnapshot> {
        let file = File::open("/proc/meminfo").context("Failed to open /proc/meminfo")?;
        let reader = BufReader::new(file);

        let mut snapshot = MemorySnapshot::default();

        for line in reader.lines() {
            let line = line?;
            let mut parts = line.split_whitespace();
            let key = match parts.next() {
                Some(k) => k,
                None => continue,
            };
            let val = match parts.next().and_then(|v| v.parse::<u64>().ok()) {
                Some(v) => v,
                None => continue,
            };

            match key {
                "MemTotal:" => snapshot.mem_total_kb = val,
                "MemAvailable:" => snapshot.mem_available_kb = val,
                "MemFree:" => snapshot.mem_free_kb = val,
                "Cached:" => snapshot.cached_kb = val,
                "SReclaimable:" => snapshot.sreclaimable_kb = val,
                "Buffers:" => snapshot.buffers_kb = val,
                "Shmem:" => snapshot.shmem_kb = val,
                "SwapTotal:" => snapshot.swap_total_kb = val,
                "SwapFree:" => snapshot.swap_free_kb = val,
                _ => {}
            }
        }

        // If MemAvailable was missing (very old kernels < 3.14), fallback to Free + Buffers + Cached
        if snapshot.mem_available_kb == 0 && snapshot.mem_total_kb > 0 {
            snapshot.mem_available_kb = snapshot.mem_free_kb + snapshot.buffers_kb + snapshot.cached_kb;
        }

        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_accounting() {
        let snap = MemorySnapshot {
            mem_total_kb: 6 * 1024 * 1024,     // 6 GB
            mem_available_kb: 3 * 1024 * 1024, // 3 GB available
            mem_free_kb: 200 * 1024,           // 200 MB free
            cached_kb: 2 * 1024 * 1024,
            sreclaimable_kb: 400 * 1024,
            buffers_kb: 100 * 1024,
            shmem_kb: 50 * 1024,
            swap_total_kb: 2 * 1024 * 1024,    // 2 GB swap (ZRAM)
            swap_free_kb: 1 * 1024 * 1024,     // 1 GB swap free
        };

        // Used must be Total - Available = 3 GB (50%), NOT Total - Free = 5.8 GB (97%)!
        assert_eq!(snap.used_kb(), 3 * 1024 * 1024);
        assert!((snap.used_gb() - 3.0).abs() < 0.01);
        assert!((snap.total_gb() - 6.0).abs() < 0.01);
        assert!((snap.swap_used_gb() - 1.0).abs() < 0.01);
    }
}
