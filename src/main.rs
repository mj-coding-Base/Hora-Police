use anyhow::Result;
use hora_police::config::Config;
use hora_police::daemon::SentinelDaemon;
use std::path::PathBuf;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("hora_police=info,info")
        .init();

    info!("🚀 Hora-Police Anti-Malware Daemon starting...");

    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/hora-police/config.toml"));

    let config = Config::load(&config_path)?;
    info!("✅ Configuration loaded from: {:?}", config_path);

    // Initialize and run daemon
    let mut daemon = SentinelDaemon::new(config).await?;
    
    info!("🛡️  Hora-Police daemon initialized. Starting monitoring...");
    
    if let Err(e) = daemon.run().await {
        error!("❌ Daemon error: {}", e);
        return Err(e);
    }

    Ok(())
}

