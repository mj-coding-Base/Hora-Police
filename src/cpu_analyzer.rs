use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::process_monitor::ProcessInfo;

#[derive(Debug, Clone)]
pub struct CpuAbuseDetection {
    pub pid: i32,
    pub cpu_percent: f32,
    pub duration_seconds: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

pub struct CpuAnalyzer {
    threshold: f32,
    duration_seconds: u64,
    process_history: HashMap<i32, (f32, DateTime<Utc>)>, // (pid, (max_cpu, first_seen))
}

impl CpuAnalyzer {
    pub fn new(threshold: f32, duration_minutes: u64) -> Self {
        Self {
            threshold,
            duration_seconds: duration_minutes * 60,
            process_history: HashMap::new(),
        }
    }

    pub fn analyze(&mut self, processes: &[ProcessInfo]) -> Vec<CpuAbuseDetection> {
        let now = Utc::now();
        let mut detections = Vec::new();

        for process in processes {
            // Skip if CPU is below threshold
            if process.cpu_percent < self.threshold {
                // Remove from history if it was being tracked
                self.process_history.remove(&process.pid);
                continue;
            }

            // Check if we're already tracking this process
            if let Some((max_cpu, first_seen)) = self.process_history.get_mut(&process.pid) {
                // Update max CPU if higher
                if process.cpu_percent > *max_cpu {
                    *max_cpu = process.cpu_percent;
                }

                // Check if duration threshold exceeded
                let duration = (now - *first_seen).num_seconds() as u64;
                if duration >= self.duration_seconds {
                    detections.push(CpuAbuseDetection {
                        pid: process.pid,
                        cpu_percent: *max_cpu,
                        duration_seconds: duration,
                        first_seen: *first_seen,
                        last_seen: now,
                    });
                }
            } else {
                // Start tracking this process
                self.process_history.insert(
                    process.pid,
                    (process.cpu_percent, now),
                );
            }
        }

        // Clean up processes that no longer exist
        let existing_pids: std::collections::HashSet<i32> = 
            processes.iter().map(|p| p.pid).collect();
        self.process_history.retain(|pid, _| existing_pids.contains(pid));

        detections
    }

    pub fn get_tracked_pids(&self) -> Vec<i32> {
        self.process_history.keys().copied().collect()
    }
}

