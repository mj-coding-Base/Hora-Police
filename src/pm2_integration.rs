use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct Pm2App {
    pub name: String,
    pub pid: i32,
    pub path: PathBuf,
    pub user: String,
    pub status: String,
    pub pm_id: u32,
}

#[derive(Deserialize)]
struct Pm2JsonOutput {
    name: String,
    pid: Option<u32>,
    pm2_env: Pm2Env,
}

#[derive(Deserialize)]
struct Pm2Env {
    pm_exec_path: String,
    status: String,
    pm_id: u32,
    username: Option<String>,
}

pub struct Pm2Integration {
    apps: Vec<Pm2App>,
    pid_to_app: HashMap<i32, usize>, // pid -> index in apps
    last_refresh: std::time::Instant,
    refresh_interval: std::time::Duration,
}

impl Pm2Integration {
    pub fn new() -> Self {
        Self {
            apps: Vec::new(),
            pid_to_app: HashMap::new(),
            last_refresh: std::time::Instant::now(),
            refresh_interval: std::time::Duration::from_secs(30),
        }
    }

    /// Detect PM2 apps for all users
    pub fn detect_apps(&mut self) -> Result<Vec<Pm2App>> {
        // Refresh if needed
        if self.last_refresh.elapsed() < self.refresh_interval {
            return Ok(self.apps.clone());
        }

        let mut all_apps = Vec::new();
        let mut pid_map = HashMap::new();

        // Try to detect PM2 apps for current user and common users
        let users = vec!["root", "deploy", "www-data", "ubuntu"];
        
        for user in users {
            match Self::detect_apps_for_user(user) {
                Ok(mut apps) => {
                    for (idx, app) in apps.iter().enumerate() {
                        pid_map.insert(app.pid, all_apps.len() + idx);
                    }
                    all_apps.append(&mut apps);
                }
                Err(e) => {
                    // Silently fail for users that don't exist or don't have PM2
                    if user == "root" {
                        warn!("Failed to detect PM2 apps for {}: {}", user, e);
                    }
                }
            }
        }

        // Also check for PM2 daemon process and its children
        if let Ok(daemon_apps) = Self::detect_via_process_tree() {
            for app in daemon_apps {
                if !pid_map.contains_key(&app.pid) {
                    pid_map.insert(app.pid, all_apps.len());
                    all_apps.push(app);
                }
            }
        }

        self.apps = all_apps;
        self.pid_to_app = pid_map;
        self.last_refresh = std::time::Instant::now();

        info!("Detected {} PM2 apps", self.apps.len());
        Ok(self.apps.clone())
    }

    fn detect_apps_for_user(user: &str) -> Result<Vec<Pm2App>> {
        // Try to run pm2 jlist (JSON list) first, then fallback to pm2 ls
        let output = if user == "root" {
            // Try pm2 jlist first (more reliable JSON output)
            Command::new("pm2")
                .args(&["jlist"])
                .output()
                .or_else(|_| {
                    // Fallback to pm2 ls
                    Command::new("pm2")
                        .args(&["ls", "--no-color", "--format", "json"])
                        .output()
                })
        } else {
            // Try pm2 jlist first
            Command::new("sudo")
                .args(&["-u", user, "pm2", "jlist"])
                .output()
                .or_else(|_| {
                    // Fallback to pm2 ls
                    Command::new("sudo")
                        .args(&["-u", user, "pm2", "ls", "--no-color", "--format", "json"])
                        .output()
                })
        };

        let output = output.context("Failed to execute pm2 command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "PM2 command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Failed to parse PM2 output")?;

        // PM2 outputs JSON array or object with processes array
        let apps: Vec<Pm2JsonOutput> = if json_str.trim().starts_with('[') {
            serde_json::from_str(&json_str)
                .context("Failed to parse PM2 JSON array")?
        } else {
            // Try parsing as object with processes field
            #[derive(Deserialize)]
            struct Pm2Output {
                processes: Vec<Pm2JsonOutput>,
            }
            let output: Pm2Output = serde_json::from_str(&json_str)
                .context("Failed to parse PM2 JSON object")?;
            output.processes
        };

        let mut result = Vec::new();
        for app_json in apps {
            if let Some(pid) = app_json.pid {
                let path = PathBuf::from(&app_json.pm2_env.pm_exec_path);
                let username = app_json.pm2_env.username
                    .unwrap_or_else(|| user.to_string());

                result.push(Pm2App {
                    name: app_json.name,
                    pid: pid as i32,
                    path,
                    user: username,
                    status: app_json.pm2_env.status,
                    pm_id: app_json.pm2_env.pm_id,
                });
            }
        }

        Ok(result)
    }

    fn detect_via_process_tree() -> Result<Vec<Pm2App>> {
        // Check for PM2 daemon process and find its children
        use sysinfo::{System, Pid};
        
        let mut system = System::new_all();
        system.refresh_all();

        let mut apps = Vec::new();
        
        // Find PM2 daemon process and Node processes with PM2 in parent chain
        for (pid, process) in system.processes() {
            let exe = process.exe();
            if let Some(exe_path) = exe {
                let exe_str = exe_path.to_string_lossy();
                
                // Check if it's a Node process
                if exe_str.contains("node") {
                    // Check parent process to see if it's PM2
                    let parent_pid = process.parent();
                    if let Some(parent_pid) = parent_pid {
                        if let Some(parent) = system.process(parent_pid) {
                            if let Some(parent_exe) = parent.exe() {
                                let parent_exe_str = parent_exe.to_string_lossy();
                                if parent_exe_str.contains("pm2") {
                                    // This is a PM2-managed Node process
                                    let cmd = process.cmd();
                                    let name = cmd.first()
                                        .and_then(|s| s.split('/').last())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    
                                    let path = process.cwd()
                                        .unwrap_or_else(|| PathBuf::from("/"));

                                    apps.push(Pm2App {
                                        name,
                                        pid: pid.as_u32() as i32,
                                        path,
                                        user: "unknown".to_string(),
                                        status: "online".to_string(),
                                        pm_id: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(apps)
    }

    pub fn is_pm2_managed(&mut self, pid: i32) -> bool {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_apps();
        }
        self.pid_to_app.contains_key(&pid)
    }

    pub fn get_app_by_pid(&mut self, pid: i32) -> Option<&Pm2App> {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_apps();
        }
        self.pid_to_app.get(&pid)
            .and_then(|&idx| self.apps.get(idx))
    }

    pub async fn stop_app(&self, app_name: &str, user: &str) -> Result<()> {
        info!("Stopping PM2 app: {} (user: {})", app_name, user);

        let output = if user == "root" {
            Command::new("pm2")
                .args(&["stop", app_name])
                .output()
        } else {
            Command::new("sudo")
                .args(&["-u", user, "pm2", "stop", app_name])
                .output()
        };

        let output = output.context("Failed to execute pm2 stop")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("PM2 stop failed: {}", stderr));
        }

        info!("Successfully stopped PM2 app: {}", app_name);
        Ok(())
    }

    pub fn get_all_apps(&self) -> &[Pm2App] {
        &self.apps
    }
}

impl Default for Pm2Integration {
    fn default() -> Self {
        Self::new()
    }
}

