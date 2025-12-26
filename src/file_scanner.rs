use anyhow::Result;
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use walkdir::WalkDir;
use regex::Regex;
use tracing::{info, warn, error};

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
}

impl FileScanner {
    pub fn new(scan_paths: Vec<PathBuf>, quarantine_path: PathBuf) -> Self {
        let mut scanner = Self {
            signatures: Vec::new(),
            scan_paths,
            quarantine_path,
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

    pub fn scan_file(&self, file_path: &Path) -> Result<Option<DetectedMalware>> {
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
        let file_path_str = file_path.to_string_lossy();

        // Calculate file hash
        let file_hash = self.calculate_hash(file_path)?;

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

    pub fn scan_directory(&self, dir_path: &Path) -> Result<Vec<DetectedMalware>> {
        let mut detected = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(detected);
        }

        info!("Scanning directory: {}", dir_path.display());

        for entry in WalkDir::new(dir_path)
            .follow_links(false)
            .max_depth(20) // Limit depth to prevent excessive scanning
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip if it's a directory
            if path.is_dir() {
                continue;
            }

            // Skip if it's a symlink (to avoid following malicious symlinks)
            if entry.file_type().is_symlink() {
                continue;
            }

            // Scan the file
            match self.scan_file(path) {
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

        Ok(detected)
    }

    pub fn scan_all_paths(&self) -> Result<Vec<DetectedMalware>> {
        let mut all_detected = Vec::new();

        for scan_path in &self.scan_paths {
            if scan_path.is_file() {
                // Single file scan
                if let Ok(Some(malware)) = self.scan_file(scan_path) {
                    all_detected.push(malware);
                }
            } else if scan_path.is_dir() {
                // Directory scan
                match self.scan_directory(scan_path) {
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


