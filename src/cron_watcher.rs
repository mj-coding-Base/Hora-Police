use anyhow::Result;
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct CronJob {
    pub file_path: String,
    pub content: String,
    pub content_hash: String,
    pub user: String,
    pub suspicious: bool,
    pub suspicious_reasons: Vec<String>,
}

pub struct CronWatcher {
    suspicious_patterns: Vec<Regex>,
    last_snapshots: std::collections::HashMap<String, String>, // (file_path, hash)
}

impl CronWatcher {
    pub fn new() -> Self {
        let suspicious_patterns = vec![
            // Base64 encoded commands
            Regex::new(r#"echo\s+['"]?[A-Za-z0-9+/=]{50,}['"]?\s*\||base64\s+-d"#).unwrap(),
            // curl | wget | bash patterns
            Regex::new(r"(curl|wget)\s+.*\s*\|\s*(bash|sh|zsh)").unwrap(),
            // npm install at runtime
            Regex::new(r"npm\s+install.*\s+&&").unwrap(),
            // Obfuscated commands
            Regex::new(r"\$\{?[A-Z_]+\}?.*\|\s*(bash|sh)").unwrap(),
            // Suspicious URL patterns
            Regex::new(r"(curl|wget)\s+-[^s]*s[^s]*\s+https?://[^\s]+").unwrap(),
        ];

        Self {
            suspicious_patterns,
            last_snapshots: std::collections::HashMap::new(),
        }
    }

    pub fn scan_all(&mut self) -> Result<Vec<CronJob>> {
        let mut jobs = Vec::new();

        // Scan /etc/crontab
        if Path::new("/etc/crontab").exists() {
            if let Ok(job) = self.scan_file("/etc/crontab", "root") {
                jobs.push(job);
            }
        }

        // Scan /etc/cron.d/*
        if let Ok(entries) = fs::read_dir("/etc/cron.d") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(job) = self.scan_file(
                        path.to_str().unwrap(),
                        "root",
                    ) {
                        jobs.push(job);
                    }
                }
            }
        }

        // Scan /etc/cron.hourly, /etc/cron.daily, etc.
        for dir in &["/etc/cron.hourly", "/etc/cron.daily", "/etc/cron.weekly", "/etc/cron.monthly"] {
            if Path::new(dir).exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Ok(job) = self.scan_file(
                                path.to_str().unwrap(),
                                "root",
                            ) {
                                jobs.push(job);
                            }
                        }
                    }
                }
            }
        }

        // Scan user crontabs
        if Path::new("/var/spool/cron/crontabs").exists() {
            if let Ok(entries) = fs::read_dir("/var/spool/cron/crontabs") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let username = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        if let Ok(job) = self.scan_file(
                            path.to_str().unwrap(),
                            &username,
                        ) {
                            jobs.push(job);
                        }
                    }
                }
            }
        }

        Ok(jobs)
    }

    fn scan_file(&mut self, file_path: &str, user: &str) -> Result<CronJob> {
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| String::new());
        
        let content_hash = self.hash_content(&content);
        
        // Check if this is a new or changed file
        let is_new = self.last_snapshots
            .get(file_path)
            .map(|old_hash| old_hash != &content_hash)
            .unwrap_or(true);

        if is_new {
            self.last_snapshots.insert(file_path.to_string(), content_hash.clone());
        }

        let mut suspicious = false;
        let mut reasons = Vec::new();

        // Check for suspicious patterns
        for pattern in &self.suspicious_patterns {
            if pattern.is_match(&content) {
                suspicious = true;
                reasons.push(format!("Matches pattern: {}", pattern.as_str()));
            }
        }

        // Check for base64-like strings
        if content.contains("base64") && content.len() > 200 {
            suspicious = true;
            reasons.push("Contains base64 decoding".to_string());
        }

        // Check for npm install
        if content.contains("npm install") && !content.contains("npm ci") {
            suspicious = true;
            reasons.push("Contains npm install (potential supply-chain risk)".to_string());
        }

        Ok(CronJob {
            file_path: file_path.to_string(),
            content: content.clone(),
            content_hash,
            user: user.to_string(),
            suspicious,
            suspicious_reasons: reasons,
        })
    }

    fn hash_content(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn has_changes(&self) -> bool {
        // This is checked during scan_all by comparing hashes
        true
    }

    /// Safely remove cron entry with backup and rollback manifest
    pub async fn remove_cron_safely(
        &self,
        cron_file: &str,
        malicious_content: &str,
        user: &str,
        dry_run: bool,
    ) -> Result<Option<crate::rollback::RollbackManifest>> {
        use anyhow::Context;
        use chrono::Utc;
        use std::path::PathBuf;

        // Read current cron file
        let current_content = fs::read_to_string(cron_file)
            .with_context(|| format!("Failed to read cron file: {}", cron_file))?;

        // Check if malicious content exists
        if !current_content.contains(malicious_content) {
            return Ok(None); // Already removed or not present
        }

        // Create backup
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("{}.backup.{}", cron_file, timestamp);
        fs::copy(cron_file, &backup_path)
            .with_context(|| format!("Failed to create backup: {}", backup_path))?;

        // Generate rollback manifest
        let mut manifest = crate::rollback::RollbackManifest::new();
        manifest.add_action(crate::rollback::RollbackAction::RestoreCron {
            user: user.to_string(),
            content: current_content.clone(),
            file: cron_file.to_string(),
        });

        // Sign manifest
        if let Ok(key) = crate::rollback::get_rollback_key() {
            manifest.sign(&key)?;
        }

        if dry_run {
            info!("[DRY RUN] Would remove malicious cron entry from {} (backup: {})", 
                  cron_file, backup_path);
            return Ok(Some(manifest));
        }

        // Remove lines containing malicious content
        let lines: Vec<&str> = current_content
            .lines()
            .filter(|line| !line.contains(malicious_content) && !line.trim().is_empty())
            .collect();

        // Write to temp file first
        let temp_file = format!("{}.tmp", cron_file);
        let new_content = if lines.is_empty() {
            // If file would be empty, we might want to keep a comment
            format!("# Cron file cleaned by Hora-Police at {}\n", Utc::now().to_rfc3339())
        } else {
            lines.join("\n") + "\n"
        };

        fs::write(&temp_file, new_content)
            .with_context(|| format!("Failed to write temp cron file: {}", temp_file))?;

        // Atomic rename
        fs::rename(&temp_file, cron_file)
            .with_context(|| format!("Failed to rename temp file to cron file: {}", cron_file))?;

        info!("Removed malicious cron entry from {} (backup: {})", cron_file, backup_path);

        // Save rollback manifest
        let manifest_path = PathBuf::from("/var/lib/hora-police/rollbacks")
            .join(format!("cron_{}_{}.rollback", 
                  PathBuf::from(cron_file).file_name()
                      .and_then(|n| n.to_str())
                      .unwrap_or("unknown"),
                  timestamp));
        
        if let Some(parent) = manifest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        manifest.save(&manifest_path)?;

        Ok(Some(manifest))
    }
}

