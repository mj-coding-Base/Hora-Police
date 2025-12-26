use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use std::fs;

use crate::process_monitor::ProcessInfo;
use crate::pm2_integration::Pm2Integration;
use crate::systemd_integration::SystemdIntegration;
use crate::nginx_integration::NginxIntegration;

#[derive(Debug, Clone)]
pub struct WhitelistEntry {
    pub pattern: String,  // Regex or exact match
    pub source: WhitelistSource,
    pub fingerprint: Option<String>,  // SHA256 of binary or package.json
}

#[derive(Debug, Clone)]
pub enum WhitelistSource {
    Pm2App,
    SystemdUnit,
    NginxUpstream,
    PackageJson,
    Manual,
}

pub struct WhitelistManager {
    entries: Vec<WhitelistEntry>,
    compiled_patterns: Vec<Regex>,
    fingerprints: HashSet<String>,
}

impl WhitelistManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            compiled_patterns: Vec::new(),
            fingerprints: HashSet::new(),
        }
    }

    /// Build whitelist from environment (PM2, systemd, Nginx, package.json)
    pub fn build_from_environment(
        pm2: &mut Pm2Integration,
        systemd: &mut SystemdIntegration,
        nginx: &mut NginxIntegration,
        manual_patterns: &[String],
    ) -> Result<Self> {
        let mut manager = Self::new();

        // 1. Add PM2 apps
        if let Ok(apps) = pm2.detect_apps() {
            for app in apps {
                // Add app name pattern
                manager.add_entry(WhitelistEntry {
                    pattern: format!("^{}$", regex::escape(&app.name)),
                    source: WhitelistSource::Pm2App,
                    fingerprint: None,
                });

                // Add path pattern
                if let Some(path_str) = app.path.to_str() {
                    manager.add_entry(WhitelistEntry {
                        pattern: format!("^{}", regex::escape(path_str)),
                        source: WhitelistSource::Pm2App,
                        fingerprint: None,
                    });
                }

                // Generate fingerprint from package.json if exists
                if let Some(pkg_json) = manager.find_package_json(&app.path) {
                    if let Ok(fingerprint) = manager.fingerprint_file(&pkg_json) {
                        manager.add_entry(WhitelistEntry {
                            pattern: format!("^{}", regex::escape(path_str.unwrap_or(""))),
                            source: WhitelistSource::Pm2App,
                            fingerprint: Some(fingerprint),
                        });
                    }
                }
            }
        }

        // 2. Add systemd units
        if let Ok(units) = systemd.detect_units() {
            for unit in units {
                // Add ExecStart pattern
                manager.add_entry(WhitelistEntry {
                    pattern: format!("^{}", regex::escape(&unit.exec_start)),
                    source: WhitelistSource::SystemdUnit,
                    fingerprint: None,
                });

                // Add working directory pattern
                if let Some(wd) = &unit.working_directory {
                    if let Some(wd_str) = wd.to_str() {
                        manager.add_entry(WhitelistEntry {
                            pattern: format!("^{}", regex::escape(wd_str)),
                            source: WhitelistSource::SystemdUnit,
                            fingerprint: None,
                        });
                    }
                }
            }
        }

        // 3. Add Nginx upstreams
        if let Ok(upstreams) = nginx.detect_upstreams() {
            for upstream in upstreams {
                if let Some(app_path) = &upstream.app_path {
                    if let Some(path_str) = app_path.to_str() {
                        manager.add_entry(WhitelistEntry {
                            pattern: format!("^{}", regex::escape(path_str)),
                            source: WhitelistSource::NginxUpstream,
                            fingerprint: None,
                        });
                    }
                }
            }
        }

        // 4. Scan for package.json files
        let scan_paths = vec![
            "/var/www",
            "/srv",
            "/opt",
        ];

        for base_path in scan_paths {
            if let Ok(entries) = fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(pkg_json) = manager.find_package_json(&path) {
                            if let Ok(pkg_name) = manager.extract_package_name(&pkg_json) {
                                manager.add_entry(WhitelistEntry {
                                    pattern: format!("^{}$", regex::escape(&pkg_name)),
                                    source: WhitelistSource::PackageJson,
                                    fingerprint: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Also scan /home/*/www and /home/*/projects
        if let Ok(entries) = fs::read_dir("/home") {
            for entry in entries.flatten() {
                let home_path = entry.path();
                if home_path.is_dir() {
                    for subdir in &["www", "projects"] {
                        let project_path = home_path.join(subdir);
                        if project_path.is_dir() {
                            if let Some(pkg_json) = manager.find_package_json(&project_path) {
                                if let Ok(pkg_name) = manager.extract_package_name(&pkg_json) {
                                    manager.add_entry(WhitelistEntry {
                                        pattern: format!("^{}$", regex::escape(&pkg_name)),
                                        source: WhitelistSource::PackageJson,
                                        fingerprint: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Add common patterns
        let common_patterns = vec![
            "^next$",
            "^nest$",
            "node.*dist/main.js",
            "next start",
            "nest start",
        ];

        for pattern in common_patterns {
            manager.add_entry(WhitelistEntry {
                pattern: pattern.to_string(),
                source: WhitelistSource::Manual,
                fingerprint: None,
            });
        }

        // 6. Add manual patterns
        for pattern in manual_patterns {
            manager.add_entry(WhitelistEntry {
                pattern: pattern.clone(),
                source: WhitelistSource::Manual,
                fingerprint: None,
            });
        }

        Ok(manager)
    }

    fn add_entry(&mut self, entry: WhitelistEntry) {
        // Try to compile pattern
        if let Ok(regex) = Regex::new(&entry.pattern) {
            self.compiled_patterns.push(regex);
        }
        
        if let Some(ref fingerprint) = entry.fingerprint {
            self.fingerprints.insert(fingerprint.clone());
        }
        
        self.entries.push(entry);
    }

    pub fn is_whitelisted(&self, process: &ProcessInfo) -> bool {
        // Check binary path
        for pattern in &self.compiled_patterns {
            if pattern.is_match(&process.binary_path) {
                return true;
            }
        }

        // Check command line
        for pattern in &self.compiled_patterns {
            if pattern.is_match(&process.command_line) {
                return true;
            }
        }

        // Check fingerprint if we have binary path
        if let Ok(fingerprint) = self.fingerprint_file(&PathBuf::from(&process.binary_path)) {
            if self.fingerprints.contains(&fingerprint) {
                return true;
            }
        }

        false
    }

    pub fn add_manual_entry(&mut self, pattern: String) {
        self.add_entry(WhitelistEntry {
            pattern,
            source: WhitelistSource::Manual,
            fingerprint: None,
        });
    }

    fn find_package_json(&self, dir: &PathBuf) -> Option<PathBuf> {
        let pkg_json = dir.join("package.json");
        if pkg_json.exists() {
            return Some(pkg_json);
        }
        None
    }

    fn extract_package_name(&self, pkg_json: &PathBuf) -> Result<String> {
        let content = fs::read_to_string(pkg_json)
            .context("Failed to read package.json")?;
        
        #[derive(serde::Deserialize)]
        struct PackageJson {
            name: String,
        }

        let pkg: PackageJson = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;

        Ok(pkg.name)
    }

    fn fingerprint_file(&self, path: &PathBuf) -> Result<String> {
        let content = fs::read(path)
            .context("Failed to read file for fingerprinting")?;
        
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        
        Ok(hex::encode(hash))
    }

    pub fn get_entries(&self) -> &[WhitelistEntry] {
        &self.entries
    }
}

impl Default for WhitelistManager {
    fn default() -> Self {
        Self::new()
    }
}

