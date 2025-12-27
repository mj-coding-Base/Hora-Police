use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use tracing::info;

use crate::process_monitor::ProcessInfo;

pub struct DeployDetector {
    grace_period_minutes: u64,
    recent_deploys: HashMap<PathBuf, DateTime<Utc>>,
}

impl DeployDetector {
    pub fn new(grace_period_minutes: u64) -> Self {
        Self {
            grace_period_minutes,
            recent_deploys: HashMap::new(),
        }
    }

    /// Check if a process should have kill suspended due to recent deployment
    pub fn should_suspend_kill(&mut self, process: &ProcessInfo) -> bool {
        // Extract working directory from process
        let work_dir = Self::extract_working_directory(process);
        
        if let Some(dir) = work_dir {
            // Check if there was recent deploy activity in this directory
            if self.detect_recent_deploy(&dir) {
                info!("Suspending kill for PID {} due to recent deployment in {}", 
                      process.pid, dir.display());
                return true;
            }
        }

        false
    }

    fn extract_working_directory(process: &ProcessInfo) -> Option<PathBuf> {
        // Try to extract working directory from command line
        // Common patterns:
        // - node /path/to/app/dist/main.js
        // - next start (in /path/to/app)
        // - npm run start (in /path/to/app)
        
        let cmd = &process.command_line;
        
        // Look for absolute paths in command line
        for part in cmd.split_whitespace() {
            if part.starts_with('/') && PathBuf::from(part).exists() {
                if let Some(parent) = PathBuf::from(part).parent() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        // Fallback: use binary path's parent
        if !process.binary_path.is_empty() && process.binary_path != "unknown" {
            if let Some(parent) = PathBuf::from(&process.binary_path).parent() {
                return Some(parent.to_path_buf());
            }
        }

        None
    }

    /// Detect if there was recent deployment activity in a directory
    pub fn detect_recent_deploy(&mut self, path: &Path) -> bool {
        // Check if we already cached this
        if let Some(&last_check) = self.recent_deploys.get(path) {
            let elapsed = Utc::now() - last_check;
            if elapsed.num_minutes() < self.grace_period_minutes as i64 {
                return true; // Still in grace period
            }
        }

        // Check for git activity
        if self.check_git_activity(path) {
            self.recent_deploys.insert(path.to_path_buf(), Utc::now());
            return true;
        }

        // Check for npm/yarn activity
        if self.check_npm_activity(path) {
            self.recent_deploys.insert(path.to_path_buf(), Utc::now());
            return true;
        }

        false
    }

    fn check_git_activity(&self, path: &Path) -> bool {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return false;
        }

        // Check HEAD modification time
        let head_file = git_dir.join("HEAD");
        if let Ok(metadata) = fs::metadata(&head_file) {
            if let Ok(modified) = metadata.modified() {
                let modified_time: DateTime<Utc> = modified.into();
                let elapsed = Utc::now() - modified_time;
                if elapsed.num_minutes() < self.grace_period_minutes as i64 {
                    return true;
                }
            }
        }

        // Check for recent git operations (look at refs)
        let refs_dir = git_dir.join("refs/heads");
        if refs_dir.exists() {
            if let Ok(entries) = fs::read_dir(&refs_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let modified_time: DateTime<Utc> = modified.into();
                            let elapsed = Utc::now() - modified_time;
                            if elapsed.num_minutes() < self.grace_period_minutes as i64 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    fn check_npm_activity(&self, path: &Path) -> bool {
        // Check for package-lock.json or yarn.lock modification
        let lock_files = vec![
            path.join("package-lock.json"),
            path.join("yarn.lock"),
            path.join("pnpm-lock.yaml"),
        ];

        for lock_file in lock_files {
            if let Ok(metadata) = fs::metadata(&lock_file) {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: DateTime<Utc> = modified.into();
                    let elapsed = Utc::now() - modified_time;
                    if elapsed.num_minutes() < self.grace_period_minutes as i64 {
                        return true;
                    }
                }
            }
        }

        // Check for node_modules/.cache modification
        let cache_dir = path.join("node_modules/.cache");
        if cache_dir.exists() {
            if let Ok(metadata) = fs::metadata(&cache_dir) {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: DateTime<Utc> = modified.into();
                    let elapsed = Utc::now() - modified_time;
                    if elapsed.num_minutes() < self.grace_period_minutes as i64 {
                        return true;
                    }
                }
            }
        }

        // Check for running npm/yarn/pnpm install processes
        if self.check_install_processes(path) {
            return true;
        }

        false
    }

    fn check_install_processes(&self, path: &Path) -> bool {
        // Check if there are npm/yarn/pnpm install processes running
        // This is a simplified check - in production you might want to check
        // the process tree more thoroughly
        
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let path_str = path.to_string_lossy();
            
            // Look for install commands in processes
            let install_patterns = vec![
                "npm install",
                "yarn install",
                "pnpm install",
                "npm run build",
                "yarn build",
                "next build",
                "nest build",
            ];

            for pattern in install_patterns {
                if stdout.contains(pattern) && stdout.contains(&*path_str) {
                    return true;
                }
            }
        }

        false
    }

    /// Clean up old deploy records
    pub fn cleanup_old_records(&mut self) {
        let now = Utc::now();
        self.recent_deploys.retain(|_, &mut timestamp| {
            let elapsed = now - timestamp;
            elapsed.num_minutes() < (self.grace_period_minutes * 2) as i64
        });
    }
}

