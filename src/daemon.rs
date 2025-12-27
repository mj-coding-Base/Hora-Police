use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::cpu_analyzer::CpuAnalyzer;
use crate::cron_watcher::CronWatcher;
use crate::database::{IntelligenceDB, ProcessRecord, MalwareFile};
use crate::intelligence::BehaviorIntelligence;
use crate::kill_engine::KillEngine;
use crate::npm_scanner::NpmScanner;
use crate::process_monitor::ProcessMonitor;
use crate::react_detector::ReactDetector;
use crate::telegram::TelegramReporter;
use crate::file_scanner::FileScanner;
use crate::file_quarantine::FileQuarantine;
use crate::environment::SystemEnvironment;
use crate::pm2_integration::Pm2Integration;
use crate::systemd_integration::SystemdIntegration;
use crate::nginx_integration::NginxIntegration;
use crate::whitelist::WhitelistManager;
use crate::safe_kill::{SafeKillEngine, SafeKillConfig, KillActionType};
use crate::deploy_detector::DeployDetector;
use crate::file_watcher::FileWatcher;
use crate::zombie_reaper::ZombieReaper;
use sd_notify::NotifyState;

pub struct SentinelDaemon {
    config: Config,
    monitor: ProcessMonitor,
    cpu_analyzer: CpuAnalyzer,
    cron_watcher: CronWatcher,
    npm_scanner: NpmScanner,
    react_detector: ReactDetector,
    db: IntelligenceDB,
    intelligence: BehaviorIntelligence,
    kill_engine: KillEngine, // Keep for backward compatibility, but prefer safe_kill
    safe_kill: Option<SafeKillEngine>,
    telegram: TelegramReporter,
    file_scanner: Option<FileScanner>,
    file_quarantine: Option<FileQuarantine>,
    environment: SystemEnvironment,
    pm2: Pm2Integration,
    systemd: SystemdIntegration,
    nginx: NginxIntegration,
    whitelist: WhitelistManager,
    deploy_detector: DeployDetector,
    file_watcher: Option<FileWatcher>,
    deploy_cleanup_counter: u64,
    db_maintenance_counter: u64,
    zombie_reaper: ZombieReaper,
}

impl SentinelDaemon {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Hora-Police daemon components...");

        // Detect system environment and auto-tune
        let environment = SystemEnvironment::detect()?;
        info!("✅ System environment detected: {} vCPU, {}MB RAM", 
              environment.vcpu_count, environment.total_ram_mb);

