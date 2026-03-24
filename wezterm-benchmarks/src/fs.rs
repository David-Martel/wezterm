//! High-performance file system operations with caching and parallelism

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{Mutex, RwLock};
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex as SyncMutex;
use memmap2::{Mmap, MmapOptions};
use walkdir::WalkDir;
use rayon::prelude::*;
use std::num::NonZeroUsize;
use serde::{Serialize, Deserialize};

/// Directory entry with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
}

/// High-performance directory scanner with caching
pub struct DirectoryScanner {
    cache: Arc<SyncMutex<LruCache<PathBuf, Vec<DirEntry>>>>,
}

impl DirectoryScanner {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(SyncMutex::new(
                LruCache::new(NonZeroUsize::new(100).unwrap())
            )),
        }
    }

    pub fn with_cache(size: usize) -> Self {
        Self {
            cache: Arc::new(SyncMutex::new(
                LruCache::new(NonZeroUsize::new(size).unwrap())
            )),
        }
    }

    pub async fn scan(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
        let entries = tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || Self::scan_sync(&path)
        }).await??;

        // Update cache
        {
            let mut cache = self.cache.lock();
            cache.put(path.to_path_buf(), entries.clone());
        }

        Ok(entries)
    }

    pub fn scan_cached(&self, path: &Path) -> Vec<DirEntry> {
        let mut cache = self.cache.lock();

        if let Some(entries) = cache.get(&path.to_path_buf()) {
            return entries.clone();
        }

        // Cache miss, scan directory
        drop(cache);
        let entries = Self::scan_sync(path).unwrap_or_default();

        let mut cache = self.cache.lock();
        cache.put(path.to_path_buf(), entries.clone());
        entries
    }

    fn scan_sync(path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(path).max_depth(3) {
            if let Ok(e) = entry {
                let metadata = e.metadata()?;
                entries.push(DirEntry {
                    path: e.path().to_path_buf(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.modified()?,
                });
            }
        }

        Ok(entries)
    }
}

/// Parallel directory scanner using rayon
pub struct ParallelScanner;

impl ParallelScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
        let path = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let walker = WalkDir::new(&path);

            let entries: Vec<DirEntry> = walker
                .into_iter()
                .par_bridge()
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        e.metadata().ok().map(|m| DirEntry {
                            path: e.path().to_path_buf(),
                            is_dir: m.is_dir(),
                            size: m.len(),
                            modified: m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        })
                    })
                })
                .collect();

            Ok(entries)
        }).await?
    }
}

/// Incremental directory scanner that tracks changes
pub struct IncrementalScanner {
    state: Arc<RwLock<HashMap<PathBuf, DirEntry>>>,
    last_scan: Arc<SyncMutex<HashMap<PathBuf, Instant>>>,
}

impl IncrementalScanner {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
            last_scan: Arc::new(SyncMutex::new(HashMap::new())),
        }
    }

    pub async fn initial_scan(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
        let scanner = DirectoryScanner::new();
        let entries = scanner.scan(path).await?;

        let mut state = self.state.write().await;
        for entry in &entries {
            state.insert(entry.path.clone(), entry.clone());
        }

        let mut last_scan = self.last_scan.lock();
        last_scan.insert(path.to_path_buf(), Instant::now());

        Ok(entries)
    }

    pub async fn get_changes(&self, path: &Path) -> Result<Vec<FileChange>, std::io::Error> {
        let scanner = DirectoryScanner::new();
        let current_entries = scanner.scan(path).await?;

        let state = self.state.read().await;
        let mut changes = Vec::new();

        // Check for new and modified files
        for entry in &current_entries {
            match state.get(&entry.path) {
                Some(old_entry) if old_entry.modified != entry.modified => {
                    changes.push(FileChange::Modified(entry.clone()));
                }
                None => {
                    changes.push(FileChange::Added(entry.clone()));
                }
                _ => {}
            }
        }

        // Check for deleted files
        let current_paths: std::collections::HashSet<_> =
            current_entries.iter().map(|e| &e.path).collect();

        for (path, entry) in state.iter() {
            if !current_paths.contains(path) {
                changes.push(FileChange::Deleted(entry.clone()));
            }
        }

        // Update state
        drop(state);
        let mut state = self.state.write().await;
        state.clear();
        for entry in current_entries {
            state.insert(entry.path.clone(), entry);
        }

        Ok(changes)
    }
}

