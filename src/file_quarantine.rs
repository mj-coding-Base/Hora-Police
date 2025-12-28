use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use chrono::Utc;
use tracing::{info, warn, error};
use walkdir::WalkDir;
use nix::unistd::Pid;
use nix::sys::signal;

pub struct FileQuarantine {
    quarantine_dir: PathBuf,
    auto_delete: bool,
    aggressive_cleanup: bool,
}

impl FileQuarantine {
    pub fn new(quarantine_dir: PathBuf, auto_delete: bool) -> Self {
        Self::new_with_cleanup(quarantine_dir, auto_delete, true)
    }

    pub fn new_with_cleanup(quarantine_dir: PathBuf, auto_delete: bool, aggressive_cleanup: bool) -> Self {
        // Ensure quarantine directory exists
        if let Err(e) = fs::create_dir_all(&quarantine_dir) {
            warn!("Failed to create quarantine directory {}: {}", quarantine_dir.display(), e);
        }

        Self {
            quarantine_dir,
            auto_delete,
            aggressive_cleanup,
        }
    }

    /// Quarantine a file by moving it to the quarantine directory
    pub fn quarantine_file(&self, file_path: &Path) -> Result<PathBuf> {
        if !file_path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", file_path.display()));
        }

        // Generate quarantine filename with timestamp
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let quarantine_name = format!("{}_{}", timestamp, file_name);
        let quarantine_path = self.quarantine_dir.join(&quarantine_name);

        // Move file to quarantine
        fs::rename(file_path, &quarantine_path)
            .with_context(|| format!("Failed to move file to quarantine: {}", file_path.display()))?;

        info!("✅ Quarantined file: {} -> {}", 
              file_path.display(), quarantine_path.display());