        // Initialize database
        let db_path = PathBuf::from(&config.database_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = IntelligenceDB::new(&db_path).await?;
        info!("✅ Database initialized at: {}", config.database_path);

        // Initialize integrations
        let mut pm2 = Pm2Integration::new();
        let mut systemd = SystemdIntegration::new();
        let mut nginx = NginxIntegration::new();

        // Build whitelist from environment
        let whitelist = if config.whitelist.auto_detect {
            WhitelistManager::build_from_environment(
                &mut pm2,
                &mut systemd,
                &mut nginx,
                &config.whitelist.manual_patterns,
            )?
        } else {
            let mut wl = WhitelistManager::new();
            for pattern in &config.whitelist.manual_patterns {
                wl.add_manual_entry(pattern.clone());
            }
            wl
        };
        info!("✅ Whitelist initialized with {} entries", whitelist.get_entries().len());

        // Initialize components
        let monitor = ProcessMonitor::new();
        
        // Auto-tune CPU analyzer
        let cpu_analyzer = if config.auto_tune.enabled {
            CpuAnalyzer::new_with_environment(
                config.cpu_threshold,
                config.duration_minutes,
                &environment,
                config.auto_tune.vcpu_override,
            )
        } else {
            CpuAnalyzer::new(config.cpu_threshold, config.duration_minutes)
        };
        
        let cron_watcher = CronWatcher::new();
        let npm_scanner = NpmScanner::new();
        let react_detector = ReactDetector::new();
        
        let intelligence = BehaviorIntelligence::new(db.clone(), config.learning_mode).await?;
        
        // Keep old kill engine for backward compatibility
        let kill_engine = KillEngine::new(
            db.clone(),
            ProcessMonitor::new(),
            config.auto_kill,
            config.threat_confidence_threshold,
        );
        
        // Initialize safe kill engine
        let safe_kill_config = SafeKillConfig::from(&config);
        let safe_kill = Some(SafeKillEngine::new(
            db.clone(),
            pm2.clone(),
            systemd.clone(),
            nginx.clone(),
            whitelist.clone(),
            safe_kill_config,
        ));
        
        let telegram = TelegramReporter::new(config.telegram.clone(), db.clone());
        
        // Initialize deploy detector
        let deploy_detector = DeployDetector::new(config.deploy_grace_minutes);

        // Initialize file scanner if enabled
        let (file_scanner, file_quarantine, file_watcher) = if config.file_scanning.enabled {
            let scan_paths: Vec<PathBuf> = config.file_scanning.scan_paths
                .iter()
                .map(|p| PathBuf::from(p))
                .collect();
            let quarantine_path = PathBuf::from(&config.file_scanning.quarantine_path);
            
            let scanner = FileScanner::new(scan_paths.clone(), quarantine_path.clone());
            let quarantine = FileQuarantine::new_with_cleanup(
                quarantine_path,
                config.file_scanning.auto_delete,
                config.file_scanning.aggressive_cleanup,
            );
            
            // Initialize file watcher for efficient scanning
            let watcher = FileWatcher::new(scan_paths.clone()).ok();
            if watcher.is_some() {
                info!("✅ File watcher initialized (inotify enabled)");
            }
            
            info!("✅ File scanner initialized (scanning {} paths)", 
                  config.file_scanning.scan_paths.len());
            
            (Some(scanner), Some(quarantine), watcher)
        } else {
            (None, None, None)
        };

        Ok(Self {
            config,
            monitor,
            cpu_analyzer,
            cron_watcher,
            npm_scanner,
            react_detector,
            db,
            intelligence,
            kill_engine,
            safe_kill,
            telegram,
            file_scanner,
            file_quarantine,
            environment,
            pm2,
            systemd,
            nginx,
            whitelist,
            deploy_detector,
            file_watcher,
            deploy_cleanup_counter: 0,
            db_maintenance_counter: 0,
            zombie_reaper: ZombieReaper::new(100), // Alert if > 100 zombies
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("🚀 Hora-Police daemon running. Monitoring started.");

        // Start daily report scheduler if Telegram is configured
        if let Some(telegram_config) = &self.config.telegram {
            let telegram_config_clone = telegram_config.clone();
            let db_clone = self.db.clone();
            tokio::spawn(async move {
                let reporter = TelegramReporter::new(Some(telegram_config_clone), db_clone);
                loop {
                    sleep(Duration::from_secs(86400)).await; // 24 hours
                    if let Err(e) = reporter.send_daily_report().await {
                        error!("Failed to send daily report: {}", e);
                    }
                }
            });
        }

        let mut cron_check_counter = 0u64;
        let cron_check_interval = 60; // Check cron every 60 iterations (5 min at 5s intervals)
        
        let mut file_scan_counter = 0u64;
        let file_scan_interval = if self.config.file_scanning.enabled {
            // Convert minutes to iterations (assuming 5s polling interval)
            (self.config.file_scanning.scan_interval_minutes * 60) / (self.config.polling_interval_ms / 1000)
        } else {
            u64::MAX // Never scan if disabled
        };

        loop {
            // Refresh process information
            self.monitor.refresh();
            
            // Get all processes
            let processes = match self.monitor.get_all_processes() {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to get processes: {}", e);
                    sleep(Duration::from_millis(self.config.polling_interval_ms)).await;
                    continue;
                }
            };

            // Record all processes to database (sampled to reduce overhead)
            for process in &processes {
                if process.cpu_percent > 1.0 { // Only record processes using CPU
                    let record = ProcessRecord {
                        pid: process.pid,
                        ppid: process.ppid,
                        uid: process.uid,
                        binary_path: process.binary_path.clone(),
                        command_line: process.command_line.clone(),
                        cpu_percent: process.cpu_percent,
                        timestamp: Utc::now(),
                    };
                    
                    if let Err(e) = self.db.record_process(&record).await {
                        warn!("Failed to record process: {}", e);
                    }
                }
            }

            // Analyze CPU usage
            let cpu_abuses = self.cpu_analyzer.analyze(&processes);

            for abuse in cpu_abuses {
                if let Some(process) = processes.iter().find(|p| p.pid == abuse.pid) {
                    // Skip system processes
                    if self.kill_engine.is_system_process(&process.binary_path) {
                        continue;
                    }

                    // Check deploy grace period
                    if self.deploy_detector.should_suspend_kill(process) {
                        info!("Suspending kill for PID {} due to recent deployment activity", process.pid);
                        continue;
                    }

                    // Calculate threat confidence
                    let confidence = match self.intelligence.analyze_process(
                        process,
                        abuse.cpu_percent,
                        abuse.duration_seconds,
                        abuse.first_seen,
                    ).await {
                        Ok(c) => c,
                        Err(e) => {
                            error!("Failed to analyze process: {}", e);
                            continue;
                        }
                    };

                    // Record suspicious process
                    if let Err(e) = self.intelligence.record_suspicious_process(
                        process,
                        abuse.cpu_percent,
                        abuse.duration_seconds,
                        confidence,
                        abuse.first_seen,
                    ).await {
                        error!("Failed to record suspicious process: {}", e);
                    }

                    // Check for npm infections
                    let npm_infections = match self.npm_scanner.scan_process(
                        &process.binary_path,
                        &process.command_line,
                    ) {
                        Ok(inf) => inf,
                        Err(e) => {
                            warn!("Failed to scan npm: {}", e);
                            vec![]
                        }
                    };

                    for infection in &npm_infections {
                        let db_infection = crate::database::NpmInfection {
                            id: 0,
                            package_name: infection.package_name.clone(),
                            version: infection.version.clone(),
                            install_scripts: infection.install_scripts.join("; "),
                            binary_path: infection.binary_path.clone(),
                            detected_at: Utc::now(),
                            threat_level: infection.threat_level,
                        };

                        if let Err(e) = self.db.record_npm_infection(&db_infection).await {
                            warn!("Failed to record npm infection: {}", e);
                        }

                        // Increase confidence if npm infection found
                        let adjusted_confidence = (confidence + infection.threat_level * 0.3).min(1.0);
                        
                        if adjusted_confidence >= self.config.threat_confidence_threshold {
                            let reason = format!(
                                "CPU abuse ({}% for {}s) + npm infection: {}",
                                abuse.cpu_percent,
                                abuse.duration_seconds,
                                infection.package_name
                            );

                            // Use safe kill engine if available
                            if let Some(ref mut safe_kill) = self.safe_kill {
                                let action = safe_kill.decide_action(process, adjusted_confidence).await;
                                if let Err(e) = safe_kill.execute_action(action, process, &reason, adjusted_confidence).await {
                                    error!("Failed to execute safe kill action: {}", e);
                                }
                            } else {
                                // Fallback to old kill engine
                                if let Err(e) = self.kill_engine.kill_process(
                                    process.pid,
                                    process.uid,
                                    &process.binary_path,
                                    &reason,
                                    adjusted_confidence,
                                ).await {
                                    error!("Failed to kill process: {}", e);
                                }
                            }

                            // Send real-time alert if enabled
                            if self.config.real_time_alerts {
                                if let Some(telegram_config) = &self.config.telegram {
                                    let alert_msg = format!(
                                        "Killed process PID {} ({})\nReason: {}\nConfidence: {:.0}%",
                                        process.pid,
                                        process.binary_path,
                                        reason,
                                        adjusted_confidence * 100.0
                                    );
                                    let _ = self.telegram.send_alert("Malware Detected", &alert_msg).await;
                                }
                            }
                        }
                    }

                    // Check for React abuse
                    if let Some(react_abuse) = self.react_detector.detect(process, abuse.cpu_percent) {
                        let adjusted_confidence = (confidence + react_abuse.confidence * 0.2).min(1.0);
                        
                        if adjusted_confidence >= self.config.threat_confidence_threshold {
                            let reason = format!(
                                "CPU abuse + React abuse detected: {}",
                                react_abuse.reasons.join(", ")
                            );

                            // Use safe kill engine if available
                            if let Some(ref mut safe_kill) = self.safe_kill {
                                let action = safe_kill.decide_action(process, adjusted_confidence).await;
                                if let Err(e) = safe_kill.execute_action(action, process, &reason, adjusted_confidence).await {
                                    error!("Failed to execute safe kill action: {}", e);
                                }
                            } else {
                                if let Err(e) = self.kill_engine.kill_process(
                                    process.pid,
                                    process.uid,
                                    &process.binary_path,
                                    &reason,
                                    adjusted_confidence,
                                ).await {
                                    error!("Failed to kill process: {}", e);
                                }
                            }
                        }
                    }

                    // Kill if confidence threshold exceeded
                    if confidence >= self.config.threat_confidence_threshold {
                        let reason = format!(
                            "CPU abuse: {}% for {} seconds",
                            abuse.cpu_percent,
                            abuse.duration_seconds
                        );

                        // Use safe kill engine if available
                        if let Some(ref mut safe_kill) = self.safe_kill {
                            let action = safe_kill.decide_action(process, confidence).await;
                            
                            // Send notification if action is Notify
                            if matches!(action, KillActionType::Notify) && self.config.real_time_alerts {
                                if let Some(_) = &self.config.telegram {
                                    let alert_msg = format!(
                                        "Suspicious process detected (not killed due to safety policy):\n\nPID: {}\nBinary: {}\nCPU: {:.1}%\nDuration: {}s\nConfidence: {:.0}%",
                                        process.pid,
                                        process.binary_path,
                                        abuse.cpu_percent,
                                        abuse.duration_seconds,
                                        confidence * 100.0
                                    );
                                    let _ = self.telegram.send_alert("Suspicious Process Detected", &alert_msg).await;
                                }
                            }
                            
                            if let Err(e) = safe_kill.execute_action(action, process, &reason, confidence).await {
                                error!("Failed to execute safe kill action: {}", e);
                            }
                        } else {
                            // Fallback to old kill engine
                            if let Err(e) = self.kill_engine.kill_process(
                                process.pid,
                                process.uid,
                                &process.binary_path,
                                &reason,
                                confidence,
                            ).await {
                                error!("Failed to kill process: {}", e);
                            }
                        }
                    }
                }
            }

            // Periodically check cron jobs
            cron_check_counter += 1;
            if cron_check_counter >= cron_check_interval {
                cron_check_counter = 0;
                
                match self.cron_watcher.scan_all() {
                    Ok(jobs) => {
                        for job in jobs {
                            if job.suspicious {
                                let snapshot = crate::database::CronSnapshot {
                                    id: 0,
                                    file_path: job.file_path.clone(),
                                    content_hash: job.content_hash.clone(),
                                    content: job.content.clone(),
                                    user: job.user.clone(),
                                    detected_at: Utc::now(),
                                    suspicious: true,
                                };

                                if let Err(e) = self.db.record_cron_snapshot(&snapshot).await {
                                    warn!("Failed to record cron snapshot: {}", e);
                                }

                                warn!("⚠️  Suspicious cron job detected: {} (User: {})", 
                                      job.file_path, job.user);
                                
                                if self.config.real_time_alerts {
                                    if let Some(_) = &self.config.telegram {
                                        let alert_msg = format!(
                                            "Suspicious cron job detected:\nFile: {}\nUser: {}\nReasons: {}",
                                            job.file_path,
                                            job.user,
                                            job.suspicious_reasons.join(", ")
                                        );
                                        let _ = self.telegram.send_alert("Suspicious Cron Job", &alert_msg).await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan cron jobs: {}", e);
                    }
                }
            }

            // Periodically scan for malware files
            if self.config.file_scanning.enabled {
                file_scan_counter += 1;
                if file_scan_counter >= file_scan_interval {
                    file_scan_counter = 0;
                    
                    if let (Some(ref scanner), Some(ref quarantine)) = 
                        (&self.file_scanner, &self.file_quarantine) {
                        
                        info!("🔍 Starting file system malware scan...");
                        
                        match scanner.scan_all_paths() {
                            Ok(detected_files) => {
                                if !detected_files.is_empty() {
                                    warn!("🚨 Found {} malicious file(s)!", detected_files.len());
                                    
                                    for malware in detected_files {
                                        // Kill processes using the file if configured
                                        if self.config.file_scanning.kill_processes_using_file {
                                            if let Err(e) = quarantine
                                                .kill_processes_using_file(&malware.file_path)
                                                .await {
                                                warn!("Failed to kill processes using {}: {}", 
                                                      malware.file_path.display(), e);
                                            }
                                        }
                                        
                                        // Generate rollback manifest before cleanup
                                        use crate::rollback::{RollbackManifest, RollbackAction, get_rollback_key};
                                        
                                        let mut rollback_manifest = RollbackManifest::new();
                                        rollback_manifest.add_action(RollbackAction::RestoreFile {
                                            from: format!("{}/{}", 
                                                quarantine.get_quarantine_dir().display(),
                                                malware.file_path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("unknown")),
                                            to: malware.file_path.to_string_lossy().to_string(),
                                        });

                                        // Aggressively clean up malware origin (parent dirs, related files, cron jobs)
                                        let origin_cleanup = if self.config.file_scanning.aggressive_cleanup && !self.config.dry_run {
                                            match quarantine.delete_malware_origin(&malware.file_path) {
                                                Ok(result) => {
                                                    if !result.is_empty() {
                                                        info!("🧹 Cleaned malware origin: {} files, {} dirs, {} cron jobs",
                                                              result.deleted_files.len(),
                                                              result.deleted_directories.len(),
                                                              result.cleaned_cron_jobs.len());
                                                        
                                                        // Add rollback actions for deleted files/dirs
                                                        for file in &result.deleted_files {
                                                            rollback_manifest.add_action(RollbackAction::RestoreFile {
                                                                from: format!("{}/{}", 
                                                                    quarantine.get_quarantine_dir().display(),
                                                                    PathBuf::from(file).file_name()
                                                                        .and_then(|n| n.to_str())
                                                                        .unwrap_or("unknown")),
                                                                to: file.clone(),
                                                            });
                                                        }
                                                    }
                                                    Some(result)
                                                }
                                                Err(e) => {
                                                    warn!("Failed to clean malware origin: {}", e);
                                                    None
                                                }
                                            }
                                        } else {
                                            None
                                        };
                                        
                                        // Sign and save rollback manifest
                                        if let Ok(key) = get_rollback_key() {
                                            if let Err(e) = rollback_manifest.sign(&key) {
                                                warn!("Failed to sign rollback manifest: {}", e);
                                            }
                                            
                                            let manifest_path = PathBuf::from("/var/lib/hora-police/rollbacks")
                                                .join(format!("malware_{}_{}.rollback",
                                                    Utc::now().format("%Y%m%d_%H%M%S"),
                                                    malware.file_path.file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or("unknown")));
                                            
                                            if let Some(parent) = manifest_path.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            
                                            if let Err(e) = rollback_manifest.save(&manifest_path) {
                                                warn!("Failed to save rollback manifest: {}", e);
                                            }
                                        }

                                        // Quarantine or delete the file
                                        let action_result = match quarantine.handle_malware(&malware.file_path) {
                                            Ok(result) => result,
                                            Err(e) => {
                                                error!("Failed to handle malware file {}: {}", 
                                                      malware.file_path.display(), e);
                                                continue;
                                            }
                                        };
                                        
                                        // Record in database
                                        let db_malware = MalwareFile {
                                            id: 0,
                                            file_path: malware.file_path.to_string_lossy().to_string(),
                                            file_hash: malware.file_hash.clone(),
                                            file_size: malware.file_size as i64,
                                            signature_name: malware.signature.name.clone(),
                                            threat_level: malware.signature.threat_level,
                                            action_taken: match action_result {
                                                crate::file_quarantine::QuarantineResult::Quarantined(_) => 
                                                    "quarantined".to_string(),
                                                crate::file_quarantine::QuarantineResult::Deleted => 
                                                    "deleted".to_string(),
                                            },
                                            quarantine_path: match action_result {
                                                crate::file_quarantine::QuarantineResult::Quarantined(ref path) => 
                                                    Some(path.to_string_lossy().to_string()),
                                                crate::file_quarantine::QuarantineResult::Deleted => None,
                                            },
                                            detected_at: malware.detected_at,
                                        };
                                        
                                        if let Err(e) = self.db.record_malware_file(&db_malware).await {
                                            error!("Failed to record malware file: {}", e);
                                        }
                                        
                                        // Send alert if enabled
                                        if self.config.real_time_alerts {
                                            if let Some(_) = &self.config.telegram {
                                                let action_str = match action_result {
                                                    crate::file_quarantine::QuarantineResult::Quarantined(ref p) => 
                                                        format!("Quarantined to: {}", p.display()),
                                                    crate::file_quarantine::QuarantineResult::Deleted => 
                                                        "Deleted".to_string(),
                                                };
                                                
                                                let mut alert_msg = format!(
                                                    "Malware file detected and {}!\n\nFile: {}\nSignature: {}\nThreat Level: {:.0}%\nHash: {}",
                                                    action_str,
                                                    malware.file_path.display(),
                                                    malware.signature.name,
                                                    malware.signature.threat_level * 100.0,
                                                    &malware.file_hash[..16] // First 16 chars of hash
                                                );
                                                
                                                // Add origin cleanup info if available
                                                if let Some(ref cleanup) = origin_cleanup {
                                                    if !cleanup.is_empty() {
                                                        alert_msg.push_str(&format!(
                                                            "\n\n🧹 Origin Cleanup:\n- Deleted {} related files\n- Removed {} directories\n- Cleaned {} cron jobs",
                                                            cleanup.deleted_files.len(),
                                                            cleanup.deleted_directories.len(),
                                                            cleanup.cleaned_cron_jobs.len()
                                                        ));
                                                    }
                                                }
                                                
                                                let _ = self.telegram
                                                    .send_alert("Malware File Detected", &alert_msg)
                                                    .await;
                                            }
                                        }
                                    }
                                } else {
                                    info!("✅ File scan complete - no malware detected");
                                }
                            }
                            Err(e) => {
                                error!("File scan failed: {}", e);
                            }
                        }
                    }
                }
            }

            // Cleanup old deploy records periodically
            self.deploy_cleanup_counter += 1;
            if self.deploy_cleanup_counter >= 360 { // Every 30 minutes (360 * 5s)
                self.deploy_cleanup_counter = 0;
                self.deploy_detector.cleanup_old_records();
            }

            // Database retention and vacuum (daily)
            self.db_maintenance_counter += 1;
            if self.db_maintenance_counter >= 17280 { // Every 24 hours (17280 * 5s)
                self.db_maintenance_counter = 0;
                if let Err(e) = self.db.archive_old_records(30).await {
                    warn!("Failed to archive old records: {}", e);
                }
                if let Err(e) = self.db.vacuum_database().await {
                    warn!("Failed to vacuum database: {}", e);
                }
            }

            // Auto-tune polling interval based on load
            let polling_interval = if self.config.auto_tune.enabled {
                self.environment.compute_polling_interval_ms(self.config.polling_interval_ms)
            } else {
                self.config.polling_interval_ms
            };

            // Sleep before next iteration
            sleep(Duration::from_millis(polling_interval)).await;
        }
    }
}


