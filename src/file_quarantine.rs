use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use chrono::Utc;
use tracing::{info, warn, error};

pub struct FileQuarantine {
    quarantine_dir: PathBuf,
    auto_delete: bool,
}

impl FileQuarantine {
    pub fn new(quarantine_dir: PathBuf, auto_delete: bool) -> Self {
        // Ensure quarantine directory exists
        if let Err(e) = fs::create_dir_all(&quarantine_dir) {
            warn!("Failed to create quarantine directory {}: {}", quarantine_dir.display(), e);
        }

        Self {
            quarantine_dir,
            auto_delete,
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

    /// Kill any processes using the file
    pub async fn kill_processes_using_file(&self, file_path: &Path) -> Result<Vec<i32>> {
        use crate::process_monitor::ProcessMonitor;
        
        let monitor = ProcessMonitor::new();
        monitor.refresh();
        
        let processes = monitor.get_all_processes()?;
        let mut killed_pids = Vec::new();
        let file_path_str = file_path.to_string_lossy();

        for process in processes {
            // Check if process binary matches the file
            if process.binary_path == file_path_str {
                info!("🔪 Killing process PID {} using malicious file: {}", 
                      process.pid, file_path_str);
                
                use nix::sys::signal;
                use nix::unistd::Pid;
                
                let pid_obj = Pid::from_raw(process.pid);
                if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
                    killed_pids.push(process.pid);
                    
                    // Wait and force kill if needed
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    if monitor.get_process_by_pid(process.pid).is_some() {
                        let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                    }
                }
            }

            // Check if command line references the file
            if process.command_line.contains(&file_path_str) {
                info!("🔪 Killing process PID {} referencing malicious file: {}", 
                      process.pid, file_path_str);
                
                let pid_obj = Pid::from_raw(process.pid);
                if signal::kill(pid_obj, signal::Signal::SIGTERM).is_ok() {
                    killed_pids.push(process.pid);
                    
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    if monitor.get_process_by_pid(process.pid).is_some() {
                        let _ = signal::kill(pid_obj, signal::Signal::SIGKILL);
                    }
                }
            }
        }

        Ok(killed_pids)
    }

    /// Get quarantine directory path
    pub fn get_quarantine_dir(&self) -> &Path {
        &self.quarantine_dir
    }
}

#[derive(Debug)]
pub enum QuarantineResult {
    Quarantined(PathBuf),
    Deleted,
}

