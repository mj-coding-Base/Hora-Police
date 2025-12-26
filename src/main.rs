use anyhow::Result;
use sentinel_daemon::config::Config;
use sentinel_daemon::daemon::SentinelDaemon;
use std::path::PathBuf;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("sentinel_daemon=info,info")
        .init();

    info!("🚀 Sentinel Anti-Malware Daemon starting...");

    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/sentinel/config.toml"));

    let config = Config::load(&config_path)?;
    info!("✅ Configuration loaded from: {:?}", config_path);

    // Initialize and run daemon
    let mut daemon = SentinelDaemon::new(config).await?;
    
    info!("🛡️  Sentinel daemon initialized. Starting monitoring...");
    
    if let Err(e) = daemon.run().await {
        error!("❌ Daemon error: {}", e);
        return Err(e);
    }

    Ok(())
}

