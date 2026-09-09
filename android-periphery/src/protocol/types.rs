use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemInformation {
    pub name: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub core_count: Option<u32>,
    pub logical_core_count: Option<u32>,
    pub host_name: Option<String>,
    #[serde(default)]
    pub cpu_brand: String,
    #[serde(default)]
    pub cpu_arch: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct SystemLoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SingleDiskUsage {
    pub mount: PathBuf,
    pub file_system: String,
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SingleNetworkInterfaceUsage {
    pub name: String,
    pub ingress_bytes: f64,
    pub egress_bytes: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemStats {
    pub cpu_perc: f32,
    #[serde(default)]
    pub load_average: SystemLoadAverage,
    #[serde(default)]
    pub mem_free_gb: f64,
    pub mem_used_gb: f64,
    pub mem_total_gb: f64,
    #[serde(default)]
    pub mem_buff_cache_gb: f64,
    #[serde(default)]
    pub mem_zfs_arc_gb: f64,
    #[serde(default)]
    pub swap_total_gb: f64,
    #[serde(default)]
    pub swap_used_gb: f64,
    pub disks: Vec<SingleDiskUsage>,
    #[serde(default)]
    pub networks: Vec<SingleNetworkInterfaceUsage>,
    #[serde(default)]
    pub cpus: Vec<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemProcess {
    pub pid: u32,
    pub name: String,
    #[serde(default)]
    pub exe: String,
    pub cmd: Vec<String>,
    #[serde(default)]
    pub start_time: f64,
    pub cpu_perc: f32,
    pub mem_mb: f64,
    pub disk_read_kb: f64,
    pub disk_write_kb: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeripheryInformation {
    pub version: String,
    pub public_key: String,
    pub terminals_disabled: bool,
    pub container_terminals_disabled: bool,
    pub stats_polling_rate: String,
    pub docker_connected: bool,
    pub public_ip: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PollStatus {
    pub include_stats: bool,
    pub include_docker: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PollStatusResponse {
    pub periphery_info: PeripheryInformation,
    pub system_info: SystemInformation,
    pub system_stats: Option<SystemStats>,
    pub docker: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetHealthResponse {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetVersionResponse {
    pub version: String,
}
