use anyhow::{Context, Result};
use chrono::Utc;
use nix::sys::signal;
use nix::unistd::Pid;
use tracing::{info, warn, error};
use std::path::Path;

use crate::process_monitor::ProcessInfo;
use crate::database::{IntelligenceDB, KillAction};
use crate::pm2_integration::Pm2Integration;
use crate::systemd_integration::SystemdIntegration;
use crate::nginx_integration::NginxIntegration;
use crate::whitelist::WhitelistManager;
use crate::config::Config;

#[derive(Debug, Clone)]
pub enum KillActionType {
    Skip,  // Whitelisted or systemd/pm2 managed (low confidence)
    Notify,  // Send Telegram alert only
    StopUnit,  // systemctl stop
    StopPm2,  // pm2 stop
    KillDirect,  // Direct kill (unprivileged, high confidence)
}

pub struct SafeKillEngine {
    db: IntelligenceDB,
    pm2: Pm2Integration,
    systemd: SystemdIntegration,
    nginx: NginxIntegration,
    whitelist: WhitelistManager,
    config: SafeKillConfig,
}

#[derive(Debug, Clone)]
pub struct SafeKillConfig {
    pub auto_kill: bool,
    pub dry_run: bool,
    pub audit_only: bool,
    pub canary_mode: bool,
    pub threat_confidence_threshold: f32,
    pub high_confidence_threshold: f32,
}

impl SafeKillEngine {
    pub fn new(
        db: IntelligenceDB,
        pm2: Pm2Integration,
        systemd: SystemdIntegration,
        nginx: NginxIntegration,
        whitelist: WhitelistManager,
        config: SafeKillConfig,
    ) -> Self {
        Self {
            db,
            pm2,
            systemd,
            nginx,
            whitelist,
            config,
        }
    }

    /// Decide what action to take for a flagged process
    pub async fn decide_action(
        &mut self,
        process: &ProcessInfo,
        confidence: f32,
    ) -> KillActionType {
        // 1. Check whitelist
        if self.whitelist.is_whitelisted(process) {
            info!("Process PID {} is whitelisted, skipping", process.pid);
            return KillActionType::Skip;
        }

        // 2. Check if PM2-managed
        if self.pm2.is_pm2_managed(process.pid) {
            if let Some(app) = self.pm2.get_app_by_pid(process.pid) {
                if confidence >= self.config.high_confidence_threshold {
                    info!("PM2-managed process PID {} (app: {}) - will stop via PM2", 
                          process.pid, app.name);
                    return KillActionType::StopPm2;
                } else {
                    info!("PM2-managed process PID {} (app: {}) - confidence too low, notifying only", 
                          process.pid, app.name);
                    return KillActionType::Notify;
                }
            }
        }

        // 3. Check if systemd-managed
        if self.systemd.is_systemd_managed(process.pid) {
            if let Some(unit) = self.systemd.get_unit_by_pid(process.pid) {
                if confidence >= self.config.high_confidence_threshold {
                    info!("systemd-managed process PID {} (unit: {}) - will stop via systemctl", 
                          process.pid, unit.name);
                    return KillActionType::StopUnit;
                } else {
                    info!("systemd-managed process PID {} (unit: {}) - confidence too low, notifying only", 
                          process.pid, unit.name);
                    return KillActionType::Notify;
                }
            }
        }

        // 4. Check if Nginx upstream (high sensitivity - always notify first)
        if self.nginx.is_nginx_upstream(process.pid) {
            if let Some(upstream) = self.nginx.get_upstream_by_pid(process.pid) {
                warn!("Nginx upstream process PID {} (upstream: {}) - high sensitivity, notifying only", 
                      process.pid, upstream.name);
                return KillActionType::Notify;
            }
        }

        // 5. Check location: /tmp, /var/tmp, non-whitelisted home → allow direct kill
        let binary_path = Path::new(&process.binary_path);
        let is_suspicious_location = binary_path.starts_with("/tmp") ||
            binary_path.starts_with("/var/tmp") ||
            (binary_path.starts_with("/home") && 
             !self.is_whitelisted_home_directory(binary_path));

        if is_suspicious_location {
            if confidence >= self.config.threat_confidence_threshold {
                info!("Process PID {} in suspicious location - will kill directly", process.pid);
                return KillActionType::KillDirect;
            } else {
                return KillActionType::Notify;
            }
        }

        // 6. Default: Notify only (conservative approach)
        KillActionType::Notify
    }

