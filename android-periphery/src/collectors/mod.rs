pub mod battery;
pub mod cpu;
pub mod gpu;
pub mod ip;
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

#[derive(Clone)]
pub struct TelemetrySnapshot {
    pub stats: SystemStats,
    pub public_ip: Option<String>,
    pub proc_collector: Arc<tokio::sync::Mutex<(ProcessCollector, Option<std::time::Instant>, Vec<SystemProcess>)>>,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            stats: SystemStats::default(),
            public_ip: None,
            proc_collector: Arc::new(tokio::sync::Mutex::new((
                ProcessCollector::new(),
                None,
                Vec::new(),
            ))),
        }
    }
}

impl std::fmt::Debug for TelemetrySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetrySnapshot")
            .field("stats", &self.stats)
            .field("public_ip", &self.public_ip)
            .finish()
    }
}

impl TelemetrySnapshot {
    /// On-demand process collection with 800ms cache.
    /// This avoids scanning /proc on Android during normal idle telemetry while supporting dynamic 1s UI polling.
    pub async fn get_processes(&self) -> Vec<SystemProcess> {
        let mut lock = self.proc_collector.lock().await;
        let now = std::time::Instant::now();
        if let Some(last) = lock.1 {
            if now.duration_since(last).as_millis() < 800 {
                return lock.2.clone();
            }
        }
        let procs = lock.0.collect();
        lock.1 = Some(now);
        lock.2 = procs.clone();
        procs
    }
}

pub struct TelemetryEngine {
    cache: Arc<RwLock<TelemetrySnapshot>>,
}

impl TelemetryEngine {
    pub fn new(polling_rate: &str) -> Self {
        let mut initial = TelemetrySnapshot::default();
        initial.stats.disks = StorageCollector::collect();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        initial.stats.refresh_list_ts = now_ms;
        initial.stats.refresh_ts = now_ms;
        initial.stats.polling_rate = polling_rate.to_string();
        Self {
            cache: Arc::new(RwLock::new(initial)),
        }
    }

    pub fn snapshot(&self) -> Arc<RwLock<TelemetrySnapshot>> {
        self.cache.clone()
    }

    /// Spawns the background sampling tasks configured with the active polling rate.
    pub fn start(&self, polling_rate_str: String, polling_duration: Duration) {
        let cache = self.cache.clone();
        let rate_str = polling_rate_str.clone();

        // 1. PRIMARY SYSTEM STATS LOOP (CPU, Memory, Disks, Network, Load Average)
        tokio::spawn(async move {
            let mut cpu_collector = CpuCollector::new();
            let mut net_collector = NetworkCollector::new();
            let mut last_disk_check = std::time::Instant::now();
            let mut cached_disks = StorageCollector::collect();

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

                // Refresh Disks / Storage only every 30 seconds to avoid statvfs overhead
                if last_disk_check.elapsed().as_secs() >= 30 {
                    cached_disks = StorageCollector::collect();
                    last_disk_check = std::time::Instant::now();
                }

                // Update cache with unified timestamps
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
                    lock.stats.disks = cached_disks.clone();
                    lock.stats.polling_rate = rate_str.clone();
                    lock.stats.refresh_ts = now_ms;
                    lock.stats.refresh_list_ts = now_ms;
                }

                tokio::time::sleep(polling_duration).await;
            }
        });

        // 2. HARDWARE SENSORS LOOP (conservative 10s sampling for battery/thermals)
        tokio::spawn(async move {
            let battery_collector = BatteryCollector::new();
            let thermal_collector = ThermalCollector::new();
            let gpu_collector = GpuCollector::new();

            loop {
                let _batt = battery_collector.collect();
                let _thermal = thermal_collector.collect();
                let _gpu = gpu_collector.collect();

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        // 3. BACKGROUND IP RESOLUTION (non-blocking, cached)
        let cache_ip = self.cache.clone();
        tokio::spawn(async move {
            loop {
                if let Some(resolved_ip) = ip::IpResolver::resolve().await {
                    let mut lock = cache_ip.write().await;
                    lock.public_ip = Some(resolved_ip);
                }
                // Re-check IP periodically every 15 minutes
                tokio::time::sleep(Duration::from_secs(900)).await;
            }
        });

        info!(
            "High-frequency telemetry engine started (polling_rate={}, interval={:?})",
            polling_rate_str, polling_duration
        );
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
