use anyhow::Result;
use chrono::{DateTime, Utc, NaiveTime};
use crate::config::TelegramConfig;
use crate::database::{IntelligenceDB, DailySummary};

pub struct TelegramReporter {
    config: Option<TelegramConfig>,
    client: reqwest::Client,
    db: IntelligenceDB,
}

impl TelegramReporter {
    pub fn new(config: Option<TelegramConfig>, db: IntelligenceDB) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            db,
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<()> {
        let config = match &self.config {
            Some(c) => c,
            None => return Ok(()), // No config, skip
        };

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            config.bot_token
        );

        let payload = serde_json::json!({
            "chat_id": config.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Telegram API error: {}", text));
        }

        Ok(())
    }

    pub async fn send_daily_report(&self) -> Result<()> {
        let yesterday = Utc::now() - chrono::Duration::hours(24);
        let summary = self.db.get_daily_summary(yesterday).await?;

        let mut message = format!(
            "🛡️ *Sentinel Daily Report*\n\n\
            *Summary:*\n\
            • Processes Killed: {}\n\
            • Suspicious Processes: {}\n\
            • npm Infections: {}\n\
            • Malware Files Detected: {}\n\n",
            summary.killed_count,
            summary.suspicious_processes,
            summary.npm_infections,
            summary.malware_files
        );

        if !summary.recent_kills.is_empty() {
            message.push_str("*Recent Actions:*\n");
            for kill in summary.recent_kills.iter().take(10) {
                message.push_str(&format!(
                    "• PID {} ({}) - {:.0}% confidence\n  Reason: {}\n",
                    kill.pid,
                    kill.binary_path,
                    kill.confidence * 100.0,
                    kill.reason
                ));
            }
        }

        self.send_message(&message).await?;
        Ok(())
    }

    pub async fn send_alert(&self, title: &str, message: &str) -> Result<()> {
        let full_message = format!("🚨 *{}*\n\n{}", title, message);
        self.send_message(&full_message).await?;
        Ok(())
    }

    pub fn get_daily_report_time(&self) -> Option<NaiveTime> {
        self.config.as_ref()
            .and_then(|c| NaiveTime::parse_from_str(&c.daily_report_time, "%H:%M").ok())
    }

    pub async fn schedule_daily_report(&self) -> Result<tokio::task::JoinHandle<()>> {
        let reporter = self.clone_for_task();
        let report_time = self.get_daily_report_time()
            .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());

        let handle = tokio::spawn(async move {
            loop {
                let now = Utc::now().time();
                let target = report_time;
                
                // Calculate time until next report
                let wait_seconds = if now < target {
                    (target - now).num_seconds()
                } else {
                    (target - now + chrono::Duration::days(1)).num_seconds()
                };

                // Wait until report time
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds as u64)).await;

                // Send report
                if let Err(e) = reporter.send_daily_report().await {
                    tracing::error!("Failed to send daily report: {}", e);
                }

                // Wait 24 hours for next report
                tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;
            }
        });

        Ok(handle)
    }

    pub fn clone_for_task(&self) -> Self {
        // Create a new instance for the async task
        Self {
            config: self.config.clone(),
            client: reqwest::Client::new(),
            db: self.db.clone(), // Now properly cloneable via Arc
        }
    }
}

