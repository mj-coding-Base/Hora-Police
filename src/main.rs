use anyhow::Result;
use hora_police::config::Config;
use hora_police::daemon::SentinelDaemon;
use std::path::PathBuf;
use tracing::{error, info};
use clap::Parser;
use sd_notify::NotifyState;
use tracing::warn;

#[derive(Parser)]
#[command(name = "hora-police")]
#[command(about = "Hora-Police Anti-Malware Daemon")]
struct Args {
    /// Configuration file path
    #[arg(long, default_value = "/etc/hora-police/config.toml")]
    config: PathBuf,
    
    /// Enable dry-run mode (no destructive actions)
    #[arg(long)]
    dry_run: bool,
    
    /// Enable canary mode (limited enforcement)
    #[arg(long)]
    canary: bool,
    
    /// Start telemetry probe endpoint
    #[arg(long)]
    probe: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("hora_police=info,info")
        .init();

    let args = Args::parse();

    info!("🚀 Hora-Police Anti-Malware Daemon starting...");

    // Load configuration
    let mut config = Config::load(&args.config)?;
    
    // Override config with CLI flags
    if args.dry_run {
        config.dry_run = true;
        info!("🔍 Dry-run mode enabled via CLI");
    }
    if args.canary {
        config.canary_mode = true;
        info!("🪶 Canary mode enabled via CLI");
    }
    
    info!("✅ Configuration loaded from: {:?}", args.config);

    // Start probe endpoint if requested
    if args.probe {
        tokio::spawn(async move {
            start_probe_endpoint().await;
        });
    }

    // Initialize and run daemon
    let mut daemon = SentinelDaemon::new(config).await?;
    
    info!("🛡️  Hora-Police daemon initialized. Starting monitoring...");
    
    // Notify systemd that we're ready
    if let Err(e) = sd_notify::notify(false, &[NotifyState::Ready]) {
        warn!("Failed to notify systemd of ready state: {}", e);
    }
    
    if let Err(e) = daemon.run().await {
        error!("❌ Daemon error: {}", e);
        // Notify systemd of failure
        let _ = sd_notify::notify(false, &[NotifyState::Status("Daemon error occurred")]);
        return Err(e);
    }

    Ok(())
}

async fn start_probe_endpoint() {
    use tokio::net::TcpListener;
    use std::io::Write;
    
    let addr = "127.0.0.1:9999";
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind probe endpoint: {}", e);
            return;
        }
    };

    info!("📊 Telemetry probe endpoint started on http://{}", addr);

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    // Simple HTTP response
                    let summary = serde_json::json!({
                        "status": "running",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "version": "0.1.0",
                    });

                    let json = serde_json::to_string_pretty(&summary).unwrap();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(),
                        json
                    );

                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
            Err(e) => {
                error!("Probe endpoint error: {}", e);
            }
        }
    }
}

