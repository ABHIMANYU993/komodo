pub mod battery;
pub mod cpu;
pub mod gpu;
pub mod mem;
pub mod net;
pub mod proc;
pub mod storage;
pub mod thermal;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

use crate::protocol::types::{
    SingleNetworkInterfaceUsage, SystemLoadAverage, SystemProcess, SystemStats,
};
use battery::BatteryCollector;
use cpu::CpuCollector;
use gpu::GpuCollector;
use mem::MemoryCollector;
use net::NetworkCollector;
use proc::ProcessCollector;
use storage::StorageCollector;
use thermal::ThermalCollector;

#[derive(Debug, Clone, Default)]
pub struct TelemetrySnapshot {
    pub stats: SystemStats,
    pub processes: Vec<SystemProcess>,
}

pub struct TelemetryEngine {
    cache: Arc<RwLock<TelemetrySnapshot>>,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        let mut initial = TelemetrySnapshot::default();
        initial.stats.disks = StorageCollector::collect();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        initial.stats.refresh_list_ts = now_ms;
        initial.stats.refresh_ts = now_ms;
        Self {
            cache: Arc::new(RwLock::new(initial)),
        }
    }

    pub fn snapshot(&self) -> Arc<RwLock<TelemetrySnapshot>> {
        self.cache.clone()
    }

    /// Spawns the multi-rate background sampling tasks.
    pub fn start(&self) {
        let cache = self.cache.clone();

        // 1. FAST TIER (~1 second): CPU, Memory, Network
        tokio::spawn(async move {
            let mut cpu_collector = CpuCollector::new();
            let mut net_collector = NetworkCollector::new();

            loop {
                // Read CPU
                let cpu_metrics = cpu_collector.collect().unwrap_or_default();

                // Read Memory
                let mem_snap = MemoryCollector::collect().unwrap_or_default();

                // Read Network
                let net_rates = net_collector.collect().unwrap_or_default();
                let network_usages: Vec<SingleNetworkInterfaceUsage> = net_rates
                    .into_iter()
                    .map(|r| SingleNetworkInterfaceUsage {
                        name: r.name,
                        ingress_bytes: r.rx_bytes_per_sec,
                        egress_bytes: r.tx_bytes_per_sec,
                    })
                    .collect();

                // Read Load Average
                let load_avg = Self::read_load_average();

                // Update cache
                {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let total_ingress: f64 = network_usages.iter().map(|u| u.ingress_bytes).sum();
                    let total_egress: f64 = network_usages.iter().map(|u| u.egress_bytes).sum();

                    let mut lock = cache.write().await;
                    lock.stats.cpu_perc = cpu_metrics.total_percentage;
                    lock.stats.mem_used_gb = mem_snap.used_gb();
                    lock.stats.mem_total_gb = mem_snap.total_gb();
                    lock.stats.mem_free_gb = mem_snap.free_gb();
                    lock.stats.mem_buff_cache_gb = mem_snap.buff_cache_gb();
                    lock.stats.swap_total_gb = mem_snap.swap_total_gb();
                    lock.stats.swap_used_gb = mem_snap.swap_used_gb();
                    lock.stats.network_ingress_bytes = total_ingress;
                    lock.stats.network_egress_bytes = total_egress;
                    lock.stats.load_average = load_avg;
                    lock.stats.polling_rate = "5-sec".to_string();
                    lock.stats.refresh_ts = now_ms;
                }

                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        });

        // 2. MEDIUM TIER (~3 seconds): Process list, Battery
        let cache_med = self.cache.clone();
        tokio::spawn(async move {
            let mut proc_collector = ProcessCollector::new();
            let battery_collector = BatteryCollector::new();

            loop {
                let procs = proc_collector.collect();
                let _batt = battery_collector.collect();

                {
                    let mut lock = cache_med.write().await;
                    lock.processes = procs;
                }

                tokio::time::sleep(Duration::from_millis(3000)).await;
            }
        });

        // 3. SLOW TIER (~30 seconds): Disks, Thermal, Static Hardware
        let cache_slow = self.cache.clone();
        tokio::spawn(async move {
            let thermal_collector = ThermalCollector::new();
            let gpu_collector = GpuCollector::new();

            loop {
                let disks = StorageCollector::collect();
                let _thermal = thermal_collector.collect();
                let _gpu = gpu_collector.collect();

                {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let mut lock = cache_slow.write().await;
                    lock.stats.disks = disks;
                    lock.stats.refresh_list_ts = now_ms;
                }

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        info!("Multi-rate telemetry engine started (FAST=1s, MEDIUM=3s, SLOW=30s)");
    }

    /// Read `/proc/loadavg` for 1m, 5m, 15m load averages.
    fn read_load_average() -> SystemLoadAverage {
        let Ok(content) = std::fs::read_to_string("/proc/loadavg") else {
            return SystemLoadAverage::default();
        };
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            SystemLoadAverage {
                one: parts[0].parse().unwrap_or(0.0),
                five: parts[1].parse().unwrap_or(0.0),
                fifteen: parts[2].parse().unwrap_or(0.0),
            }
        } else {
            SystemLoadAverage::default()
        }
    }
}
