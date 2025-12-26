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
    
    // New safety options
    #[serde(default = "default_false")]
    pub dry_run: bool,
    
    #[serde(default = "default_false")]
    pub canary_mode: bool,
    
    #[serde(default = "default_false")]
    pub audit_only: bool,
    
    #[serde(default = "default_deploy_grace")]
    pub deploy_grace_minutes: u64,
    
    #[serde(default = "default_high_threshold")]
    pub high_confidence_threshold: f32,  // For systemd/pm2 escalation
    
    #[serde(default)]
    pub auto_tune: AutoTuneConfig,
    
    #[serde(default)]
    pub whitelist: WhitelistConfig,
    
    #[serde(default = "default_true")]
    pub adaptive_polling: bool,
    
    #[serde(default = "default_adaptive_load_factor")]
    pub adaptive_polling_load_factor: f64,
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

fn default_false() -> bool {
    false
}

fn default_deploy_grace() -> u64 {
    10
}

fn default_high_threshold() -> f32 {
    0.95
}

fn default_adaptive_load_factor() -> f64 {
    1.5
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTuneConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub vcpu_override: Option<usize>,
    pub ram_override_mb: Option<u64>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistConfig {
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    #[serde(default)]
    pub manual_patterns: Vec<String>,
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
            dry_run: false,
            canary_mode: false,
            audit_only: false,
            deploy_grace_minutes: 10,
            high_confidence_threshold: 0.95,
            auto_tune: AutoTuneConfig {
                enabled: true,
                vcpu_override: None,
                ram_override_mb: None,
            },
            whitelist: WhitelistConfig {
                auto_detect: true,
                manual_patterns: Vec::new(),
            },
            adaptive_polling: true,
            adaptive_polling_load_factor: 1.5,
        }
    }
}

