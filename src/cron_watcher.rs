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
            Regex::new(r"echo\s+['\"]?[A-Za-z0-9+/=]{50,}['\"]?\s*\||base64\s+-d").unwrap(),
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
}

