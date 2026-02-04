use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    #[allow(dead_code)] // Used in tests and reserved for future use
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Staged,
    Unknown,
}

impl FileStatus {
    pub fn to_short_str(&self) -> &str {
        match self {
            FileStatus::Modified => "M",
            FileStatus::Added => "A",
            FileStatus::Deleted => "D",
            FileStatus::Renamed => "R",
            FileStatus::Untracked => "?",
            FileStatus::Conflicted => "U",
            FileStatus::Staged => "S",
            FileStatus::Unknown => " ",
        }
    }

    pub fn to_colored_str(&self) -> String {
        use colored::Colorize;
        match self {
            FileStatus::Modified => "M".yellow().to_string(),
            FileStatus::Added => "A".green().to_string(),
            FileStatus::Deleted => "D".red().to_string(),
            FileStatus::Renamed => "R".blue().to_string(),
            FileStatus::Untracked => "?".bright_black().to_string(),
            FileStatus::Conflicted => "U".red().bold().to_string(),
            FileStatus::Staged => "S".green().to_string(),
            FileStatus::Unknown => " ".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub has_conflicts: bool,
    pub file_statuses: HashMap<PathBuf, FileStatus>,
}

pub struct GitMonitor {
    repo_path: Option<PathBuf>,
    repo: Option<gix::Repository>,
    cache: Arc<Mutex<CachedGitInfo>>,
}

struct CachedGitInfo {
    info: Option<GitInfo>,
    last_update: Instant,
    cache_duration: Duration,
}

impl GitMonitor {
    pub fn new(path: &Path) -> Self {
        let (repo_path, repo) = Self::find_repository(path);

        Self {
            repo_path,
            repo,
            cache: Arc::new(Mutex::new(CachedGitInfo {
                info: None,
                last_update: Instant::now() - Duration::from_secs(10),
                cache_duration: Duration::from_millis(500),
            })),
        }
    }

    fn find_repository(path: &Path) -> (Option<PathBuf>, Option<gix::Repository>) {
        gix::discover(path)
            .ok()
            .map(|repo| {
                let workdir = repo.work_dir().map(|p| p.to_path_buf());
                (workdir, Some(repo))
            })
            .unwrap_or((None, None))
    }

    pub fn is_git_repo(&self) -> bool {
        self.repo.is_some()
    }

    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_path.as_deref()
    }

    pub fn get_status(&self) -> Result<GitInfo> {
        let mut cache = self.cache.lock().unwrap();

        // Return cached info if still valid
        if let Some(info) = &cache.info {
            if cache.last_update.elapsed() < cache.cache_duration {
                return Ok(info.clone());
            }
        }

        // Update cache
        let info = self.fetch_status()?;
        cache.info = Some(info.clone());
        cache.last_update = Instant::now();

        Ok(info)
    }

    pub fn invalidate_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.last_update = Instant::now() - Duration::from_secs(10);
    }

    fn fetch_status(&self) -> Result<GitInfo> {
        let repo = self.repo.as_ref().context("No git repository")?;

        // Get branch info
        let branch = self.get_branch_name(repo)?;

        // Get ahead/behind counts
        let (ahead, behind) = self.get_ahead_behind(repo).unwrap_or((0, 0));

        // Get file statuses using gix status
        let (file_statuses, has_conflicts) = self.get_file_statuses(repo)?;

        Ok(GitInfo {
            branch,
            ahead,
            behind,
            has_conflicts,
            file_statuses,
        })
    }

    fn get_branch_name(&self, repo: &gix::Repository) -> Result<String> {
        let head = repo.head().context("Failed to get HEAD")?;

        Ok(match head.kind {
            gix::head::Kind::Symbolic(ref reference) => reference.name.shorten().to_string(),
            gix::head::Kind::Unborn(ref name) => name.shorten().to_string(),
            gix::head::Kind::Detached { .. } => "detached".to_string(),
        })
    }

