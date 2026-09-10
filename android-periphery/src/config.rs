use std::path::{Path, PathBuf};
use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_COMPATIBILITY_VERSION: &str = "2.4.1";
pub const UPSTREAM_COMPATIBILITY_COMMIT: &str = "780ac68b992094a9fccd5fffb760e0c84fd3c3d1";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentConfig {
    /// URL to Komodo Core, e.g. "ws://192.168.1.50:8120" or "wss://komodo.example.com"
    pub core_url: String,

    /// Node identifier name as recognized by Core (e.g. "Android_Redmi_Note_10_Pro")
    pub connect_as: String,

    /// Optional onboarding key for first-time automated server registration
    pub onboarding_key: Option<String>,

    /// Allowed Core public keys (if empty, accepts any Core public key)
    #[serde(default)]
    pub core_public_keys: Vec<String>,

    /// Optional path to store persistent cryptographic identity keys
    #[serde(default = "default_keys_dir")]
    pub keys_dir: PathBuf,

    /// Log level: "trace", "debug", "info", "warn", "error"
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Reconnect interval in seconds (default: 5)
    #[serde(default = "default_reconnect_secs")]
    pub reconnect_seconds: u64,

    /// Skip TLS certificate verification (for self-signed certs)
    #[serde(default)]
    pub tls_insecure_skip_verify: bool,

    /// Stats polling rate: e.g. "1-sec", "2-sec", "5-sec" (default: "1-sec")
    #[serde(default = "default_stats_polling_rate")]
    pub stats_polling_rate: String,
}

fn default_keys_dir() -> PathBuf {
    if Path::new("/data/adb").exists() {
        PathBuf::from("/data/adb/komodo/keys")
    } else {
        PathBuf::from("./keys")
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_reconnect_secs() -> u64 {
    5
}

fn default_stats_polling_rate() -> String {
    "1-sec".to_string()
}

impl AgentConfig {
    /// Returns the polling interval as a Duration
    pub fn stats_interval_duration(&self) -> std::time::Duration {
        let rate = self.stats_polling_rate.trim();
        if rate.ends_with("-sec") {
            let secs: u64 = rate.trim_end_matches("-sec").parse().unwrap_or(1);
            std::time::Duration::from_secs(secs.max(1))
        } else if let Ok(secs) = rate.parse::<u64>() {
            std::time::Duration::from_secs(secs.max(1))
        } else {
            std::time::Duration::from_secs(1)
        }
    }
    /// Load configuration from a specified path, or search standard locations:
    /// 1. Specified `--config <path>`
    /// 2. `/data/adb/komodo/config.toml`
    /// 3. `./config.toml`
    pub fn load(explicit_path: Option<&Path>) -> anyhow::Result<Self> {
        let path = match explicit_path {
            Some(p) => p.to_path_buf(),
            None => {
                let magisk_path = Path::new("/data/adb/komodo/config.toml");
                if magisk_path.exists() {
                    magisk_path.to_path_buf()
                } else {
                    PathBuf::from("config.toml")
                }
            }
        };

        if !path.exists() {
            return Err(anyhow!(
                "Configuration file not found at {}. Please create one from config.toml.example.",
                path.display()
            ));
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read configuration file at {}", path.display()))?;

        let config: AgentConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse configuration TOML at {}", path.display()))?;

        Ok(config)
    }
}
