use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePool, Row};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ProcessRecord {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub binary_path: String,
    pub command_line: String,
    pub cpu_percent: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SuspiciousProcess {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub binary_path: String,
    pub command_line: String,
    pub cpu_percent: f32,
    pub duration_seconds: u64,
    pub threat_confidence: f32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub spawn_count: i32,
    pub restart_detected: bool,
}

#[derive(Debug, Clone)]
pub struct CronSnapshot {
    pub id: i64,
    pub file_path: String,
    pub content_hash: String,
    pub content: String,
    pub user: String,
    pub detected_at: DateTime<Utc>,
    pub suspicious: bool,
}

#[derive(Debug, Clone)]
pub struct NpmInfection {
    pub id: i64,
    pub package_name: String,
    pub version: String,
    pub install_scripts: String,
    pub binary_path: String,
    pub detected_at: DateTime<Utc>,
    pub threat_level: f32,
}

#[derive(Debug, Clone)]
pub struct KillAction {
    pub id: i64,
    pub pid: i32,
    pub uid: u32,
    pub binary_path: String,
    pub reason: String,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MalwareFile {
    pub id: i64,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub signature_name: String,
    pub threat_level: f32,
    pub action_taken: String, // "quarantined" or "deleted"
    pub quarantine_path: Option<String>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct IntelligenceDB {
    pool: Arc<SqlitePool>,
}

impl IntelligenceDB {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_url = format!("sqlite:{}", db_path.as_ref().display());
        let pool = SqlitePool::connect(&db_url).await?;
        
        let db = Self { pool: Arc::new(pool) };
        db.init_schema().await?;
        
        Ok(db)
    }

    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS process_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pid INTEGER NOT NULL,
                ppid INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                binary_path TEXT NOT NULL,
                command_line TEXT NOT NULL,
                cpu_percent REAL NOT NULL,
                timestamp DATETIME NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_process_pid ON process_history(pid);
            CREATE INDEX IF NOT EXISTS idx_process_timestamp ON process_history(timestamp);
            CREATE INDEX IF NOT EXISTS idx_process_uid ON process_history(uid);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS suspicious_processes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pid INTEGER NOT NULL,
                ppid INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                binary_path TEXT NOT NULL,
                command_line TEXT NOT NULL,
                cpu_percent REAL NOT NULL,
                duration_seconds INTEGER NOT NULL,
                threat_confidence REAL NOT NULL,
                first_seen DATETIME NOT NULL,
                last_seen DATETIME NOT NULL,
                spawn_count INTEGER DEFAULT 1,
                restart_detected BOOLEAN DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_suspicious_binary ON suspicious_processes(binary_path);
            CREATE INDEX IF NOT EXISTS idx_suspicious_confidence ON suspicious_processes(threat_confidence);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cron_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                content TEXT NOT NULL,
                user TEXT NOT NULL,
                detected_at DATETIME NOT NULL,
                suspicious BOOLEAN DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS npm_infections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                package_name TEXT NOT NULL,
                version TEXT NOT NULL,
                install_scripts TEXT NOT NULL,
                binary_path TEXT NOT NULL,
                detected_at DATETIME NOT NULL,
                threat_level REAL NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS kill_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pid INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                binary_path TEXT NOT NULL,
                reason TEXT NOT NULL,
                confidence REAL NOT NULL,
                timestamp DATETIME NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_kill_timestamp ON kill_actions(timestamp);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS malware_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                signature_name TEXT NOT NULL,
                threat_level REAL NOT NULL,
                action_taken TEXT NOT NULL,
                quarantine_path TEXT,
                detected_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_malware_file_path ON malware_files(file_path);
            CREATE INDEX IF NOT EXISTS idx_malware_hash ON malware_files(file_hash);
            CREATE INDEX IF NOT EXISTS idx_malware_timestamp ON malware_files(detected_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_process(&self, record: &ProcessRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO process_history (pid, ppid, uid, binary_path, command_line, cpu_percent, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.pid)
        .bind(record.ppid)
        .bind(record.uid as i64)
        .bind(&record.binary_path)
        .bind(&record.command_line)
        .bind(record.cpu_percent)
        .bind(record.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_suspicious_process(&self, process: &SuspiciousProcess) -> Result<()> {
        // Check if process with same binary_path exists
        let existing: Option<(i64, i32, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, spawn_count, first_seen
            FROM suspicious_processes
            WHERE binary_path = ? AND pid = ?
            LIMIT 1
            "#,
        )
        .bind(&process.binary_path)
        .bind(process.pid)
        .try_map(|row: sqlx::sqlite::SqliteRow| {
            Ok((
                row.get(0),
                row.get(1),
                row.get(2),
            ))
        })
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, old_spawn_count, first_seen)) = existing {
            // Update existing record
            sqlx::query(
                r#"
                UPDATE suspicious_processes
                SET cpu_percent = ?, duration_seconds = ?, threat_confidence = ?,
                    last_seen = ?, spawn_count = ?, restart_detected = ?
                WHERE id = ?
                "#,
            )
            .bind(process.cpu_percent)
            .bind(process.duration_seconds as i64)
            .bind(process.threat_confidence)
            .bind(process.last_seen)
            .bind(old_spawn_count + 1)
            .bind(process.restart_detected)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            // Insert new record
            sqlx::query(
                r#"
                INSERT INTO suspicious_processes 
                (pid, ppid, uid, binary_path, command_line, cpu_percent, duration_seconds,
                 threat_confidence, first_seen, last_seen, spawn_count, restart_detected)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(process.pid)
            .bind(process.ppid)
            .bind(process.uid as i64)
            .bind(&process.binary_path)
            .bind(&process.command_line)
            .bind(process.cpu_percent)
            .bind(process.duration_seconds as i64)
            .bind(process.threat_confidence)
            .bind(process.first_seen)
            .bind(process.last_seen)
            .bind(process.spawn_count)
            .bind(process.restart_detected)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_suspicious_by_binary(&self, binary_path: &str) -> Result<Option<SuspiciousProcess>> {
        let row = sqlx::query(
            r#"
            SELECT pid, ppid, uid, binary_path, command_line, cpu_percent, duration_seconds,
                   threat_confidence, first_seen, last_seen, spawn_count, restart_detected
            FROM suspicious_processes
            WHERE binary_path = ?
            ORDER BY last_seen DESC
            LIMIT 1
            "#,
        )
        .bind(binary_path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(SuspiciousProcess {
                pid: row.get(0),
                ppid: row.get(1),
                uid: row.get(2),
                binary_path: row.get(3),
                command_line: row.get(4),
                cpu_percent: row.get(5),
                duration_seconds: row.get(6) as u64,
                threat_confidence: row.get(7),
                first_seen: row.get(8),
                last_seen: row.get(9),
                spawn_count: row.get(10),
                restart_detected: row.get(11),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn record_cron_snapshot(&self, snapshot: &CronSnapshot) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cron_snapshots (file_path, content_hash, content, user, detected_at, suspicious)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&snapshot.file_path)
        .bind(&snapshot.content_hash)
        .bind(&snapshot.content)
        .bind(&snapshot.user)
        .bind(snapshot.detected_at)
        .bind(snapshot.suspicious)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_npm_infection(&self, infection: &NpmInfection) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO npm_infections (package_name, version, install_scripts, binary_path, detected_at, threat_level)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&infection.package_name)
        .bind(&infection.version)
        .bind(&infection.install_scripts)
        .bind(&infection.binary_path)
        .bind(infection.detected_at)
        .bind(infection.threat_level)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_kill_action(&self, action: &KillAction) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO kill_actions (pid, uid, binary_path, reason, confidence, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(action.pid)
        .bind(action.uid as i64)
        .bind(&action.binary_path)
        .bind(&action.reason)
        .bind(action.confidence)
        .bind(action.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_malware_file(&self, malware: &MalwareFile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO malware_files 
            (file_path, file_hash, file_size, signature_name, threat_level, action_taken, quarantine_path, detected_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&malware.file_path)
        .bind(&malware.file_hash)
        .bind(malware.file_size)
        .bind(&malware.signature_name)
        .bind(malware.threat_level)
        .bind(&malware.action_taken)
        .bind(&malware.quarantine_path)
        .bind(malware.detected_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_daily_summary(&self, since: DateTime<Utc>) -> Result<DailySummary> {
        let killed_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM kill_actions WHERE timestamp >= ?
            "#,
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        let suspicious_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT binary_path) FROM suspicious_processes WHERE last_seen >= ?
            "#,
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        let npm_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM npm_infections WHERE detected_at >= ?
            "#,
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        let recent_kills: Vec<KillAction> = sqlx::query(
            r#"
            SELECT pid, uid, binary_path, reason, confidence, timestamp
            FROM kill_actions
            WHERE timestamp >= ?
            ORDER BY timestamp DESC
            LIMIT 20
            "#,
        )
        .bind(since)
        .try_map(|row: sqlx::sqlite::SqliteRow| {
            Ok(KillAction {
                id: 0,
                pid: row.get(0),
                uid: row.get(1),
                binary_path: row.get(2),
                reason: row.get(3),
                confidence: row.get(4),
                timestamp: row.get(5),
            })
        })
        .fetch_all(&self.pool)
        .await?;

        Ok(DailySummary {
            killed_count: killed_count as u64,
            suspicious_processes: suspicious_count as u64,
            npm_infections: npm_count as u64,
            malware_files: malware_files_count as u64,
            recent_kills,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DailySummary {
    pub killed_count: u64,
    pub suspicious_processes: u64,
    pub npm_infections: u64,
    pub malware_files: u64,
    pub recent_kills: Vec<KillAction>,
}

