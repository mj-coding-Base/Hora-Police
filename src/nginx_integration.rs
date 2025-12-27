use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct NginxUpstream {
    pub name: String,
    pub port: u16,
    pub app_path: Option<PathBuf>,
    pub host: Option<String>,
}

#[derive(Clone)]
pub struct NginxIntegration {
    upstreams: Vec<NginxUpstream>,
    port_to_pid: HashMap<u16, Vec<i32>>,
    pid_to_upstream: HashMap<i32, usize>, // pid -> index in upstreams
    last_refresh: std::time::Instant,
    refresh_interval: std::time::Duration,
}

impl NginxIntegration {
    pub fn new() -> Self {
        Self {
            upstreams: Vec::new(),
            port_to_pid: HashMap::new(),
            pid_to_upstream: HashMap::new(),
            last_refresh: std::time::Instant::now(),
            refresh_interval: std::time::Duration::from_secs(60),
        }
    }

    /// Detect Nginx upstreams from configuration files
    pub fn detect_upstreams(&mut self) -> Result<Vec<NginxUpstream>> {
        // Refresh if needed
        if self.last_refresh.elapsed() < self.refresh_interval {
            return Ok(self.upstreams.clone());
        }

        let mut all_upstreams = Vec::new();

        // Scan Nginx configuration directories
        let nginx_dirs = vec![
            "/etc/nginx/sites-enabled",
            "/etc/nginx/conf.d",
        ];

        for dir in nginx_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(upstreams) = Self::parse_nginx_config(&path) {
                            all_upstreams.extend(upstreams);
                        }
                    }
                }
            }
        }

        // Map ports to PIDs
        let port_to_pid = Self::map_ports_to_pids()?;

        // Build reverse mapping: pid -> upstream
        let mut pid_to_upstream = HashMap::new();
        for (idx, upstream) in all_upstreams.iter().enumerate() {
            if let Some(pids) = port_to_pid.get(&upstream.port) {
                for &pid in pids {
                    pid_to_upstream.insert(pid, idx);
                }
            }
        }

        self.upstreams = all_upstreams;
        self.port_to_pid = port_to_pid;
        self.pid_to_upstream = pid_to_upstream;
        self.last_refresh = std::time::Instant::now();

        info!("Detected {} Nginx upstreams", self.upstreams.len());
        Ok(self.upstreams.clone())
    }

    fn parse_nginx_config(path: &PathBuf) -> Result<Vec<NginxUpstream>> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Nginx config: {:?}", path))?;

        let mut upstreams = Vec::new();
        let upstream_regex = Regex::new(r"upstream\s+(\w+)\s*\{([^}]+)\}").unwrap();
        let server_regex = Regex::new(r"server\s+([^;]+);").unwrap();
        let proxy_pass_regex = Regex::new(r"proxy_pass\s+http://([^;]+);").unwrap();

        // Find upstream blocks
        for cap in upstream_regex.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let servers_block = cap.get(2).unwrap().as_str();

            // Extract server addresses
            for server_cap in server_regex.captures_iter(servers_block) {
                let server_addr = server_cap.get(1).unwrap().as_str().trim();
                
                // Parse address (host:port or just :port)
                let (host, port): (Option<String>, &str) = if server_addr.starts_with(':') {
                    (None, server_addr.strip_prefix(':').unwrap_or("0"))
                } else if server_addr.contains(':') {
                    let parts: Vec<&str> = server_addr.split(':').collect();
                    (Some(parts[0].to_string()), parts.get(1).unwrap_or(&"0"))
                } else {
                    (None, "0")
                };

                if let Ok(port_num) = port.parse::<u16>() {
                    if port_num > 0 {
                        upstreams.push(NginxUpstream {
                            name: name.clone(),
                            port: port_num,
                            app_path: None, // Will try to infer from proxy_pass location
                            host,
                        });
                    }
                }
            }
        }

        // Also find proxy_pass directives to infer app paths
        for cap in proxy_pass_regex.captures_iter(&content) {
            let upstream_name = cap.get(1).unwrap().as_str();
            
            // Try to find location block context
            let location_regex = Regex::new(r"location\s+([^{]+)\s*\{[^}]*proxy_pass").unwrap();
            if let Some(loc_cap) = location_regex.captures(&content) {
                let location_path = loc_cap.get(1).unwrap().as_str().trim();
                
                // Try to infer app path from location
                for upstream in &mut upstreams {
                    if upstream.name == upstream_name {
                        // Common patterns: /var/www, /srv, /home/*/www
                        if location_path.contains("/var/www") {
                            upstream.app_path = Some(PathBuf::from("/var/www"));
                        } else if location_path.contains("/srv") {
                            upstream.app_path = Some(PathBuf::from("/srv"));
                        }
                    }
                }
            }
        }

        Ok(upstreams)
    }

    fn map_ports_to_pids() -> Result<HashMap<u16, Vec<i32>>> {
        let mut port_to_pid = HashMap::new();

        // Use ss command to get listening ports and PIDs
        let output = Command::new("ss")
            .args(&["-ltnp"])
            .output()
            .context("Failed to execute ss command")?;

        if !output.status.success() {
            // Fallback to lsof if ss is not available
            return Self::map_ports_to_pids_lsof();
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse ss output")?;

        // Parse ss output: LISTEN 0 128 *:3000 *:* users:(("node",pid=12345,fd=3))
        let pid_regex = Regex::new(r"pid=(\d+)").unwrap();
        let port_regex = Regex::new(r":(\d+)\s").unwrap();

        for line in stdout.lines() {
            if line.contains("LISTEN") && line.contains("node") {
                // Extract port
                if let Some(port_cap) = port_regex.captures(line) {
                    if let Ok(port) = port_cap.get(1).unwrap().as_str().parse::<u16>() {
                        // Extract PID
                        if let Some(pid_cap) = pid_regex.captures(line) {
                            if let Ok(pid) = pid_cap.get(1).unwrap().as_str().parse::<i32>() {
                                port_to_pid.entry(port).or_insert_with(Vec::new).push(pid);
                            }
                        }
                    }
                }
            }
        }

        Ok(port_to_pid)
    }

    fn map_ports_to_pids_lsof() -> Result<HashMap<u16, Vec<i32>>> {
        let mut port_to_pid = HashMap::new();

        let output = Command::new("lsof")
            .args(&["-i", "-P", "-n", "-t"])
            .output()
            .context("Failed to execute lsof command")?;

        if !output.status.success() {
            return Ok(port_to_pid);
        }

        // lsof -i output is complex, use a simpler approach
        // Get all Node processes and check their open files
        use sysinfo::{System, Pid};
        let mut system = System::new_all();
        system.refresh_all();

        for (pid, process) in system.processes() {
            if let Some(exe) = process.exe() {
                if exe.to_string_lossy().contains("node") {
                    // Try to get port from process's open files or command line
                    // This is a simplified approach - in production, you might want
                    // to use procfs to read /proc/PID/fd or /proc/PID/net/tcp
                    let cmd = process.cmd();
                    for arg in cmd {
                        // Look for port patterns in command line
                        if let Some(port_str) = arg.strip_prefix("--port=") {
                            if let Ok(port) = port_str.parse::<u16>() {
                                port_to_pid.entry(port).or_insert_with(Vec::new)
                                    .push(pid.as_u32() as i32);
                            }
                        }
                    }
                }
            }
        }

        Ok(port_to_pid)
    }

    pub fn is_nginx_upstream(&mut self, pid: i32) -> bool {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_upstreams();
        }
        self.pid_to_upstream.contains_key(&pid)
    }

    pub fn get_upstream_by_pid(&mut self, pid: i32) -> Option<&NginxUpstream> {
        // Refresh if needed
        if self.last_refresh.elapsed() >= self.refresh_interval {
            let _ = self.detect_upstreams();
        }
        self.pid_to_upstream.get(&pid)
            .and_then(|&idx| self.upstreams.get(idx))
    }

    pub fn get_all_upstreams(&self) -> &[NginxUpstream] {
        &self.upstreams
    }

    pub fn get_pids_for_port(&self, port: u16) -> Vec<i32> {
        self.port_to_pid.get(&port)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for NginxIntegration {
    fn default() -> Self {
        Self::new()
    }
}