    fn is_whitelisted_home_directory(&self, path: &Path) -> bool {
        // Check if path is in a whitelisted home directory
        // This is a simplified check - in production you might want more sophisticated logic
        if let Some(components) = path.components().next() {
            // Check against whitelist patterns
            let path_str = path.to_string_lossy();
            for entry in self.whitelist.get_entries() {
                if let Ok(regex) = regex::Regex::new(&entry.pattern) {
                    if regex.is_match(&path_str) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Execute the decided action
    pub async fn execute_action(
        &mut self,
        action: KillActionType,
        process: &ProcessInfo,
        reason: &str,
        confidence: f32,
    ) -> Result<bool> {
        if self.config.audit_only || self.config.dry_run {
            info!("[DRY RUN] Would execute action: {:?} for PID {} ({})", 
                  action, process.pid, reason);
            return Ok(false);
        }

        match action {
            KillActionType::Skip => {
                info!("Skipping action for PID {} (whitelisted)", process.pid);
                Ok(false)
            }
            KillActionType::Notify => {
                info!("Notifying about process PID {} ({})", process.pid, reason);
                // Notification will be handled by caller via Telegram
                Ok(false)
            }
            KillActionType::StopUnit => {
                if let Some(unit) = self.systemd.get_unit_by_pid(process.pid) {
                    info!("Stopping systemd unit: {} (PID: {})", unit.name, process.pid);
                    self.systemd.stop_unit(&unit.name).await?;
                    self.record_kill_action(process, reason, confidence).await?;
                    Ok(true)
                } else {
                    warn!("Unit not found for PID {}, falling back to direct kill", process.pid);
                    self.kill_direct(process, reason, confidence).await
                }
            }
            KillActionType::StopPm2 => {
                if let Some(app) = self.pm2.get_app_by_pid(process.pid) {
                    info!("Stopping PM2 app: {} (PID: {})", app.name, process.pid);
                    self.pm2.stop_app(&app.name, &app.user).await?;
                    self.record_kill_action(process, reason, confidence).await?;
                    Ok(true)
                } else {
                    warn!("PM2 app not found for PID {}, falling back to direct kill", process.pid);
                    self.kill_direct(process, reason, confidence).await
                }
            }
            KillActionType::KillDirect => {
                self.kill_direct(process, reason, confidence).await
            }
        }
    }

    async fn kill_direct(
        &self,
        process: &ProcessInfo,
        reason: &str,
        confidence: f32,
    ) -> Result<bool> {
        if !self.config.auto_kill {
            info!("Auto-kill disabled, would kill PID {} ({})", process.pid, reason);
            return Ok(false);
        }

        info!("Killing process PID={}, binary={}, reason={}, confidence={:.2}", 
              process.pid, process.binary_path, reason, confidence);

        // Try graceful termination first (SIGTERM)
        let pid_obj = Pid::from_raw(process.pid);
        match signal::kill(pid_obj, signal::Signal::SIGTERM) {
            Ok(_) => {
                info!("Sent SIGTERM to PID {}", process.pid);
                
                // Wait a bit and check if process still exists
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Check if process is still alive (simplified - would need process monitor)
                // For now, always try SIGKILL after SIGTERM
                warn!("Sending SIGKILL to PID {} (force kill)", process.pid);
                let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
            }
            Err(e) => {
                error!("Failed to kill PID {}: {}", process.pid, e);
                return Err(anyhow::anyhow!("Failed to kill process: {}", e));
            }
        }

        // Record kill action
        self.record_kill_action(process, reason, confidence).await?;

        Ok(true)
    }

    async fn record_kill_action(
        &self,
        process: &ProcessInfo,
        reason: &str,
        confidence: f32,
    ) -> Result<()> {
        let action = KillAction {
            id: 0,
            pid: process.pid,
            uid: process.uid,
            binary_path: process.binary_path.clone(),
            reason: reason.to_string(),
            confidence,
            timestamp: Utc::now(),
        };

        self.db.record_kill_action(&action).await?;
        Ok(())
    }

    pub fn should_kill(&self, confidence: f32) -> bool {
        self.config.auto_kill && 
        !self.config.audit_only && 
        !self.config.dry_run &&
        confidence >= self.config.threat_confidence_threshold
    }
}

impl From<&Config> for SafeKillConfig {
    fn from(config: &Config) -> Self {
        Self {
            auto_kill: config.auto_kill,
            dry_run: config.dry_run,
            audit_only: config.audit_only,
            canary_mode: config.canary_mode,
            threat_confidence_threshold: config.threat_confidence_threshold,
            high_confidence_threshold: config.high_confidence_threshold,
        }
    }
}

