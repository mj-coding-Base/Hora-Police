use anyhow::Result;
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use walkdir::WalkDir;
use regex::Regex;
use tracing::{info, warn, error};
use std::sync::Arc;
use crate::database::IntelligenceDB;
use crate::config::FileScanningConfig;

#[derive(Debug, Clone)]
pub struct MalwareSignature {
    pub name: String,
    pub file_name_pattern: Option<Regex>,
    pub path_pattern: Option<Regex>,
    pub file_hash: Option<String>, // SHA256 hash
    pub threat_level: f32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DetectedMalware {
    pub file_path: PathBuf,
    pub signature: MalwareSignature,
    pub file_hash: String,
    pub file_size: u64,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

pub struct FileScanner {
    signatures: Vec<MalwareSignature>,
    scan_paths: Vec<PathBuf>,
    quarantine_path: PathBuf,
    db: Option<Arc<IntelligenceDB>>,
    config: FileScanningConfig,
}

impl FileScanner {
    pub fn new(scan_paths: Vec<PathBuf>, quarantine_path: PathBuf) -> Self {
        Self::new_with_config(scan_paths, quarantine_path, None, FileScanningConfig {
            enabled: true,
            scan_interval_minutes: 15,
            scan_paths: Vec::new(),
            quarantine_path: String::new(),
            auto_delete: false,
            kill_processes_using_file: true,
            aggressive_cleanup: true,
            use_hash_cache: true,
            incremental_scan: true,
            parallel_scan: true,
            max_scan_threads: 4,
        })
    }

    pub fn new_with_config(
        scan_paths: Vec<PathBuf>,
        quarantine_path: PathBuf,
        db: Option<Arc<IntelligenceDB>>,
        config: FileScanningConfig,
    ) -> Self {
        let mut scanner = Self {
            signatures: Vec::new(),
            scan_paths,
            quarantine_path,
            db,
            config,
        };
        
        // Load built-in malware signatures
        scanner.load_builtin_signatures();
        
        scanner
    }

    fn load_builtin_signatures(&mut self) {
        // Load the specific malware signatures from the user's list
        let builtin_signatures = vec![
            MalwareSignature {
                name: "solrz".to_string(),
                file_name_pattern: Some(Regex::new(r"^solrz$").unwrap()),
                path_pattern: Some(Regex::new(r".*[/\\]solrz$").unwrap()),
                file_hash: None,
                threat_level: 1.0,
                description: "Malicious file: solrz".to_string(),
            },
            MalwareSignature {
                name: "e386".to_string(),
                file_name_pattern: Some(Regex::new(r"^e386$").unwrap()),
                path_pattern: Some(Regex::new(r".*[/\\]e386$").unwrap()),
                file_hash: None,
                threat_level: 1.0,
                description: "Malicious file: e386".to_string(),
            },
            MalwareSignature {
                name: "payload.so".to_string(),
                file_name_pattern: Some(Regex::new(r"^payload\.so$").unwrap()),
                path_pattern: Some(Regex::new(r".*[/\\]payload\.so$").unwrap()),
                file_hash: None,
                threat_level: 1.0,
                description: "Malicious shared library: payload.so".to_string(),
            },
            MalwareSignature {
                name: "next".to_string(),
                file_name_pattern: Some(Regex::new(r"^next$").unwrap()),
                path_pattern: Some(Regex::new(r".*[/\\]\.local[/\\]share[/\\]next$").unwrap()),
                file_hash: None,
                threat_level: 1.0,
                description: "Malicious file: next".to_string(),
            },
            // Additional common malware patterns
            MalwareSignature {
                name: "crypto_miner_pattern".to_string(),
                file_name_pattern: Some(Regex::new(r"(?i)(miner|mining|xmrig|ccminer|cpuminer)").unwrap()),
                path_pattern: None,
                file_hash: None,
                threat_level: 0.9,
                description: "Potential crypto miner binary".to_string(),
            },
            MalwareSignature {
                name: "suspicious_so_pattern".to_string(),
                file_name_pattern: Some(Regex::new(r"\.so$").unwrap()),
                path_pattern: Some(Regex::new(r"(?i)(tmp|/tmp|/var/tmp|/dev/shm|payload|malicious|evil)").unwrap()),
                file_hash: None,
                threat_level: 0.8,
                description: "Suspicious shared library location".to_string(),
            },
        ];

        self.signatures.extend(builtin_signatures);
        info!("Loaded {} malware signatures", self.signatures.len());
    }

    pub fn add_signature(&mut self, signature: MalwareSignature) {
        self.signatures.push(signature);
    }

    pub async fn scan_file(&self, file_path: &Path) -> Result<Option<DetectedMalware>> {
        // Check if file exists and is readable
        if !file_path.exists() || !file_path.is_file() {
            return Ok(None);
        }

        // Get file metadata
        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let file_path_str = file_path.to_string_lossy().to_string();

        // Get modification time for caching
        let mtime = metadata.modified()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Check cache if enabled
        let file_hash = if self.config.use_hash_cache {
            if let Some(ref db) = self.db {
                if let Ok(Some((cached_hash, cached_mtime))) = db.get_file_cache(&file_path_str, mtime).await {
                    // File hasn't changed, use cached hash
                    cached_hash
                } else {
                    // File changed or not in cache, calculate hash
                    let hash = self.calculate_hash(file_path)?;
                    // Update cache
                    if let Err(e) = db.update_file_cache(&file_path_str, &hash, file_size as i64, mtime).await {
                        warn!("Failed to update file cache for {}: {}", file_path_str, e);
                    }
                    hash
                }
            } else {
                // No database, calculate hash
                self.calculate_hash(file_path)?
            }
        } else {
            // Caching disabled, calculate hash
            self.calculate_hash(file_path)?
        };

        // Check against all signatures
        for signature in &self.signatures {
            let mut matches = false;

            // Check file name pattern
            if let Some(ref name_pattern) = signature.file_name_pattern {
                if name_pattern.is_match(file_name) {
                    matches = true;
                }
            }

            // Check path pattern
            if let Some(ref path_pattern) = signature.path_pattern {
                if path_pattern.is_match(&file_path_str) {
                    matches = true;
                }
            }

            // Check file hash (exact match)
            if let Some(ref sig_hash) = signature.file_hash {
                if sig_hash.eq_ignore_ascii_case(&file_hash) {
                    matches = true;
                }
            }

            if matches {
                info!("🚨 Malware detected: {} (signature: {})", 
                      file_path.display(), signature.name);

                return Ok(Some(DetectedMalware {
                    file_path: file_path.to_path_buf(),
                    signature: signature.clone(),
                    file_hash,
                    file_size,
                    detected_at: chrono::Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    pub async fn scan_directory(&self, dir_path: &Path) -> Result<Vec<DetectedMalware>> {
        let mut detected = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(detected);
        }

        info!("Scanning directory: {}", dir_path.display());

        // Collect all files first
        let mut files_to_scan = Vec::new();
        for entry in WalkDir::new(dir_path)
            .follow_links(false)
            .max_depth(20) // Limit depth to prevent excessive scanning
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();

            // Skip if it's a directory
            if path.is_dir() {
                continue;
            }

            // Skip if it's a symlink (to avoid following malicious symlinks)
            if entry.file_type().is_symlink() {
                continue;
            }

            // Skip system directories early for performance
            let path_str = path.to_string_lossy();
            if path_str.contains("/proc/") || path_str.contains("/sys/") || path_str.contains("/dev/") {
                continue;
            }

            files_to_scan.push(path);
        }

        // Parallel or sequential scanning
        if self.config.parallel_scan && files_to_scan.len() > 10 {
            // Use parallel scanning for large directories
            use tokio::task;
            let mut handles = Vec::new();
            let chunk_size = (files_to_scan.len() / self.config.max_scan_threads).max(1);
            let signatures = self.signatures.clone();
            let use_cache = self.config.use_hash_cache;
            let db_opt = self.db.clone();
            
            for chunk in files_to_scan.chunks(chunk_size) {
                let chunk = chunk.to_vec();
                let signatures_clone = signatures.clone();
                let db_clone = db_opt.clone();
                
                let handle = task::spawn(async move {
                    let mut chunk_detected = Vec::new();
                    for path in chunk {
                        if let Ok(Some(malware)) = Self::scan_file_internal(&path, &signatures_clone, use_cache, db_clone.as_ref()).await {
                            chunk_detected.push(malware);
                        }
                    }
                    chunk_detected
                });
                handles.push(handle);
            }

            // Collect results
            for handle in handles {
                if let Ok(mut chunk_result) = handle.await {
                    detected.append(&mut chunk_result);
                }
            }
        } else {
            // Sequential scanning
            for path in files_to_scan {
                match self.scan_file(&path).await {
                    Ok(Some(malware)) => {
                        detected.push(malware);
                    }
                    Ok(None) => {
                        // File is clean
                    }
                    Err(e) => {
                        warn!("Failed to scan file {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(detected)
    }

    // Internal helper for parallel scanning
    async fn scan_file_internal(
        path: &Path,
        signatures: &[MalwareSignature],
        use_cache: bool,
        db: Option<&Arc<IntelligenceDB>>,
    ) -> Result<Option<DetectedMalware>> {
        if !path.exists() || !path.is_file() {
            return Ok(None);
        }

        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let file_path_str = path.to_string_lossy().to_string();

        // Get modification time for caching
        let mtime = metadata.modified()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Calculate or get cached hash
        let file_hash = if use_cache {
            if let Some(db) = db {
                if let Ok(Some((cached_hash, _))) = db.get_file_cache(&file_path_str, mtime).await {
                    cached_hash
                } else {
                    let hash = Self::calculate_hash_static(path)?;
                    if let Err(e) = db.update_file_cache(&file_path_str, &hash, file_size as i64, mtime).await {
                        warn!("Failed to update file cache for {}: {}", file_path_str, e);
                    }
                    hash
                }
            } else {
                Self::calculate_hash_static(path)?
            }
        } else {
            Self::calculate_hash_static(path)?
        };

        // Check against signatures
        for signature in signatures {
            let mut matches = false;

            if let Some(ref name_pattern) = signature.file_name_pattern {
                if name_pattern.is_match(file_name) {
                    matches = true;
                }
            }

            if let Some(ref path_pattern) = signature.path_pattern {
                if path_pattern.is_match(&file_path_str) {
                    matches = true;
                }
            }

            if let Some(ref sig_hash) = signature.file_hash {
                if sig_hash.eq_ignore_ascii_case(&file_hash) {
                    matches = true;
                }
            }

            if matches {
                return Ok(Some(DetectedMalware {
                    file_path: path.to_path_buf(),
                    signature: signature.clone(),
                    file_hash,
                    file_size,
                    detected_at: chrono::Utc::now(),
                }));
            }
        }

        Ok(None)
    }

    fn calculate_hash_static(file_path: &Path) -> Result<String> {
        let mut file = fs::File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = hasher.finalize();
        Ok(hex::encode(hash))
    }

    pub async fn scan_all_paths(&self) -> Result<Vec<DetectedMalware>> {
        let mut all_detected = Vec::new();

        for scan_path in &self.scan_paths {
            if scan_path.is_file() {
                // Single file scan
                if let Ok(Some(malware)) = self.scan_file(scan_path).await {
                    all_detected.push(malware);
                }
            } else if scan_path.is_dir() {
                // Directory scan
                match self.scan_directory(scan_path).await {
                    Ok(mut detected) => {
                        all_detected.append(&mut detected);
                    }
                    Err(e) => {
                        warn!("Failed to scan directory {}: {}", scan_path.display(), e);
                    }
                }
            }
        }

        Ok(all_detected)
    }

    fn calculate_hash(&self, file_path: &Path) -> Result<String> {
        let mut file = fs::File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = hasher.finalize();
        Ok(hex::encode(hash))
    }

    pub fn get_quarantine_path(&self) -> &Path {
        &self.quarantine_path
    }
}


