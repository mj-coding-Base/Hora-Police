use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use tracing::{info, warn, error};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;

#[derive(Debug, Clone)]
pub struct ZombieInfo {
    pub pid: i32,
    pub ppid: i32,
    pub cmd: String,
}

#[derive(Debug)]
pub struct ZombieStats {
    pub total_count: usize,
    pub by_parent: HashMap<i32, usize>,
    pub zombies: Vec<ZombieInfo>,
}

pub struct ZombieReaper {
    pub threshold: usize,
    last_cleanup: std::time::Instant,
    cleanup_interval: std::time::Duration,
}

impl ZombieReaper {
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            last_cleanup: std::time::Instant::now(),
            cleanup_interval: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    /// Detect all zombie processes in the system
    pub fn detect_zombies(&self) -> Result<ZombieStats> {
        let mut zombies = Vec::new();
        let mut by_parent: HashMap<i32, usize> = HashMap::new();

        // Read /proc to find zombie processes
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let pid_str = entry.file_name().to_string_lossy().to_string();
                
                // Skip non-numeric entries
                if pid_str.parse::<i32>().is_err() {
                    continue;
                }

                let pid = pid_str.parse::<i32>().unwrap();

                // Read process stat to check state
                let stat_path = format!("/proc/{}/stat", pid);
                if let Ok(stat_content) = fs::read_to_string(&stat_path) {
                    let fields: Vec<&str> = stat_content.split_whitespace().collect();
                    if fields.len() >= 3 {
                        let state = fields[2];
                        if state == "Z" {
                            // This is a zombie
                            let ppid = fields[3].parse::<i32>().unwrap_or(0);
                            let cmd = fields[1].trim_matches('(').trim_matches(')').to_string();

                            zombies.push(ZombieInfo {
                                pid,
                                ppid,
                                cmd,
                            });

                            *by_parent.entry(ppid).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        Ok(ZombieStats {
            total_count: zombies.len(),
            by_parent,
            zombies,
        })
    }

    /// Safely reap zombies by calling waitpid on them
    /// Returns number of zombies reaped
    pub fn reap_zombies(&self, zombies: &[ZombieInfo]) -> Result<usize> {
        let mut reaped = 0;

        for zombie in zombies {
            // Try to reap the zombie
            // Using WNOHANG to avoid blocking
            match waitpid(Pid::from_raw(zombie.pid), Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive) => {
                    // Process is still alive (shouldn't happen for zombies, but handle it)
                    continue;
                }
                Ok(WaitStatus::Exited(_, _)) => {
                    reaped += 1;
                    info!("Reaped zombie process PID {} (parent: {})", zombie.pid, zombie.ppid);
                }
                Ok(WaitStatus::Signaled(_, _, _)) => {
                    reaped += 1;
                    info!("Reaped zombie process PID {} (parent: {})", zombie.pid, zombie.ppid);
                }
                Ok(_) => {
                    reaped += 1;
                    info!("Reaped zombie process PID {} (parent: {})", zombie.pid, zombie.ppid);
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // Process already reaped by parent or doesn't exist
                    // This is fine, continue
                }
                Err(e) => {
                    warn!("Failed to reap zombie PID {}: {}", zombie.pid, e);
                }
            }
        }

        Ok(reaped)
    }

    /// Check for zombies and reap if threshold exceeded
    /// Returns (total_count, reaped_count, should_alert)
    pub fn check_and_reap(&mut self) -> Result<(usize, usize, bool)> {
        // Only check periodically to avoid overhead
        if self.last_cleanup.elapsed() < self.cleanup_interval {
            return Ok((0, 0, false));
        }

        self.last_cleanup = std::time::Instant::now();

        let stats = self.detect_zombies()?;
        let total_count = stats.total_count;

        if total_count == 0 {
            return Ok((0, 0, false));
        }

        info!("Detected {} zombie processes", total_count);

        // Log parent distribution
        if !stats.by_parent.is_empty() {
            let mut parent_counts: Vec<_> = stats.by_parent.iter().collect();
            parent_counts.sort_by(|a, b| b.1.cmp(&a.1));
            
            info!("Zombie distribution by parent:");
            for (ppid, count) in parent_counts.iter().take(10) {
                info!("  Parent PID {}: {} zombies", ppid, count);
            }
        }

        // Reap zombies if threshold exceeded
        let reaped = if total_count >= self.threshold {
            warn!("Zombie count {} exceeds threshold {}, attempting to reap", 
                  total_count, self.threshold);
            self.reap_zombies(&stats.zombies)?
        } else {
            0
        };

        let should_alert = total_count >= self.threshold * 2;

        Ok((total_count, reaped, should_alert))
    }

    /// Get the most common zombie parent PIDs
    pub fn get_top_zombie_parents(&self, limit: usize) -> Result<Vec<(i32, usize)>> {
        let stats = self.detect_zombies()?;
        let mut parent_counts: Vec<(i32, usize)> = stats.by_parent
            .into_iter()
            .collect();
        parent_counts.sort_by(|a, b| b.1.cmp(&a.1));
        parent_counts.truncate(limit);
        Ok(parent_counts)
    }
}

impl Default for ZombieReaper {
    fn default() -> Self {
        Self::new(100) // Default threshold: 100 zombies
    }
}