    fn get_ahead_behind(&self, repo: &gix::Repository) -> Result<(usize, usize)> {
        let head = repo.head().context("Failed to get HEAD")?;

        // Only calculate ahead/behind for branch heads
        let branch_ref = match &head.kind {
            gix::head::Kind::Symbolic(reference) => reference.clone(),
            _ => return Ok((0, 0)),
        };

        // Get the local commit
        let local_id = match head.id() {
            Some(id) => id.detach(),
            None => return Ok((0, 0)),
        };

        // Try to find upstream tracking branch
        let branch_name = branch_ref.name.shorten();
        let upstream_name = format!("refs/remotes/origin/{}", branch_name);

        let upstream_ref = match repo.find_reference(&upstream_name) {
            Ok(r) => r,
            Err(_) => return Ok((0, 0)),
        };

        let upstream_id = upstream_ref.id().detach();

        // Calculate ahead/behind using revision walk
        let (ahead, behind) = self.count_ahead_behind(repo, local_id, upstream_id)?;

        Ok((ahead, behind))
    }

    fn count_ahead_behind(
        &self,
        repo: &gix::Repository,
        local: gix::ObjectId,
        upstream: gix::ObjectId,
    ) -> Result<(usize, usize)> {
        // Use simpler approach - just count unique commits on each side
        let ahead = self.count_commits_reachable_from_not_from(repo, local, upstream)?;
        let behind = self.count_commits_reachable_from_not_from(repo, upstream, local)?;

        Ok((ahead, behind))
    }

    fn count_commits_reachable_from_not_from(
        &self,
        repo: &gix::Repository,
        include: gix::ObjectId,
        exclude: gix::ObjectId,
    ) -> Result<usize> {
        if include == exclude {
            return Ok(0);
        }

        // Walk from `include` and count commits not reachable from `exclude`
        let mut count = 0;

        // Get all commits reachable from exclude for filtering
        let mut excluded_commits = std::collections::HashSet::new();
        if let Ok(walk) = repo
            .rev_walk([exclude])
            .sorting(gix::revision::walk::Sorting::ByCommitTime(Default::default()))
            .all()
        {
            for commit_result in walk.take(1000) {
                // Limit to prevent slowdowns
                if let Ok(commit) = commit_result {
                    excluded_commits.insert(commit.id);
                }
            }
        }

        // Walk from include and count non-excluded
        if let Ok(walk) = repo
            .rev_walk([include])
            .sorting(gix::revision::walk::Sorting::ByCommitTime(Default::default()))
            .all()
        {
            for commit_result in walk.take(1000) {
                if let Ok(commit) = commit_result {
                    if !excluded_commits.contains(&commit.id) {
                        count += 1;
                    } else {
                        // Once we hit an excluded commit, we've found merge base area
                        break;
                    }
                }
            }
        }

        Ok(count)
    }

    fn get_file_statuses(
        &self,
        repo: &gix::Repository,
    ) -> Result<(HashMap<PathBuf, FileStatus>, bool)> {
        let mut file_statuses = HashMap::new();
        let mut has_conflicts = false;

        // Use gix status iterator with high-level API
        let status_platform = repo
            .status(gix::progress::Discard)
            .context("Failed to create status")?;

        let status_iter = status_platform
            .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())
            .context("Failed to get status iterator")?;

        for item_result in status_iter {
            let item = match item_result {
                Ok(item) => item,
                Err(_) => continue,
            };

            // Use the summary() method to get a simplified status
            if let Some(summary) = item.summary() {
                use gix::status::index_worktree::iter::Summary;

                let (path, file_status) = match &item {
                    gix::status::index_worktree::iter::Item::Modification { rela_path, .. } => {
                        let status = match summary {
                            Summary::Removed => FileStatus::Deleted,
                            Summary::Modified => FileStatus::Modified,
                            Summary::TypeChange => FileStatus::Modified,
                            Summary::Conflict => {
                                has_conflicts = true;
                                FileStatus::Conflicted
                            }
                            Summary::IntentToAdd => FileStatus::Added,
                            _ => FileStatus::Unknown,
                        };
                        (PathBuf::from(rela_path.to_string()), status)
                    }
                    gix::status::index_worktree::iter::Item::DirectoryContents { entry, .. } => {
                        (
                            PathBuf::from(entry.rela_path.to_string()),
                            FileStatus::Untracked,
                        )
                    }
                    gix::status::index_worktree::iter::Item::Rewrite {
                        source, copy, ..
                    } => {
                        let status = if *copy {
                            FileStatus::Added
                        } else {
                            FileStatus::Renamed
                        };
                        (PathBuf::from(source.rela_path().to_string()), status)
                    }
                };

                file_statuses.insert(path, file_status);
            }
        }

