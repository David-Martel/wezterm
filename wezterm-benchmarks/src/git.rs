//! Optimized Git operations with caching and parallel processing

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use git2::{Repository, Status, StatusOptions, Oid, DiffOptions};
use parking_lot::Mutex;
use cached::proc_macro::cached;
use lru::LruCache;
use std::num::NonZeroUsize;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;

/// Git status information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitStatusInfo {
    pub modified: Vec<PathBuf>,
    pub added: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
    pub renamed: Vec<(PathBuf, PathBuf)>,
    pub untracked: Vec<PathBuf>,
    pub conflicts: Vec<PathBuf>,
}

/// Git diff information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitDiffInfo {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub patches: Vec<FilePatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilePatch {
    pub path: PathBuf,
    pub additions: usize,
    pub deletions: usize,
}

/// Git log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub oid: String,
    pub author: String,
    pub email: String,
    pub summary: String,
    pub timestamp: i64,
}

/// Git blame information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlameInfo {
    pub lines: Vec<BlameLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
}

/// Cached Git status provider
pub struct GitStatusCache {
    cache: Arc<DashMap<PathBuf, CachedEntry<GitStatusInfo>>>,
    diff_cache: Arc<DashMap<PathBuf, CachedEntry<GitDiffInfo>>>,
    log_cache: Arc<Mutex<LruCache<PathBuf, CachedEntry<Vec<LogEntry>>>>>,
    blame_cache: Arc<Mutex<LruCache<(PathBuf, PathBuf), CachedEntry<BlameInfo>>>>,
    ttl: Duration,
}

struct CachedEntry<T> {
    value: T,
    timestamp: Instant,
}

impl<T: Clone> CachedEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
        }
    }

    fn is_valid(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() < ttl
    }
}

impl GitStatusCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            diff_cache: Arc::new(DashMap::new()),
            log_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
            blame_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
            ttl,
        }
    }

    pub fn get_status(&self, repo_path: &Path) -> Result<GitStatusInfo, git2::Error> {
        // Check cache
        if let Some(entry) = self.cache.get(&repo_path.to_path_buf()) {
            if entry.is_valid(self.ttl) {
                return Ok(entry.value.clone());
            }
        }

        // Cache miss or expired
        let repo = Repository::open(repo_path)?;
        let status = self.compute_status(&repo)?;

        // Update cache
        self.cache.insert(
            repo_path.to_path_buf(),
            CachedEntry::new(status.clone()),
        );

        Ok(status)
    }

    pub fn get_diff(&self, repo_path: &Path) -> Result<GitDiffInfo, git2::Error> {
        // Check cache
        if let Some(entry) = self.diff_cache.get(&repo_path.to_path_buf()) {
            if entry.is_valid(self.ttl) {
                return Ok(entry.value.clone());
            }
        }

        // Cache miss or expired
        let repo = Repository::open(repo_path)?;
        let diff = self.compute_diff(&repo)?;

        // Update cache
        self.diff_cache.insert(
            repo_path.to_path_buf(),
            CachedEntry::new(diff.clone()),
        );

        Ok(diff)
    }

    pub fn get_log(&self, repo_path: &Path, limit: usize) -> Result<Vec<LogEntry>, git2::Error> {
        // Check cache
        {
            let mut cache = self.log_cache.lock();
            if let Some(entry) = cache.get(&repo_path.to_path_buf()) {
                if entry.is_valid(self.ttl) {
                    return Ok(entry.value.clone());
                }
            }
        }

        // Cache miss or expired
        let repo = Repository::open(repo_path)?;
        let log = self.compute_log(&repo, limit)?;

        // Update cache
        {
            let mut cache = self.log_cache.lock();
            cache.put(
                repo_path.to_path_buf(),
                CachedEntry::new(log.clone()),
            );
        }

        Ok(log)
    }

    pub fn blame_file(&self, repo_path: &Path, file_path: &Path) -> Result<BlameInfo, git2::Error> {
        let cache_key = (repo_path.to_path_buf(), file_path.to_path_buf());

        // Check cache
        {
            let mut cache = self.blame_cache.lock();
            if let Some(entry) = cache.get(&cache_key) {
                if entry.is_valid(self.ttl) {
                    return Ok(entry.value.clone());
                }
            }
        }

        // Cache miss or expired
        let repo = Repository::open(repo_path)?;
        let blame = self.compute_blame(&repo, file_path)?;

        // Update cache
        {
            let mut cache = self.blame_cache.lock();
            cache.put(cache_key, CachedEntry::new(blame.clone()));
        }

        Ok(blame)
    }

    fn compute_status(&self, repo: &Repository) -> Result<GitStatusInfo, git2::Error> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut info = GitStatusInfo {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            untracked: Vec::new(),
            conflicts: Vec::new(),
        };

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status = entry.status();

            if status.contains(Status::WT_MODIFIED) || status.contains(Status::INDEX_MODIFIED) {
                info.modified.push(path.clone());
            }
            if status.contains(Status::WT_NEW) || status.contains(Status::INDEX_NEW) {
                info.added.push(path.clone());
            }
            if status.contains(Status::WT_DELETED) || status.contains(Status::INDEX_DELETED) {
                info.deleted.push(path.clone());
            }
            if status.contains(Status::WT_RENAMED) || status.contains(Status::INDEX_RENAMED) {
                // For simplicity, just add to renamed without tracking the old name
                info.renamed.push((path.clone(), path.clone()));
            }
            if status.contains(Status::WT_NEW) && !status.contains(Status::INDEX_NEW) {
                info.untracked.push(path.clone());
            }
            if status.contains(Status::CONFLICTED) {
                info.conflicts.push(path);
            }
        }

        Ok(info)
    }

    fn compute_diff(&self, repo: &Repository) -> Result<GitDiffInfo, git2::Error> {
        let head = repo.head()?.peel_to_tree()?;
        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut opts))?;

        let stats = diff.stats()?;

        let mut patches = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    patches.push(FilePatch {
                        path: path.to_path_buf(),
                        additions: 0,
                        deletions: 0,
                    });
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok(GitDiffInfo {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            patches,
        })
    }

    fn compute_log(&self, repo: &Repository, limit: usize) -> Result<Vec<LogEntry>, git2::Error> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut entries = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }

            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            entries.push(LogEntry {
                oid: oid.to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                email: commit.author().email().unwrap_or("").to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(entries)
    }

    fn compute_blame(&self, repo: &Repository, file_path: &Path) -> Result<BlameInfo, git2::Error> {
        let blame = repo.blame_file(file_path, None)?;
        let mut lines = Vec::new();

        for i in 0..blame.len() {
            if let Some(hunk) = blame.get(i) {
                let commit = repo.find_commit(hunk.final_commit_id())?;

                lines.push(BlameLine {
                    line_number: i + 1,
                    commit_id: hunk.final_commit_id().to_string(),
                    author: commit.author().name().unwrap_or("").to_string(),
                    timestamp: commit.time().seconds(),
                });
            }
        }

        Ok(BlameInfo { lines })
    }
}

