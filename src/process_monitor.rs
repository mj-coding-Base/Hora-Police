use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use sysinfo::{Pid, System, Process, User};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub binary_path: String,
    pub command_line: String,
    pub cpu_percent: f32,
}

pub struct ProcessMonitor {
    system: System,
    last_cpu_times: HashMap<i32, (u64, u64)>, // (pid, (total_time, timestamp))
}

impl ProcessMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            last_cpu_times: HashMap::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    pub fn get_all_processes(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for (pid, process) in self.system.processes() {
            let pid_int = pid.as_u32() as i32;
            
            // Get binary path
            let binary_path = process
                .exe()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            // Get command line
            let command_line = process
                .cmd()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500) // Limit length
                .collect::<String>();

            // Get UID
            let uid = process.user_id().map(|u| *u).unwrap_or(0);

            // Get PPID
            let ppid = process.parent()
                .map(|p| p.as_u32() as i32)
                .unwrap_or(0);

            // Calculate CPU percent
            let cpu_percent = process.cpu_usage() as f32;

            processes.push(ProcessInfo {
                pid: pid_int,
                ppid,
                uid,
                binary_path,
                command_line,
                cpu_percent,
            });
        }

        Ok(processes)
    }

    pub fn get_process_by_pid(&self, pid: i32) -> Option<ProcessInfo> {
        let pid_obj = Pid::from_u32(pid as u32);
        self.system.process(pid_obj).map(|process| {
            let binary_path = process
                .exe()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            let command_line = process
                .cmd()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500)
                .collect::<String>();

            let uid = process.user_id().map(|u| *u).unwrap_or(0);
            let ppid = process.parent()
                .map(|p| p.as_u32() as i32)
                .unwrap_or(0);
            let cpu_percent = process.cpu_usage() as f32;

            ProcessInfo {
                pid,
                ppid,
                uid,
                binary_path,
                command_line,
                cpu_percent,
            }
        })
    }

    pub fn get_process_tree(&self, pid: i32) -> Vec<i32> {
        let mut tree = vec![pid];
        let mut current_pid = pid;

        // Walk up the process tree
        for _ in 0..100 { // Safety limit
            if let Some(process) = self.get_process_by_pid(current_pid) {
                if process.ppid == 0 || process.ppid == current_pid {
                    break;
                }
                tree.push(process.ppid);
                current_pid = process.ppid;
            } else {
                break;
            }
        }

        tree
    }

    pub fn is_safe_binary(&self, binary_path: &str) -> bool {
        // Whitelist of known safe binaries
        let safe_binaries = [
            "/usr/bin/",
            "/usr/sbin/",
            "/bin/",
            "/sbin/",
            "/usr/lib/",
            "/lib/",
            "/opt/",
        ];

        safe_binaries.iter().any(|prefix| binary_path.starts_with(prefix))
    }
}

