use anyhow::{Context, Result};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackManifest {
    pub timestamp: String,
    pub actions: Vec<RollbackAction>,
    pub hmac_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RollbackAction {
    RestoreFile {
        from: String,
        to: String,
    },
    RestoreCron {
        user: String,
        content: String,
        file: String,
    },
    RestartProcess {
        pid: i32,
        command: String,
    },
    RestoreDirectory {
        path: String,
    },
}

impl RollbackManifest {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            actions: Vec::new(),
            hmac_signature: String::new(),
        }
    }

    pub fn add_action(&mut self, action: RollbackAction) {
        self.actions.push(action);
    }

    /// Sign manifest with HMAC
    pub fn sign(&mut self, key: &[u8]) -> Result<()> {
        let json = serde_json::to_string(self)
            .context("Failed to serialize manifest for signing")?;
        
        // Remove signature field for signing
        let json_without_sig = json.replace(&format!("\"hmac_signature\":\"{}\"", self.hmac_signature), "\"hmac_signature\":\"\"");
        
        let mut mac = HmacSha256::new_from_slice(key)
            .context("Failed to create HMAC")?;
        mac.update(json_without_sig.as_bytes());
        let signature = mac.finalize();
        
        self.hmac_signature = hex::encode(signature.into_bytes());
        Ok(())
    }

    /// Verify manifest signature
    pub fn verify(&self, key: &[u8]) -> Result<bool> {
        let mut manifest_clone = self.clone();
        let original_sig = manifest_clone.hmac_signature.clone();
        manifest_clone.hmac_signature = String::new();
        
        let json = serde_json::to_string(&manifest_clone)
            .context("Failed to serialize manifest for verification")?;
        
        let mut mac = HmacSha256::new_from_slice(key)
            .context("Failed to create HMAC")?;
        mac.update(json.as_bytes());
        let signature = mac.finalize();
        let computed_sig = hex::encode(signature.into_bytes());
        
        Ok(computed_sig == original_sig)
    }

    /// Generate shell script for rollback
    pub fn to_shell_script(&self) -> String {
        let mut script = String::from("#!/bin/bash\n");
        script.push_str("# Hora-Police Rollback Script\n");
        script.push_str(&format!("# Generated: {}\n", self.timestamp));
        script.push_str(&format!("# Actions: {}\n\n", self.actions.len()));
        script.push_str("set -e\n\n");

        for (idx, action) in self.actions.iter().enumerate() {
            script.push_str(&format!("# Action {}\n", idx + 1));
            match action {
                RollbackAction::RestoreFile { from, to } => {
                    script.push_str(&format!(
                        "if [ -f \"{}\" ]; then\n",
                        from
                    ));
                    script.push_str(&format!(
                        "  echo \"Restoring file: {} -> {}\"\n",
                        from, to
                    ));
                    script.push_str(&format!(
                        "  mkdir -p \"$(dirname '{}')\"\n",
                        to
                    ));
                    script.push_str(&format!(
                        "  cp \"{}\" \"{}\"\n",
                        from, to
                    ));
                    script.push_str("else\n");
                    script.push_str(&format!(
                        "  echo \"Warning: Source file {} not found\"\n",
                        from
                    ));
                    script.push_str("fi\n\n");
                }
                RollbackAction::RestoreCron { user, content, file } => {
                    script.push_str(&format!(
                        "echo \"Restoring cron for user: {}\"\n",
                        user
                    ));
                    script.push_str(&format!(
                        "echo '{}' | crontab -u {} -\n",
                        content, user
                    ));
                    script.push_str("\n");
                }
                RollbackAction::RestartProcess { pid: _, command } => {
                    script.push_str(&format!(
                        "echo \"Restarting process: {}\"\n",
                        command
                    ));
                    script.push_str(&format!(
                        "{} &\n",
                        command
                    ));
                    script.push_str("\n");
                }
                RollbackAction::RestoreDirectory { path } => {
                    script.push_str(&format!(
                        "echo \"Restoring directory: {}\"\n",
                        path
                    ));
                    script.push_str(&format!(
                        "if [ -d \"{}.backup\" ]; then\n",
                        path
                    ));
                    script.push_str(&format!(
                        "  rm -rf \"{}\"\n",
                        path
                    ));
                    script.push_str(&format!(
                        "  mv \"{}.backup\" \"{}\"\n",
                        path, path
                    ));
                    script.push_str("fi\n\n");
                }
            }
        }

        script.push_str("echo \"Rollback completed\"\n");
        script
    }

    /// Generate JSON representation
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize manifest to JSON")
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Save JSON
        let json_path = path.with_extension("json");
        let json_content = self.to_json()?;
        fs::write(&json_path, json_content)
            .with_context(|| format!("Failed to write JSON manifest to {:?}", json_path))?;

        // Save shell script
        let script_path = path.with_extension("sh");
        let script_content = self.to_shell_script();
        let mut file = fs::File::create(&script_path)
            .with_context(|| format!("Failed to create shell script at {:?}", script_path))?;
        file.write_all(script_content.as_bytes())
            .with_context(|| format!("Failed to write shell script to {:?}", script_path))?;
        
        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        Ok(())
    }

    /// Load manifest from file
    pub fn load(path: &Path) -> Result<Self> {
        let json_path = path.with_extension("json");
        let content = fs::read_to_string(&json_path)
            .with_context(|| format!("Failed to read manifest from {:?}", json_path))?;
        
        let manifest: RollbackManifest = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse manifest JSON from {:?}", json_path))?;
        
        Ok(manifest)
    }
}

impl Default for RollbackManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Get or create rollback key
pub fn get_rollback_key() -> Result<Vec<u8>> {
    let key_path = PathBuf::from("/etc/hora-police/keys/rollback.key");
    
    // Create directory if it doesn't exist
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create keys directory")?;
    }

    // Load or generate key
    if key_path.exists() {
        fs::read(&key_path)
            .with_context(|| format!("Failed to read rollback key from {:?}", key_path))
    } else {
        // Generate new key
        use rand::RngCore;
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        
        fs::write(&key_path, &key)
            .with_context(|| format!("Failed to write rollback key to {:?}", key_path))?;
        
        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&key_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&key_path, perms)?;
        }
        
        Ok(key)
    }
}