#[derive(Debug, Clone)]
pub enum FileChange {
    Added(DirEntry),
    Modified(DirEntry),
    Deleted(DirEntry),
}

/// File cache with LRU eviction
pub struct FileCache {
    cache: Arc<SyncMutex<LruCache<PathBuf, Vec<u8>>>>,
}

impl FileCache {
    pub fn new(max_files: usize) -> Self {
        Self {
            cache: Arc::new(SyncMutex::new(
                LruCache::new(NonZeroUsize::new(max_files).unwrap())
            )),
        }
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, std::io::Error> {
        let mut cache = self.cache.lock();

        if let Some(content) = cache.get(&path.to_path_buf()) {
            return Ok(content.clone());
        }

        // Cache miss, read file
        drop(cache);
        let content = std::fs::read(path)?;

        let mut cache = self.cache.lock();
        cache.put(path.to_path_buf(), content.clone());

        Ok(content)
    }

    pub fn invalidate(&self, path: &Path) {
        let mut cache = self.cache.lock();
        cache.pop(&path.to_path_buf());
    }
}

/// Memory-mapped file reader for large files
pub struct MemoryMappedReader {
    mmap: Mmap,
}

impl MemoryMappedReader {
    pub fn new(path: &Path) -> Result<Self, std::io::Error> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self { mmap })
    }

    pub fn read_all(&self) -> &[u8] {
        &self.mmap[..]
    }

    pub fn read_range(&self, start: usize, end: usize) -> &[u8] {
        &self.mmap[start..end]
    }
}

/// File watcher with event aggregation
pub struct Watcher {
    events: Arc<Mutex<Vec<FileEvent>>>,
}

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

impl Watcher {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn watch(&self, path: &Path) -> Result<(), std::io::Error> {
        // Mock implementation for benchmarking
        Ok(())
    }

    pub async fn get_events(&self) -> Vec<FileEvent> {
        let mut events = self.events.lock().await;
        events.drain(..).collect()
    }
}

/// Debounced file watcher
pub struct DebouncedWatcher {
    debounce_duration: Duration,
    pending: Arc<Mutex<HashMap<PathBuf, FileEvent>>>,
}

impl DebouncedWatcher {
    pub fn new(debounce_duration: Duration) -> Self {
        Self {
            debounce_duration,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn watch(&self, path: &Path) -> Result<(), std::io::Error> {
        // Mock implementation for benchmarking
        Ok(())
    }

    pub async fn add_event(&self, event: FileEvent) {
        let mut pending = self.pending.lock().await;
        pending.insert(event.path.clone(), event);
    }

    pub async fn get_debounced_events(&self) -> Vec<FileEvent> {
        let mut pending = self.pending.lock().await;
        pending.drain().map(|(_, e)| e).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_directory_scanner() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();

        let scanner = DirectoryScanner::new();
        let entries = scanner.scan(temp_dir.path()).await.unwrap();

        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.path == file_path));
    }

    #[tokio::test]
    async fn test_incremental_scanner() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        std::fs::write(&file1, "content1").unwrap();

        let scanner = IncrementalScanner::new();
        let _ = scanner.initial_scan(temp_dir.path()).await.unwrap();

        // Add new file
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file2, "content2").unwrap();

        let changes = scanner.get_changes(temp_dir.path()).await.unwrap();

        assert!(changes.iter().any(|c| matches!(c, FileChange::Added(_))));
    }

    #[test]
    fn test_memory_mapped_reader() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.bin");
        let data = vec![42u8; 1000];
        std::fs::write(&file_path, &data).unwrap();

        let reader = MemoryMappedReader::new(&file_path).unwrap();
        let content = reader.read_all();

        assert_eq!(content.len(), 1000);
        assert_eq!(content[0], 42);
    }
}