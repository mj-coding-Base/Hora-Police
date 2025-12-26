use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cpu_threshold: f32,
    pub duration_minutes: u64,
    pub real_time_alerts: bool,
    pub auto_kill: bool,
    pub learning_mode: bool,
    pub database_path: String,
    pub telegram: Option<TelegramConfig>,
    pub polling_interval_ms: u64,
    pub threat_confidence_threshold: f32,
    #[serde(default = "default_file_scanning")]
    pub file_scanning: FileScanningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScanningConfig {
    pub enabled: bool,
    pub scan_interval_minutes: u64,
    pub scan_paths: Vec<String>,
    pub quarantine_path: String,
    pub auto_delete: bool,
    pub kill_processes_using_file: bool,
    #[serde(default = "default_aggressive_cleanup")]
    pub aggressive_cleanup: bool,
}

fn default_aggressive_cleanup() -> bool {
    true
}

fn default_file_scanning() -> FileScanningConfig {
    FileScanningConfig {
        enabled: true,
        scan_interval_minutes: 15,
        scan_paths: vec![
            "/home".to_string(),
            "/tmp".to_string(),
            "/var/tmp".to_string(),
        ],
        quarantine_path: "/var/lib/hora-police/quarantine".to_string(),
        auto_delete: false,
        kill_processes_using_file: true,
        aggressive_cleanup: true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub daily_report_time: String, // HH:MM format
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config from {:?}", path.as_ref()))?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config TOML")?;
        
        Ok(config)
    }

    pub fn default() -> Self {
        Self {
            cpu_threshold: 20.0,
            duration_minutes: 5,
            real_time_alerts: false,
            auto_kill: true,
            learning_mode: true,
            database_path: "/var/lib/hora-police/intelligence.db".to_string(),
            telegram: None,
            polling_interval_ms: 5000, // 5 seconds
            threat_confidence_threshold: 0.7,
            file_scanning: default_file_scanning(),
        }
    }
}