        // Check for conflicts via index
        if let Ok(index) = repo.index() {
            for entry in index.entries() {
                if entry.flags.stage() != gix::index::entry::Stage::Unconflicted {
                    has_conflicts = true;
                    let path = PathBuf::from(entry.path(&index).to_string());
                    file_statuses.insert(path, FileStatus::Conflicted);
                }
            }
        }

        Ok((file_statuses, has_conflicts))
    }

    pub fn get_file_status(&self, path: &Path) -> Result<Option<FileStatus>> {
        let info = self.get_status()?;

        // Try exact match first
        if let Some(status) = info.file_statuses.get(path) {
            return Ok(Some(status.clone()));
        }

        // Try relative to repo root
        if let Some(repo_root) = self.repo_root() {
            if let Ok(rel_path) = path.strip_prefix(repo_root) {
                if let Some(status) = info.file_statuses.get(rel_path) {
                    return Ok(Some(status.clone()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper to create initial commit using git command (more reliable than gix API)
    fn create_initial_commit(temp_dir: &Path) {
        use std::process::Command;

        // Configure git user for this repo
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir)
            .output()
            .ok();

        // Create an empty commit
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial commit"])
            .current_dir(temp_dir)
            .output()
            .ok();
    }

    // ============================================
    // FileStatus Unit Tests
    // ============================================

    #[test]
    fn test_file_status_to_short_str_all_variants() {
        assert_eq!(FileStatus::Modified.to_short_str(), "M");
        assert_eq!(FileStatus::Added.to_short_str(), "A");
        assert_eq!(FileStatus::Deleted.to_short_str(), "D");
        assert_eq!(FileStatus::Renamed.to_short_str(), "R");
        assert_eq!(FileStatus::Untracked.to_short_str(), "?");
        assert_eq!(FileStatus::Conflicted.to_short_str(), "U");
        assert_eq!(FileStatus::Staged.to_short_str(), "S");
        assert_eq!(FileStatus::Unknown.to_short_str(), " ");
    }

    #[test]
    fn test_file_status_to_colored_str_not_empty() {
        // Colored strings should not be empty
        assert!(!FileStatus::Modified.to_colored_str().is_empty());
        assert!(!FileStatus::Added.to_colored_str().is_empty());
        assert!(!FileStatus::Deleted.to_colored_str().is_empty());
        assert!(!FileStatus::Renamed.to_colored_str().is_empty());
        assert!(!FileStatus::Untracked.to_colored_str().is_empty());
        assert!(!FileStatus::Conflicted.to_colored_str().is_empty());
        assert!(!FileStatus::Staged.to_colored_str().is_empty());
        // Unknown is a space, which is technically not empty
        assert_eq!(FileStatus::Unknown.to_colored_str().trim(), "");
    }

    #[test]
    fn test_file_status_equality() {
        assert_eq!(FileStatus::Modified, FileStatus::Modified);
        assert_ne!(FileStatus::Modified, FileStatus::Added);
        assert_ne!(FileStatus::Staged, FileStatus::Untracked);
    }

    #[test]
    fn test_file_status_clone() {
        let status = FileStatus::Modified;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    // ============================================
    // GitInfo Unit Tests
    // ============================================

    #[test]
    fn test_git_info_default_values() {
        let info = GitInfo {
            branch: "main".to_string(),
            ahead: 0,
            behind: 0,
            has_conflicts: false,
            file_statuses: HashMap::new(),
        };

        assert_eq!(info.branch, "main");
        assert_eq!(info.ahead, 0);
        assert_eq!(info.behind, 0);
        assert!(!info.has_conflicts);
        assert!(info.file_statuses.is_empty());
    }

    #[test]
    fn test_git_info_with_file_statuses() {
        let mut statuses = HashMap::new();
        statuses.insert(PathBuf::from("file1.rs"), FileStatus::Modified);
        statuses.insert(PathBuf::from("file2.rs"), FileStatus::Untracked);

        let info = GitInfo {
            branch: "feature".to_string(),
            ahead: 2,
            behind: 1,
            has_conflicts: false,
            file_statuses: statuses,
        };

        assert_eq!(info.file_statuses.len(), 2);
        assert_eq!(
            info.file_statuses.get(&PathBuf::from("file1.rs")),
            Some(&FileStatus::Modified)
        );
    }

    #[test]
    fn test_git_info_clone() {
        let mut statuses = HashMap::new();
        statuses.insert(PathBuf::from("test.rs"), FileStatus::Staged);

        let info = GitInfo {
            branch: "main".to_string(),
            ahead: 1,
            behind: 0,
            has_conflicts: true,
            file_statuses: statuses,
        };

        let cloned = info.clone();
        assert_eq!(info.branch, cloned.branch);
        assert_eq!(info.ahead, cloned.ahead);
        assert_eq!(info.has_conflicts, cloned.has_conflicts);
    }

    // ============================================
    // GitMonitor Unit Tests
    // These tests use gix (pure Rust) so no linking issues
    // ============================================

    #[test]
    fn test_git_monitor_non_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        let monitor = GitMonitor::new(temp_dir.path());

        assert!(!monitor.is_git_repo());
        assert!(monitor.repo_root().is_none());
    }

    #[test]
    fn test_git_monitor_with_git_repo() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize a git repository using git command (more reliable)
        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        create_initial_commit(temp_dir.path());

        let monitor = GitMonitor::new(temp_dir.path());

        assert!(monitor.is_git_repo());
        assert!(monitor.repo_root().is_some());
    }

    #[test]
    fn test_git_monitor_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let monitor = GitMonitor::new(temp_dir.path());

        // Invalidate cache
        monitor.invalidate_cache();

        // Should not panic and cache should be marked as stale
        let cache = monitor.cache.lock().unwrap();
        assert!(cache.last_update.elapsed() > Duration::from_secs(5));
    }

    #[test]
    fn test_git_monitor_get_file_status_no_repo() {
        let temp_dir = TempDir::new().unwrap();
        let monitor = GitMonitor::new(temp_dir.path());

        let result = monitor.get_file_status(Path::new("test.rs"));
        assert!(result.is_err());
    }

    #[test]
    fn test_git_monitor_find_repository_in_subdir() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create a subdirectory
        let subdir = temp_dir.path().join("src").join("nested");
        fs::create_dir_all(&subdir).unwrap();

        // Monitor should find repo from subdirectory
        let monitor = GitMonitor::new(&subdir);
        assert!(monitor.is_git_repo());
    }

    // ============================================
    // Integration Tests with Real Git Repo
    // ============================================

    #[test]
    fn test_git_monitor_detects_untracked_file() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        create_initial_commit(temp_dir.path());

        // Create an untracked file
        let file_path = temp_dir.path().join("untracked.txt");
        fs::write(&file_path, "test content").unwrap();

        let monitor = GitMonitor::new(temp_dir.path());
        let info = monitor.get_status().unwrap();

        assert!(
            info.file_statuses
                .values()
                .any(|s| *s == FileStatus::Untracked)
        );
    }

    #[test]
    fn test_git_monitor_detects_modified_file() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_dir.path())
            .output()
            .ok();

        // Create and commit a file
        let file_path = temp_dir.path().join("tracked.txt");
        fs::write(&file_path, "initial content").unwrap();

        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Add tracked file"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Modify the tracked file
        fs::write(&file_path, "modified content").unwrap();

        let monitor = GitMonitor::new(temp_dir.path());
        let info = monitor.get_status().unwrap();

        assert!(
            info.file_statuses
                .values()
                .any(|s| *s == FileStatus::Modified)
        );
    }

    #[test]
    fn test_git_monitor_branch_detection() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        create_initial_commit(temp_dir.path());

        let monitor = GitMonitor::new(temp_dir.path());
        let info = monitor.get_status().unwrap();

        // Branch should be detected (usually "master" or "main")
        assert!(!info.branch.is_empty());
        // gix defaults to "main" in recent versions
        assert!(info.branch == "master" || info.branch == "main");
    }

    #[test]
    fn test_git_monitor_cache_returns_same_data() {
        let temp_dir = TempDir::new().unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        create_initial_commit(temp_dir.path());

        let monitor = GitMonitor::new(temp_dir.path());

        // First call - populates cache
        let info1 = monitor.get_status().unwrap();
        // Second call - should return cached data
        let info2 = monitor.get_status().unwrap();

        assert_eq!(info1.branch, info2.branch);
        assert_eq!(info1.ahead, info2.ahead);
        assert_eq!(info1.behind, info2.behind);
    }
}
