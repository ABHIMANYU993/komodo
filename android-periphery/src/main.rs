pub mod auth;
pub mod config;
pub mod protocol;
pub mod transport;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version = config::AGENT_VERSION, about = "Komodo Android Periphery Daemon")]
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

    tracing_subscriber::fmt::init();

    info!(
        agent_version = config::AGENT_VERSION,
        protocol_version = config::PROTOCOL_COMPATIBILITY_VERSION,
        upstream_commit = config::UPSTREAM_COMPATIBILITY_COMMIT,
        "Starting Komodo Android Periphery"
    );

    Ok(())
}
