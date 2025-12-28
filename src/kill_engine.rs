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
                let still_alive = {
                    let monitor = self.monitor.lock().await;
                    monitor.get_process_by_pid(pid).is_some()
                };
                if still_alive {
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

        // Check for respawn with improved detection
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let respawned_info = {
            let mut monitor = self.monitor.lock().await;
            monitor.refresh();
            
            // Check if same binary path exists with different PID
            let all_processes = monitor.get_all_processes().unwrap_or_default();
            all_processes.into_iter()
                .find(|p| p.binary_path == binary_path && p.pid != pid)
        };
        
        if let Some(respawned) = respawned_info {
            // Enhanced respawn detection: check parent PID and binary path
            if respawned.ppid > 0 {
                warn!("⚠️  Process respawned! New PID={}, Old PID={}, Parent PID={} should be investigated. Binary: {}", 
                      respawned.pid, pid, respawned.ppid, binary_path);
                
                // Check if parent is suspicious (not init/systemd)
                if respawned.ppid != 1 && !self.is_system_process(&respawned.binary_path) {
                    warn!("🚨 Suspicious parent process detected! Parent PID={} may be malware controller", 
                          respawned.ppid);
                }
            } else {
                warn!("🔄 Process respawned! New PID={}, Old PID={}, Binary: {}", 
                      respawned.pid, pid, binary_path);
            }
            
            return Ok(true);
        }

        Ok(true)
    }

    /// Kill an entire process tree (parent + all children) recursively
    pub async fn kill_process_tree(&mut self, root_pid: i32) -> Result<Vec<i32>> {
        let monitor = self.monitor.lock().await;
        let child_pids = monitor.get_full_process_tree(root_pid);
        drop(monitor);

        let mut killed_pids = Vec::new();
        
        // Kill children first (bottom-up), then parent
        // This prevents orphaned processes
        let mut pids_to_kill = child_pids.clone();
        pids_to_kill.reverse(); // Kill deepest children first
        
        for pid in pids_to_kill {
            if pid == root_pid {
                continue; // Kill parent last
            }
            
            let pid_obj = Pid::from_raw(pid);
            if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
                killed_pids.push(pid);
                info!("✅ Sent SIGTERM to child process PID {}", pid);
            }
        }
        
        // Wait a bit for children to terminate
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Force kill any remaining children
        let monitor = self.monitor.lock().await;
        for pid in &child_pids {
            if pid == &root_pid {
                continue;
            }
            if monitor.get_process_by_pid(*pid).is_some() {
                let pid_obj = Pid::from_raw(*pid);
                let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                warn!("⚠️  Force killed child PID {}", pid);
            }
        }
        drop(monitor);
        
        // Now kill the parent
        let pid_obj = Pid::from_raw(root_pid);
        if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
            killed_pids.push(root_pid);
            info!("✅ Sent SIGTERM to root process PID {}", root_pid);
            
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Check if parent is still alive
            let monitor = self.monitor.lock().await;
            if monitor.get_process_by_pid(root_pid).is_some() {
                let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                warn!("⚠️  Force killed root process PID {}", root_pid);
            }
        }
        
        Ok(killed_pids)
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

