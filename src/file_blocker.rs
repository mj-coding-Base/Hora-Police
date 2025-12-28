use anyhow::{Result, Context};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;
use chrono::Utc;
use tracing::{info, warn, error};
use crate::file_watcher::FileWatcher;
use crate::database::IntelligenceDB;
use std::sync::Arc;

pub struct FileBlocker {
    blocked_paths: HashSet<PathBuf>,
    monitor: Option<FileWatcher>,
    db: Option<Arc<IntelligenceDB>>,
    enabled: bool,
}

impl FileBlocker {
    pub fn new(
        blocked_paths: Vec<PathBuf>,
        monitor_paths: Vec<PathBuf>,
        db: Option<Arc<IntelligenceDB>>,
        enabled: bool,
    ) -> Result<Self> {
        let blocked_set: HashSet<PathBuf> = blocked_paths.into_iter().collect();
        
        // Initialize file watcher for monitoring
        let monitor = if enabled {
            FileWatcher::new(monitor_paths).ok()
        } else {
            None
        };

        Ok(Self {
            blocked_paths: blocked_set,
            monitor,
            db,
            enabled,
        })
    }

    /// Block a file path from being recreated
    pub fn block_path(&mut self, path: &Path) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let path_buf = path.to_path_buf();
        self.blocked_paths.insert(path_buf.clone());
        
        // Try to make the directory immutable if it's a directory, or create a blocking file
        if let Some(parent) = path.parent() {
            // Create a marker file to track blocked paths
            let marker_path = parent.join(format!(".hora-police-blocked-{}", 
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")));
            
            if let Err(e) = fs::write(&marker_path, format!("Blocked by Hora-Police at {}\nOriginal path: {}", 
                Utc::now().to_rfc3339(), path.display())) {
                warn!("Failed to create block marker for {}: {}", path.display(), e);
            } else {
                info!("✅ Created block marker for {}", path.display());
            }
        }

        // Record in database if available
        if let Some(ref db) = self.db {
            // Could add a table for blocked paths tracking
            // For now, we'll just log
            info!("🚫 Blocked path: {}", path.display());
        }

        Ok(())
    }

    /// Check if a path is blocked
    pub fn is_blocked(&self, path: &Path) -> bool {
        self.blocked_paths.contains(path)
    }

    /// Monitor file system and block recreation attempts
    pub async fn monitor_and_block(&mut self) -> Result<Vec<PathBuf>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut blocked_attempts = Vec::new();

        if let Some(ref mut monitor) = self.monitor {
            // Get changed directories from file watcher
            let changed_dirs = monitor.watch_changes().await?;
            
            for dir in changed_dirs {
                // Check if any blocked files are being recreated
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        
                        // Check if this path matches a blocked path
                        if self.is_blocked(&path) {
                            warn!("🚨 Blocked file recreation detected: {}", path.display());
                            
                            // Try to delete the recreated file immediately
                            if let Err(e) = self.block_file_recreation(&path).await {
                                error!("Failed to block file recreation for {}: {}", path.display(), e);
                            } else {
                                blocked_attempts.push(path);
                            }
                        }
                    }
                }
            }
        }

        Ok(blocked_attempts)
    }

    /// Block file recreation by deleting it and creating a marker
    async fn block_file_recreation(&self, path: &Path) -> Result<()> {
        // Delete the recreated file
        if path.exists() {
            // Remove write protection if present
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
            
            // Delete the file
            fs::remove_file(path)
                .with_context(|| format!("Failed to delete recreated file: {}", path.display()))?;
            
            info!("🗑️  Deleted recreated blocked file: {}", path.display());
        }

        // Create or update block marker
        if let Some(parent) = path.parent() {
            let marker_path = parent.join(format!(".hora-police-blocked-{}", 
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")));
            
            fs::write(&marker_path, format!(
                "Blocked by Hora-Police\nOriginal path: {}\nLast recreation attempt: {}\n",
                path.display(),
                Utc::now().to_rfc3339()
            ))?;
        }

        Ok(())
    }

    /// Get all currently blocked paths
    pub fn get_blocked_paths(&self) -> Vec<PathBuf> {
        self.blocked_paths.iter().cloned().collect()
    }

    /// Remove a path from blocking (unblock)
    pub fn unblock_path(&mut self, path: &Path) -> Result<()> {
        self.blocked_paths.remove(path);
        
        // Remove block marker if it exists
        if let Some(parent) = path.parent() {
            let marker_path = parent.join(format!(".hora-police-blocked-{}", 
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")));
            
            if marker_path.exists() {
                fs::remove_file(&marker_path)?;
                info!("✅ Removed block marker for {}", path.display());
            }
        }

        Ok(())
    }
}

