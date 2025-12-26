use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct NpmPackageInfo {
    pub package_name: String,
    pub version: String,
    pub install_scripts: Vec<String>,
    pub binary_path: String,
    pub threat_level: f32,
}

pub struct NpmScanner {
    known_miner_packages: Vec<String>,
    suspicious_script_patterns: Vec<String>,
}

impl NpmScanner {
    pub fn new() -> Self {
        let known_miner_packages = vec![
            "coinhive",
            "cryptonight",
            "xmrig",
            "miner",
            "crypto-miner",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let suspicious_script_patterns = vec![
            "miner",
            "crypto",
            "coin",
            "hash",
            "mine",
            "xmrig",
            "stratum",
            "pool",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        Self {
            known_miner_packages,
            suspicious_script_patterns,
        }
    }

    pub fn scan_process(&self, binary_path: &str, command_line: &str) -> Result<Vec<NpmPackageInfo>> {
        let mut infections = Vec::new();

        // Check if this is a Node.js process
        if !binary_path.contains("node") && !command_line.contains("node") {
            return Ok(infections);
        }

        // Try to find the working directory from command line
        let working_dir = self.extract_working_dir(binary_path, command_line);
        
        if let Some(dir) = working_dir {
            if let Ok(packages) = self.scan_directory(&dir) {
                infections.extend(packages);
            }
        }

        Ok(infections)
    }

    fn extract_working_dir(&self, binary_path: &str, command_line: &str) -> Option<PathBuf> {
        // Try to extract directory from command line
        // Common patterns: node /path/to/script.js, node index.js (relative)
        
        // Look for .js files in command line
        let parts: Vec<&str> = command_line.split_whitespace().collect();
        for part in parts {
            if part.ends_with(".js") || part.ends_with("/index.js") {
                let path = PathBuf::from(part);
                if path.is_absolute() {
                    return path.parent().map(|p| p.to_path_buf());
                }
            }
        }

        // Try to find node_modules in parent directories
        if let Some(proc_path) = Path::new(binary_path).parent() {
            let mut current = proc_path.to_path_buf();
            for _ in 0..10 {
                if current.join("node_modules").exists() {
                    return Some(current);
                }
                if !current.pop() {
                    break;
                }
            }
        }

        None
    }

    fn scan_directory(&self, dir: &Path) -> Result<Vec<NpmPackageInfo>> {
        let mut infections = Vec::new();

        // Check for package.json
        let package_json_path = dir.join("package.json");
        if !package_json_path.exists() {
            return Ok(infections);
        }

        // Parse package.json
        let content = fs::read_to_string(&package_json_path)?;
        let package_json: Value = serde_json::from_str(&content)?;

        // Check dependencies
        let deps = self.extract_dependencies(&package_json);
        
        for (name, version) in deps {
            let threat_level = self.calculate_threat_level(&name, &version, &package_json);
            
            if threat_level > 0.3 {
                let install_scripts = self.extract_scripts(&package_json);
                
                infections.push(NpmPackageInfo {
                    package_name: name.clone(),
                    version,
                    install_scripts,
                    binary_path: dir.display().to_string(),
                    threat_level,
                });
            }
        }

        // Also scan node_modules for suspicious packages
        let node_modules = dir.join("node_modules");
        if node_modules.exists() {
            if let Ok(additional) = self.scan_node_modules(&node_modules) {
                infections.extend(additional);
            }
        }

        Ok(infections)
    }

    fn extract_dependencies(&self, package_json: &Value) -> Vec<(String, String)> {
        let mut deps = Vec::new();

        for key in &["dependencies", "devDependencies", "optionalDependencies"] {
            if let Some(deps_obj) = package_json.get(key).and_then(|v| v.as_object()) {
                for (name, version) in deps_obj {
                    let version_str = version.as_str().unwrap_or("unknown").to_string();
                    deps.push((name.clone(), version_str));
                }
            }
        }

        deps
    }

    fn extract_scripts(&self, package_json: &Value) -> Vec<String> {
        let mut scripts = Vec::new();

        if let Some(scripts_obj) = package_json.get("scripts").and_then(|v| v.as_object()) {
            for (name, script) in scripts_obj {
                if let Some(script_str) = script.as_str() {
                    scripts.push(format!("{}: {}", name, script_str));
                }
            }
        }

        scripts
    }

    fn calculate_threat_level(&self, package_name: &str, _version: &str, package_json: &Value) -> f32 {
        let mut threat = 0.0;

        // Check against known miner packages
        let name_lower = package_name.to_lowercase();
        for known_miner in &self.known_miner_packages {
            if name_lower.contains(known_miner) {
                threat = 0.9;
                return threat;
            }
        }

        // Check script names for suspicious patterns
        if let Some(scripts) = package_json.get("scripts").and_then(|v| v.as_object()) {
            for script_name in scripts.keys() {
                let script_lower = script_name.to_lowercase();
                for pattern in &self.suspicious_script_patterns {
                    if script_lower.contains(pattern) {
                        threat += 0.2;
                    }
                }
            }
        }

        // Check for postinstall scripts (common attack vector)
        if let Some(scripts) = package_json.get("scripts").and_then(|v| v.as_object()) {
            if scripts.contains_key("postinstall") {
                threat += 0.3;
            }
        }

        threat.min(1.0)
    }

    fn scan_node_modules(&self, node_modules: &Path) -> Result<Vec<NpmPackageInfo>> {
        let mut infections = Vec::new();

        // Walk through node_modules looking for package.json files
        for entry in WalkDir::new(node_modules)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "package.json" {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(package_json) = serde_json::from_str::<Value>(&content) {
                        if let Some(name) = package_json.get("name").and_then(|v| v.as_str()) {
                            let version = package_json
                                .get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            let threat_level = self.calculate_threat_level(name, &version, &package_json);
                            
                            if threat_level > 0.3 {
                                let install_scripts = self.extract_scripts(&package_json);
                                
                                infections.push(NpmPackageInfo {
                                    package_name: name.to_string(),
                                    version,
                                    install_scripts,
                                    binary_path: entry.path().parent()
                                        .unwrap()
                                        .display()
                                        .to_string(),
                                    threat_level,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(infections)
    }
}

