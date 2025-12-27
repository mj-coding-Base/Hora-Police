use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct FileWatcher {
    watch_paths: Vec<PathBuf>,
    inotify: Option<inotify::Inotify>,
    changed_dirs: Arc<Mutex<HashSet<PathBuf>>>,
    use_inotify: bool,
}

impl FileWatcher {
    pub fn new(paths: Vec<PathBuf>) -> Result<Self> {
        let mut watcher = Self {
            watch_paths: paths.clone(),
            inotify: None,
            changed_dirs: Arc::new(Mutex::new(HashSet::new())),
            use_inotify: false,
        };

        // Try to initialize inotify
        match inotify::Inotify::init() {
            Ok(mut inotify) => {
                use inotify::WatchMask;
                // Add watches for all paths
                for path in &paths {
                    if path.exists() {
                        if let Err(e) = inotify.watches().add(path, WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE | WatchMask::MOVED_FROM | WatchMask::MOVED_TO) {
                            warn!("Failed to add inotify watch for {}: {}", path.display(), e);
                        } else {
                            info!("Added inotify watch for: {}", path.display());
                        }
                    }
                }
                watcher.inotify = Some(inotify);
                watcher.use_inotify = true;
            }
            Err(e) => {
                warn!("Failed to initialize inotify, falling back to scheduled scans: {}", e);
            }
        }

        Ok(watcher)
    }

    /// Watch for file system changes and return changed directories
    pub async fn watch_changes(&mut self) -> Result<Vec<PathBuf>> {
        let mut changed = Vec::new();

        if let Some(ref mut inotify) = self.inotify {
            // Read events with timeout (non-blocking)
            let mut buffer = [0u8; 4096];
            match inotify.read_events(&mut buffer) {
                Ok(events) => {
                    let mut changed_dirs = self.changed_dirs.lock().await;
                    for event in events {
                        if let Some(name) = event.name {
                            // Build full path from watch descriptor and name
                            let watch_path = self.watch_paths.iter()
                                .find(|p| p.exists())
                                .cloned();
                            
                            if let Some(base_path) = watch_path {
                                let full_path = base_path.join(name);
                                // Get parent directory
                                if let Some(parent) = full_path.parent() {
                                    changed_dirs.insert(parent.to_path_buf());
                                    changed.push(parent.to_path_buf());
                                }
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No events available, that's fine
                }
                Err(e) => {
                    warn!("Error reading inotify events: {}", e);
                }
            }
        }

        // If inotify not available or no events, return empty (will use scheduled scan)
        Ok(changed)
    }

    /// Get all changed directories since last check
    pub async fn get_changed_directories(&self) -> Vec<PathBuf> {
        let changed_dirs = self.changed_dirs.lock().await;
        changed_dirs.iter().cloned().collect()
    }

    /// Clear changed directories list
    pub async fn clear_changed_directories(&self) {
        let mut changed_dirs = self.changed_dirs.lock().await;
        changed_dirs.clear();
    }

    /// Check if inotify is being used
    pub fn is_inotify_enabled(&self) -> bool {
        self.use_inotify
    }

    /// Add a new path to watch
    pub fn add_watch_path(&mut self, path: PathBuf) -> Result<()> {
        if !self.watch_paths.contains(&path) {
            self.watch_paths.push(path.clone());
            
            if let Some(ref mut inotify) = self.inotify {
                use inotify::WatchMask;
                if path.exists() {
                    inotify.watches().add(&path, WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE | WatchMask::MOVED_FROM | WatchMask::MOVED_TO)
                        .with_context(|| format!("Failed to add watch for {}", path.display()))?;
                }
            }
        }
        Ok(())
    }
}

/// Fallback: Scheduled shallow directory walk
pub async fn shallow_scan_directories(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;
    
    let mut changed_dirs = HashSet::new();
    
    for path in paths {
        if !path.exists() {
            continue;
        }

        // Shallow walk (max depth 2 for performance)
        for entry in WalkDir::new(path).max_depth(2).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                // Check modification time
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        // If modified in last 15 minutes, consider it changed
                        let elapsed = std::time::SystemTime::now()
                            .duration_since(modified)
                            .unwrap_or_default();
                        
                        if elapsed.as_secs() < 900 { // 15 minutes
                            changed_dirs.insert(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }
    }
    
    Ok(changed_dirs.into_iter().collect())
}

