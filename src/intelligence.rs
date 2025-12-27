use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::database::{IntelligenceDB, SuspiciousProcess};
use crate::process_monitor::ProcessInfo;

pub struct BehaviorIntelligence {
    db: IntelligenceDB,
    learning_mode: bool,
}

impl BehaviorIntelligence {
    pub async fn new(db: IntelligenceDB, learning_mode: bool) -> Result<Self> {
        Ok(Self {
            db,
            learning_mode,
        })
    }

    pub async fn analyze_process(
        &self,
        process: &ProcessInfo,
        cpu_percent: f32,
        duration_seconds: u64,
        first_seen: DateTime<Utc>,
    ) -> Result<f32> {
        // Check if we've seen this binary before
        if let Ok(Some(existing)) = self.db.get_suspicious_by_binary(&process.binary_path).await {
            // Increase confidence based on repeat behavior
            let mut confidence = existing.threat_confidence;
            
            // If it restarted, increase threat
            if existing.pid != process.pid && existing.binary_path == process.binary_path {
                confidence += 0.2;
            }
            
            // If spawn count is high, increase threat
            if existing.spawn_count > 3 {
                confidence += 0.1;
            }
            
            // If it was previously killed, very high threat
            // (This would require checking kill_actions table, simplified here)
            
            return Ok(confidence.min(1.0));
        }

        // New process - calculate initial confidence
        let mut confidence: f32 = 0.0;

        // Base confidence from CPU abuse
        if cpu_percent > 30.0 {
            confidence += 0.4;
        } else if cpu_percent > 20.0 {
            confidence += 0.3;
        }

        // Increase if duration is long
        if duration_seconds > 600 { // 10 minutes
            confidence += 0.2;
        }

        // Decrease if it's a known safe binary
        if process.binary_path.starts_with("/usr/bin/") 
            || process.binary_path.starts_with("/usr/sbin/")
            || process.binary_path.starts_with("/bin/")
            || process.binary_path.starts_with("/sbin/") {
            confidence *= 0.3; // Reduce but don't eliminate
        }

        // Increase if command line looks suspicious
        if process.command_line.contains("miner")
            || process.command_line.contains("xmrig")
            || process.command_line.contains("crypto") {
            confidence += 0.3;
        }

        // Increase if running from unusual locations
        if process.binary_path.starts_with("/tmp/")
            || process.binary_path.starts_with("/var/tmp/")
            || process.binary_path.contains("/.cache/") {
            confidence += 0.2;
        }

        Ok(confidence.min(1.0f32))
    }

    pub async fn record_suspicious_process(
        &self,
        process: &ProcessInfo,
        cpu_percent: f32,
        duration_seconds: u64,
        confidence: f32,
        first_seen: DateTime<Utc>,
    ) -> Result<()> {
        // Check if this is a restart
        let existing = self.db.get_suspicious_by_binary(&process.binary_path).await?;
        let restart_detected = existing.as_ref()
            .map(|e| e.pid != process.pid)
            .unwrap_or(false);

        let spawn_count = existing.as_ref()
            .map(|e| e.spawn_count)
            .unwrap_or(1);

        let suspicious = SuspiciousProcess {
            pid: process.pid,
            ppid: process.ppid,
            uid: process.uid,
            binary_path: process.binary_path.clone(),
            command_line: process.command_line.clone(),
            cpu_percent,
            duration_seconds,
            threat_confidence: confidence,
            first_seen,
            last_seen: Utc::now(),
            spawn_count,
            restart_detected,
        };

        self.db.upsert_suspicious_process(&suspicious).await?;

        Ok(())
    }
}