        Ok(quarantine_path)
    }

    /// Delete a malicious file permanently
    pub fn delete_file(&self, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Ok(()); // Already deleted
        }

        // Remove write protection if present
        let mut perms = fs::metadata(file_path)?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(file_path, perms)?;

        // Delete the file
        fs::remove_file(file_path)
            .with_context(|| format!("Failed to delete file: {}", file_path.display()))?;

        info!("🗑️  Deleted malicious file: {}", file_path.display());

        Ok(())
    }

    /// Quarantine or delete based on configuration
    pub fn handle_malware(&self, file_path: &Path) -> Result<QuarantineResult> {
        if self.auto_delete {
            self.delete_file(file_path)?;
            Ok(QuarantineResult::Deleted)
        } else {
            let quarantine_path = self.quarantine_file(file_path)?;
            Ok(QuarantineResult::Quarantined(quarantine_path))
        }
    }

    /// Check if a process has a file open by examining /proc/PID/fd
    fn process_has_file_open(pid: i32, file_path: &Path) -> bool {
        let fd_dir = format!("/proc/{}/fd", pid);
        if let Ok(entries) = std::fs::read_dir(&fd_dir) {
            for entry in entries.flatten() {
                if let Ok(target) = std::fs::read_link(entry.path()) {
                    if target == file_path {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Kill any processes using the file (enhanced with file handle detection and process tree killing)
    pub async fn kill_processes_using_file(&self, file_path: &Path) -> Result<Vec<i32>> {
        use crate::process_monitor::ProcessMonitor;
        
        let mut monitor = ProcessMonitor::new();
        monitor.refresh();
        
        let processes = monitor.get_all_processes()?;
        let mut pids_to_kill = std::collections::HashSet::new();
        let file_path_str = file_path.to_string_lossy();

        for process in processes {
            // Method 1: Check if process binary matches the file
            if process.binary_path == file_path_str {
                info!("🔍 Found process PID {} with binary matching malicious file: {}", 
                      process.pid, file_path_str);
                pids_to_kill.insert(process.pid);
            }

            // Method 2: Check if command line references the file
            if process.command_line.contains(&*file_path_str) {
                info!("🔍 Found process PID {} with command line referencing malicious file: {}", 
                      process.pid, file_path_str);
                pids_to_kill.insert(process.pid);
            }

            // Method 3: Check if process has the file open via file descriptors
            if Self::process_has_file_open(process.pid, file_path) {
                info!("🔍 Found process PID {} with file descriptor open to malicious file: {}", 
                      process.pid, file_path_str);
                pids_to_kill.insert(process.pid);
            }
        }

        // Kill all identified processes and their process trees
        let mut killed_pids = Vec::new();

        for pid in pids_to_kill {
            info!("🔪 Killing process tree for PID {} using malicious file: {}", 
                  pid, file_path_str);
            
            // Get the full process tree (parent + all children)
            let tree_pids = monitor.get_full_process_tree(pid);
            
            // Kill children first (reverse order to kill deepest first)
            let mut children_first = tree_pids.clone();
            children_first.reverse();
            
            for tree_pid in children_first {
                if tree_pid == pid {
                    continue; // Kill parent last
                }
                
                let pid_obj = Pid::from_raw(tree_pid);
                if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
                    killed_pids.push(tree_pid);
                    info!("✅ Sent SIGTERM to child PID {}", tree_pid);
                }
            }
            
            // Wait for children to terminate
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Force kill any remaining children
            monitor.refresh();
            for tree_pid in monitor.get_full_process_tree(pid) {
                if tree_pid == pid {
                    continue; // Handle parent separately
                }
                if monitor.get_process_by_pid(tree_pid).is_some() {
                    let pid_obj = Pid::from_raw(tree_pid);
                    let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                    warn!("⚠️  Force killed child PID {}", tree_pid);
                }
            }
            
            // Now kill the parent
            let pid_obj = Pid::from_raw(pid);
            if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
                killed_pids.push(pid);
                info!("✅ Sent SIGTERM to parent PID {}", pid);
                
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Force kill if still alive
                monitor.refresh();
                if monitor.get_process_by_pid(pid).is_some() {
                    let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                    warn!("⚠️  Force killed parent PID {}", pid);
                }
            }
        }

        Ok(killed_pids)
    }

    /// Get quarantine directory path
    pub fn get_quarantine_dir(&self) -> &Path {
        &self.quarantine_dir
    }

    /// Aggressively clean up malware origin - delete parent directory and related files
    pub fn delete_malware_origin(&self, malware_path: &Path) -> Result<OriginCleanupResult> {
        if !self.aggressive_cleanup {
            return Ok(OriginCleanupResult {
                deleted_files: Vec::new(),
                deleted_directories: Vec::new(),
                cleaned_cron_jobs: Vec::new(),
            });
        }

        let mut cleanup_result = OriginCleanupResult {
            deleted_files: Vec::new(),
            deleted_directories: Vec::new(),
            cleaned_cron_jobs: Vec::new(),
        };

        // Get parent directory
        if let Some(parent_dir) = malware_path.parent() {
            // Check if parent directory only contains suspicious files
            if self.is_suspicious_directory(parent_dir)? {
                info!("🗑️  Deleting suspicious parent directory: {}", parent_dir.display());
                
                // Delete all files in the directory first
                if let Ok(entries) = fs::read_dir(parent_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Err(e) = self.force_delete_file(&path) {
                                warn!("Failed to delete file {}: {}", path.display(), e);
                            } else {
                                cleanup_result.deleted_files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }

                // Try to remove the directory
                if let Err(e) = fs::remove_dir(parent_dir) {
                    warn!("Failed to remove directory {}: {}", parent_dir.display(), e);
                } else {
                    cleanup_result.deleted_directories.push(parent_dir.to_string_lossy().to_string());
                    info!("✅ Deleted suspicious directory: {}", parent_dir.display());
                }
            } else {
                // Directory has legitimate files, only delete the malware file
                info!("⚠️  Parent directory contains legitimate files, only deleting malware file");
            }
        }

        // Clean up related files in the same directory
        if let Some(parent_dir) = malware_path.parent() {
            if let Ok(entries) = fs::read_dir(parent_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && self.is_suspicious_file(&path) {
                        if path != malware_path {
                            info!("🗑️  Deleting related suspicious file: {}", path.display());
                            if let Err(e) = self.force_delete_file(&path) {
                                warn!("Failed to delete related file {}: {}", path.display(), e);
                            } else {
                                cleanup_result.deleted_files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        // Clean up cron jobs that reference this malware
        cleanup_result.cleaned_cron_jobs = self.clean_cron_jobs_referencing(malware_path)?;

        Ok(cleanup_result)
    }

    fn is_suspicious_directory(&self, dir: &Path) -> Result<bool> {
        // Check if directory contains only suspicious files
        let mut suspicious_count = 0;
        let mut total_files = 0;

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    total_files += 1;
                    if self.is_suspicious_file(&path) {
                        suspicious_count += 1;
                    }
                }
            }
        }

        // If all files are suspicious, directory is suspicious
        Ok(total_files > 0 && suspicious_count == total_files)
    }

    fn is_suspicious_file(&self, path: &Path) -> bool {
        // Check for common malware patterns
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        let suspicious_names = [
            "solrz", "e386", "payload.so", "next", "miner", "xmrig", 
            "ccminer", "cpuminer", "malware", "trojan", "virus"
        ];

        suspicious_names.iter().any(|&name| file_name.contains(name))
    }

    fn force_delete_file(&self, path: &Path) -> Result<()> {
        // Remove all permissions and delete
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(path, perms)?;

        // Try to delete
        fs::remove_file(path)
            .with_context(|| format!("Failed to force delete: {}", path.display()))?;

        Ok(())
    }

    fn clean_cron_jobs_referencing(&self, malware_path: &Path) -> Result<Vec<String>> {
        use crate::cron_watcher::CronWatcher;
        
        let mut cleaned = Vec::new();
        let malware_path_str = malware_path.to_string_lossy();

        // Check all cron locations
        let mut cron_watcher = CronWatcher::new();
        if let Ok(jobs) = cron_watcher.scan_all() {
            let malware_name = malware_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            for job in jobs {
                let job_content_lower = job.content.to_lowercase();
                let mut should_remove = false;
                
                // Direct path reference
                if job.content.contains(&*malware_path_str) {
                    should_remove = true;
                }
                
                // Check for obfuscated references
                if job_content_lower.contains(&malware_name) {
                    should_remove = true;
                }
                
                // Check for suspicious patterns
                let suspicious_patterns = ["wget", "curl", "base64", "eval", "bash.*<"];
                for pattern in &suspicious_patterns {
                    if job_content_lower.contains(pattern) {
                        should_remove = true;
                        break;
                    }
                }
                
                if should_remove {
                    info!("🗑️  Removing suspicious cron job: {}", job.file_path);
                    
                    // Try to remove the cron entry
                    if let Err(e) = self.remove_cron_entry(&job.file_path, &job.content) {
                        warn!("Failed to remove cron entry: {}", e);
                    } else {
                        cleaned.push(job.file_path.clone());
                    }
                }
            }
        }

        Ok(cleaned)
    }

    fn remove_cron_entry(&self, cron_file: &str, content: &str) -> Result<()> {
        // Read current cron file
        let current_content = fs::read_to_string(cron_file)
            .with_context(|| format!("Failed to read cron file: {}", cron_file))?;

        // Remove lines containing the malware path
        let lines: Vec<&str> = current_content
            .lines()
            .filter(|line| !line.contains(content) && !line.trim().is_empty())
            .collect();

        // Write back without the malicious entries
        let new_content = lines.join("\n");
        if !new_content.is_empty() {
            fs::write(cron_file, new_content)
                .with_context(|| format!("Failed to write cron file: {}", cron_file))?;
        } else {
            // If file is empty, remove it
            fs::remove_file(cron_file)
                .with_context(|| format!("Failed to remove empty cron file: {}", cron_file))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct OriginCleanupResult {
    pub deleted_files: Vec<String>,
    pub deleted_directories: Vec<String>,
    pub cleaned_cron_jobs: Vec<String>,
}

impl OriginCleanupResult {
    pub fn is_empty(&self) -> bool {
        self.deleted_files.is_empty() 
            && self.deleted_directories.is_empty() 
            && self.cleaned_cron_jobs.is_empty()
    }
}

#[derive(Debug)]
pub enum QuarantineResult {
    Quarantined(PathBuf),
    Deleted,
}