/// Basic Git operations without caching
pub struct GitOperations {
    repo_path: PathBuf,
}

impl GitOperations {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }

    pub fn get_status(&self) -> Result<GitStatusInfo, git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        let cache = GitStatusCache::new(Duration::from_secs(0));
        cache.compute_status(&repo)
    }

    pub fn get_diff(&self) -> Result<GitDiffInfo, git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        let cache = GitStatusCache::new(Duration::from_secs(0));
        cache.compute_diff(&repo)
    }

    pub fn get_log(&self, limit: usize) -> Result<Vec<LogEntry>, git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        let cache = GitStatusCache::new(Duration::from_secs(0));
        cache.compute_log(&repo, limit)
    }

    pub fn blame_file(&self, file_path: &Path) -> Result<BlameInfo, git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        let cache = GitStatusCache::new(Duration::from_secs(0));
        cache.compute_blame(&repo, file_path)
    }
}

/// Parallel Git status computation
pub struct ParallelGitStatus;

impl ParallelGitStatus {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_status(&self, repo_path: &Path) -> Result<GitStatusInfo, git2::Error> {
        let repo_path = repo_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path)?;
            let mut opts = StatusOptions::new();
            opts.include_untracked(true);

            let statuses = repo.statuses(Some(&mut opts))?;

            // Process statuses in parallel
            let entries: Vec<_> = statuses
                .iter()
                .collect();

            let results: Vec<_> = entries
                .par_iter()
                .map(|entry| {
                    let path = PathBuf::from(entry.path().unwrap_or(""));
                    let status = entry.status();
                    (path, status)
                })
                .collect();

            let mut info = GitStatusInfo {
                modified: Vec::new(),
                added: Vec::new(),
                deleted: Vec::new(),
                renamed: Vec::new(),
                untracked: Vec::new(),
                conflicts: Vec::new(),
            };

            for (path, status) in results {
                if status.contains(Status::WT_MODIFIED) {
                    info.modified.push(path.clone());
                }
                if status.contains(Status::WT_NEW) {
                    info.added.push(path);
                }
            }

            Ok(info)
        }).await.unwrap()
    }
}

/// Incremental Git status tracking
pub struct IncrementalGitStatus {
    last_status: Arc<Mutex<Option<GitStatusInfo>>>,
    repo_path: PathBuf,
}

impl IncrementalGitStatus {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            last_status: Arc::new(Mutex::new(None)),
            repo_path: repo_path.to_path_buf(),
        }
    }

    pub fn get_changes(&self) -> Result<GitStatusInfo, git2::Error> {
        let ops = GitOperations::new(&self.repo_path);
        let current = ops.get_status()?;

        let mut last = self.last_status.lock();
        *last = Some(current.clone());

        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        (temp_dir, repo)
    }

    #[test]
    fn test_git_status_cache() {
        let (temp_dir, _repo) = create_test_repo();
        let cache = GitStatusCache::new(Duration::from_secs(60));

        // First call should compute
        let status1 = cache.get_status(temp_dir.path()).unwrap();

        // Second call should use cache
        let status2 = cache.get_status(temp_dir.path()).unwrap();

        assert_eq!(status1.modified.len(), status2.modified.len());
    }

    #[tokio::test]
    async fn test_parallel_git_status() {
        let (temp_dir, _repo) = create_test_repo();
        let parallel = ParallelGitStatus::new();

        let status = parallel.get_status(temp_dir.path()).await.unwrap();
        assert!(status.modified.is_empty());
    }
}