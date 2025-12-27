use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SystemdUnit {
    pub name: String,
    pub pid: Option<i32>,
    pub exec_start: String,
    pub user: String,
    pub working_directory: Option<PathBuf>,
    pub service_file: PathBuf,
}

#[derive(Clone)]
pub struct SystemdIntegration {
    units: Vec<SystemdUnit>,
    pid_to_unit: HashMap<i32, usize>, // pid -> index in units
    last_refresh: std::time::Instant,
    refresh_interval: std::time::Duration,
}

impl SystemdIntegration {
    pub fn new() -> Self {
        Self {
            units: Vec::new(),
            pid_to_unit: HashMap::new(),
            last_refresh: std::time::Instant::now(),
            refresh_interval: std::time::Duration::from_secs(60),
        }
    }

    /// Detect systemd units that manage Node.js applications
    pub fn detect_units(&mut self) -> Result<Vec<SystemdUnit>> {
        // Refresh if needed
        if self.last_refresh.elapsed() < self.refresh_interval {
            return Ok(self.units.clone());
        }

        let mut all_units = Vec::new();
        let mut pid_map = HashMap::new();

        // Scan systemd service directories
        let service_dirs = vec![
            "/etc/systemd/system",
            "/usr/lib/systemd/system",
        ];

        let node_pattern = Regex::new(r"(?i)(node|next|nest|pm2)").unwrap();

        for dir in service_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("service") {
                        if let Ok(unit) = Self::parse_service_file(&path) {
                            // Check if ExecStart contains node/next/nest/pm2
                            if node_pattern.is_match(&unit.exec_start) {
                                // Get PID for this unit
                                if let Ok(pid) = Self::get_unit_pid(&unit.name) {
                                    if let Some(pid) = pid {
                                        pid_map.insert(pid, all_units.len());
                                    }
                                }
                                all_units.push(unit);
                            }
                        }
                    }
                }
            }
        }

        // Update PIDs for all units
        for unit in &mut all_units {
            if unit.pid.is_none() {
                if let Ok(pid) = Self::get_unit_pid(&unit.name) {
                    unit.pid = pid;
                }
            }
        }
        
        // Build pid_map after updating all units
        for (idx, unit) in all_units.iter().enumerate() {
            if let Some(pid) = unit.pid {
                pid_map.insert(pid, idx);
            }
        }

        self.units = all_units;
        self.pid_to_unit = pid_map;
        self.last_refresh = std::time::Instant::now();

        info!("Detected {} systemd units managing Node.js apps", self.units.len());
        Ok(self.units.clone())
    }

    fn parse_service_file(path: &PathBuf) -> Result<SystemdUnit> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read service file: {:?}", path))?;

        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut exec_start = String::new();
        let mut user = String::from("root");
        let mut working_directory = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("ExecStart=") {
                exec_start = line.strip_prefix("ExecStart=")
                    .unwrap_or("")
                    .to_string();
            } else if line.starts_with("User=") {
                user = line.strip_prefix("User=")
                    .unwrap_or("root")
                    .to_string();
            } else if line.starts_with("WorkingDirectory=") {
                working_directory = Some(PathBuf::from(
                    line.strip_prefix("WorkingDirectory=").unwrap_or("/")
                ));
            }
        }

        Ok(SystemdUnit {
            name,
            pid: None, // Will be filled later
            exec_start,
            user,
            working_directory,
            service_file: path.clone(),
        })
    }

    fn get_unit_pid(unit_name: &str) -> Result<Option<i32>> {
        let output = Command::new("systemctl")
            .args(&["show", unit_name, "--property=MainPID", "--no-pager"])
            .output()
            .context("Failed to execute systemctl")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse systemctl output")?;

        for line in stdout.lines() {
            if line.starts_with("MainPID=") {
                let pid_str = line.strip_prefix("MainPID=").unwrap_or("0");
                if let Ok(pid) = pid_str.parse::<i32>() {
                    if pid > 0 {
                        return Ok(Some(pid));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn is_systemd_managed(&mut self, pid: i32) -> bool {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_units();
        }
        self.pid_to_unit.contains_key(&pid)
    }

    pub fn get_unit_by_pid(&mut self, pid: i32) -> Option<&SystemdUnit> {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_units();
        }
        self.pid_to_unit.get(&pid)
            .and_then(|&idx| self.units.get(idx))
    }

    pub async fn stop_unit(&self, unit_name: &str) -> Result<()> {
        // Check unit state before stopping
        let state_output = Command::new("systemctl")
            .args(&["is-active", unit_name])
            .output()
            .context("Failed to check unit state")?;

        let is_active = state_output.status.success();
        
        if !is_active {
            info!("Unit {} is not active, skipping stop", unit_name);
            return Ok(());
        }

        info!("Stopping systemd unit: {}", unit_name);

        let output = Command::new("systemctl")
            .args(&["stop", unit_name])
            .output()
            .context("Failed to execute systemctl stop")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("systemctl stop failed: {}", stderr));
        }

        info!("Successfully stopped systemd unit: {}", unit_name);
        Ok(())
    }

    pub fn get_all_units(&self) -> &[SystemdUnit] {
        &self.units
    }
}

impl Default for SystemdIntegration {
    fn default() -> Self {
        Self::new()
    }
}

