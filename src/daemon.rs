use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::cpu_analyzer::CpuAnalyzer;
use crate::cron_watcher::CronWatcher;
use crate::database::{IntelligenceDB, ProcessRecord};
use crate::intelligence::BehaviorIntelligence;
use crate::kill_engine::KillEngine;
use crate::npm_scanner::NpmScanner;
use crate::process_monitor::ProcessMonitor;
use crate::react_detector::ReactDetector;
use crate::telegram::TelegramReporter;

pub struct SentinelDaemon {
    config: Config,
    monitor: ProcessMonitor,
    cpu_analyzer: CpuAnalyzer,
    cron_watcher: CronWatcher,
    npm_scanner: NpmScanner,
    react_detector: ReactDetector,
    db: IntelligenceDB,
    intelligence: BehaviorIntelligence,
    kill_engine: KillEngine,
    telegram: TelegramReporter,
}

impl SentinelDaemon {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Sentinel daemon components...");

        // Initialize database
        let db_path = PathBuf::from(&config.database_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = IntelligenceDB::new(&db_path).await?;
        info!("✅ Database initialized at: {}", config.database_path);

        // Initialize components
        let monitor = ProcessMonitor::new();
        let cpu_analyzer = CpuAnalyzer::new(config.cpu_threshold, config.duration_minutes);
        let cron_watcher = CronWatcher::new();
        let npm_scanner = NpmScanner::new();
        let react_detector = ReactDetector::new();
        
        let intelligence = BehaviorIntelligence::new(db.clone(), config.learning_mode).await?;
        let kill_engine = KillEngine::new(
            db.clone(),
            ProcessMonitor::new(), // Create new monitor for kill engine
            config.auto_kill,
            config.threat_confidence_threshold,
        );
        
        let telegram = TelegramReporter::new(config.telegram.clone(), db.clone());

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
            telegram,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("🚀 Sentinel daemon running. Monitoring started.");

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

                            if let Err(e) = self.kill_engine.kill_process(
                                process.pid,
                                process.uid,
                                &process.binary_path,
                                &reason,
                                adjusted_confidence,
                            ).await {
                                error!("Failed to kill process: {}", e);
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

                    // Kill if confidence threshold exceeded
                    if confidence >= self.config.threat_confidence_threshold {
                        let reason = format!(
                            "CPU abuse: {}% for {} seconds",
                            abuse.cpu_percent,
                            abuse.duration_seconds
                        );

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

            // Sleep before next iteration
            sleep(Duration::from_millis(self.config.polling_interval_ms)).await;
        }
    }
}


