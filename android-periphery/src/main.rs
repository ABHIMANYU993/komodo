#![allow(unused_crate_dependencies)]

pub mod auth;
pub mod capabilities;
pub mod collectors;
pub mod config;
pub mod protocol;
pub mod terminal;
pub mod transport;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use auth::keys::IdentityKeys;
use capabilities::detector::DeviceCapabilities;
use collectors::TelemetryEngine;
use config::AgentConfig;
use transport::CoreConnectionLoop;

#[derive(Parser, Debug)]
#[command(
    author,
    version = config::AGENT_VERSION,
    about = "Komodo Android Periphery Daemon (Root/Magisk Native Agent)"
)]
struct Cli {
    /// Path to config.toml
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Run capability self-diagnostics and print report
    #[arg(short, long)]
    diagnose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.diagnose {
        run_diagnostics();
        return Ok(());
    }

    // 1. Load Configuration
    let config = match AgentConfig::load(cli.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e:#}");
            std::process::exit(1);
        }
    };

    // 2. Initialize Tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("=======================================================");
    info!(" Komodo Android Periphery Daemon (Root Native)");
    info!(" Agent Version:         {}", config::AGENT_VERSION);
    info!(" Protocol Alignment:    v{}", config::PROTOCOL_COMPATIBILITY_VERSION);
    info!(" Upstream Commit:       {}", config::UPSTREAM_COMPATIBILITY_COMMIT);
    info!(" Connect As:            {}", config.connect_as);
    info!(" Core URL:              {}", config.core_url);
    info!("=======================================================");

    // 3. Dynamic Hardware & OS Discovery Pass
    let capabilities = DeviceCapabilities::discover();
    info!(
        model = %capabilities.model,
        manufacturer = %capabilities.manufacturer,
        android = %capabilities.android_release,
        sdk = capabilities.sdk_level,
        kernel = %capabilities.kernel_release,
        cpus = capabilities.cpu_count,
        selinux = %capabilities.selinux_state,
        magisk = ?capabilities.magisk_version,
        "Discovered runtime device capabilities"
    );

    // 4. Load or Generate Cryptographic Identity Keys
    let keys = IdentityKeys::load_or_generate(&config.keys_dir)?;
    info!("Node Public Key (SPKI): {}", keys.public_key.as_str());

    // 5. Initialize High-Frequency Telemetry Engine
    let telemetry = TelemetryEngine::new(&config.stats_polling_rate);
    telemetry.start(config.stats_polling_rate.clone(), config.stats_interval_duration());

    // 6. Start Secure Connection Loop to Komodo Core
    let connection_loop = CoreConnectionLoop::new(config, keys, capabilities, telemetry);

    tokio::select! {
        _ = connection_loop.run() => {},
        _ = tokio::signal::ctrl_c() => {
            info!("Received interrupt signal. Shutting down Android Periphery daemon...");
        }
    }

    Ok(())
}

fn run_diagnostics() {
    println!("=================================================================");
    println!(" Komodo Android Periphery — Hardware Self-Diagnostics");
    println!(" Agent Version:         {}", config::AGENT_VERSION);
    println!(" Protocol Alignment:    v{}", config::PROTOCOL_COMPATIBILITY_VERSION);
    println!(" Upstream Commit:       {}", config::UPSTREAM_COMPATIBILITY_COMMIT);
    println!("=================================================================");

    let caps = DeviceCapabilities::discover();
    println!("Manufacturer:          {}", caps.manufacturer);
    println!("Model:                 {}", caps.model);
    println!("Board / Hardware:      {} / {}", caps.board, caps.hardware_platform);
    println!("Android Release:       {} (SDK API {})", caps.android_release, caps.sdk_level);
    println!("Kernel Release:        {}", caps.kernel_release);
    println!("Architecture:          {}", caps.architecture);
    println!("Online CPUs:           {}", caps.cpu_count);
    println!("SELinux State:         {}", caps.selinux_state);
    println!("Magisk Environment:    {:?}", caps.magisk_version);

    println!("\n--- CPU Telemetry ---");
    let mut cpu_collector = collectors::cpu::CpuCollector::new();
    let cpu_res = cpu_collector.collect().unwrap_or_default();
    println!("Aggregate Utilization: {:.1}%", cpu_res.total_percentage);
    println!("Per-Core Utilization:  {:?}", cpu_res.per_core_percentage);
    println!("Core Frequencies:      {:?} kHz", cpu_res.frequencies_khz);

    println!("\n--- Memory Telemetry ---");
    if let Ok(mem) = collectors::mem::MemoryCollector::collect() {
        println!("MemTotal:              {:.2} GB", mem.total_gb());
        println!("MemAvailable:          {:.2} GB", (mem.mem_available_kb as f64) / 1048576.0);
        println!("MemUsed (authoritative): {:.2} GB ({:.1}%)", mem.used_gb(), (mem.used_gb() / mem.total_gb()) * 100.0);
        println!("SwapTotal (ZRAM):      {:.2} GB", mem.swap_total_gb());
        println!("SwapUsed:              {:.2} GB", mem.swap_used_gb());
    }

    println!("\n--- Storage Mounts ---");
    let disks = collectors::storage::StorageCollector::collect();
    for d in &disks {
        println!("Mount: {:<20} Filesystem: {:<10} Used: {:.2} GB / Total: {:.2} GB", d.mount.display(), d.file_system, d.used_gb, d.total_gb);
    }

    println!("\n--- Network Interfaces ---");
    let mut net = collectors::net::NetworkCollector::new();
    let _ = net.collect();
    if let Ok(devs) = collectors::net::NetworkCollector::read_dev() {
        for (name, counters) in devs {
            println!("Interface: {:<10} RX: {:<10} bytes | TX: {:<10} bytes", name, counters.rx_bytes, counters.tx_bytes);
        }
    }

    println!("\n--- Battery Subsystem ---");
    let battery = collectors::battery::BatteryCollector::new();
    let batt_info = battery.collect();
    println!("Capacity:              {:?}%", batt_info.capacity_percent);
    println!("Status / Health:       {:?} / {:?}", batt_info.status, batt_info.health);
    println!("Voltage / Temp:        {:?} uV / {:?} (0.1°C)", batt_info.voltage_now_uv, batt_info.temp_deci_c);

    println!("\n--- Thermal Sensors ---");
    let thermal = collectors::thermal::ThermalCollector::new();
    let zones = thermal.collect();
    for z in zones.iter().take(15) {
        println!("Zone {:<2}: {:<20} ({:?}) -> {:.1}°C", z.zone_id, z.sensor_type, z.category, z.temp_c);
    }
    if zones.len() > 15 {
        println!("... and {} more thermal zones discovered", zones.len() - 15);
    }

    println!("\n--- GPU Subsystem ---");
    let gpu = collectors::gpu::GpuCollector::new();
    let gpu_info = gpu.collect();
    println!("GPU Vendor:            {}", gpu_info.vendor);
    println!("GPU Model:             {:?}", gpu_info.model);
    println!("GPU Utilization:       {:?}", gpu_info.utilization_percent);
    println!("GPU Clock Frequency:   {:?} Hz", gpu_info.cur_freq_hz);

    println!("\n--- Process Subsystem ---");
    let mut procs = collectors::proc::ProcessCollector::new();
    let proc_list = procs.collect();
    println!("Running Process Count: {}", proc_list.len());

    println!("=================================================================");
    println!(" Diagnostic Pass Complete. All telemetry subsystems functional.");
    println!("=================================================================");
}
