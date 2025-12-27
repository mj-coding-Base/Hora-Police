use anyhow::Result;
use chrono::Utc;
use nix::sys::signal;
use nix::unistd::Pid;
use tracing::{warn, info, error};
use crate::database::{IntelligenceDB, KillAction};
use crate::process_monitor::ProcessMonitor;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct KillEngine {
    db: IntelligenceDB,
    monitor: Arc<Mutex<ProcessMonitor>>,
    auto_kill: bool,
    threshold: f32,
}

impl KillEngine {
    pub fn new(db: IntelligenceDB, monitor: ProcessMonitor, auto_kill: bool, threshold: f32) -> Self {
        Self {
            db,
            monitor: Arc::new(Mutex::new(monitor)),
            auto_kill,
            threshold,
        }
    }

    pub async fn should_kill(&self, confidence: f32) -> bool {
        self.auto_kill && confidence >= self.threshold
    }

    pub async fn kill_process(
        &mut self,
        pid: i32,
        uid: u32,
        binary_path: &str,
        reason: &str,
        confidence: f32,
    ) -> Result<bool> {
        if !self.should_kill(confidence).await {
            return Ok(false);
        }

        info!("🔪 Killing process PID={}, binary={}, reason={}, confidence={:.2}", 
              pid, binary_path, reason, confidence);

        // Try graceful termination first (SIGTERM)
        let pid_obj = Pid::from_raw(pid);
        match signal::kill(pid_obj, signal::Signal::SIGTERM) {
            Ok(_) => {
                info!("✅ Sent SIGTERM to PID {}", pid);
                
                // Wait a bit and check if process still exists
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Check if process is still alive
                let monitor = self.monitor.lock().await;
                if monitor.get_process_by_pid(pid).is_some() {
                    warn!("⚠️  Process {} still alive after SIGTERM, sending SIGKILL", pid);
                    let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                }
            }
            Err(e) => {
                error!("❌ Failed to kill PID {}: {}", pid, e);
                return Err(anyhow::anyhow!("Failed to kill process: {}", e));
            }
        }

        // Record kill action
        let action = KillAction {
            id: 0,
            pid,
            uid,
            binary_path: binary_path.to_string(),
            reason: reason.to_string(),
            confidence,
            timestamp: Utc::now(),
        };

        self.db.record_kill_action(&action).await?;

        // Check for respawn
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let mut monitor = self.monitor.lock().await;
        monitor.refresh();
        
        if let Some(respawned) = monitor.get_process_by_pid(pid) {
            if respawned.binary_path == binary_path {
                warn!("🔄 Process respawned! PID={}, escalating...", pid);
                
                // Try to kill parent process
                if respawned.ppid > 0 {
                    let _ = self.kill_process(
                        respawned.ppid,
                        respawned.uid,
                        &format!("Parent of respawned process: {}", binary_path),
                        "Process respawn detected",
                        confidence + 0.1,
                    ).await;
                }
                
                return Ok(true);
            }
        }

        Ok(true)
    }

    pub fn is_system_process(&self, binary_path: &str) -> bool {
        // Never kill these system processes
        let system_binaries = [
            "/sbin/init",
            "/usr/sbin/sshd",
            "/usr/bin/systemd",
            "/lib/systemd/",
        ];

        system_binaries.iter().any(|&bin| binary_path.starts_with(bin))
    }
}

