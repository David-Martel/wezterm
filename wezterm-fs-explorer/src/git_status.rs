use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub statuses: HashMap<PathBuf, GitFileStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
}

impl GitStatus {
    pub fn from_repo(path: &Path) -> Option<Self> {
        let repo = gix::discover(path).ok()?;
        let workdir = repo.work_dir()?.to_path_buf();
        let mut statuses = HashMap::new();

        // Use gix status iterator
        let status_platform = repo.status(gix::progress::Discard).ok()?;

        let status_iter = status_platform
            .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())
            .ok()?;

        for item_result in status_iter {
            let item = match item_result {
                Ok(item) => item,
                Err(_) => continue,
            };

            // Use the summary() method to get a simplified status
            if let Some(summary) = item.summary() {
                use gix::status::index_worktree::iter::Item;
                use gix::status::index_worktree::iter::Summary;

                let (rela_path, file_status) = match &item {
                    Item::Modification { rela_path, .. } => {
                        let status = match summary {
                            Summary::Removed => GitFileStatus::Deleted,
                            Summary::Modified => GitFileStatus::Modified,
                            Summary::TypeChange => GitFileStatus::Modified,
                            Summary::Conflict => GitFileStatus::Modified,
                            Summary::IntentToAdd => GitFileStatus::Added,
                            _ => GitFileStatus::Untracked,
                        };
                        (rela_path.to_string(), status)
                    }
                    Item::DirectoryContents { entry, .. } => {
                        use gix::dir::entry::Status;
                        let status = match entry.status {
                            Status::Ignored(_) => GitFileStatus::Ignored,
                            Status::Untracked => GitFileStatus::Untracked,
                            _ => GitFileStatus::Untracked,
                        };
                        (entry.rela_path.to_string(), status)
                    }
                    Item::Rewrite { source, copy, .. } => {
                        let status = if *copy {
                            GitFileStatus::Added
                        } else {
                            GitFileStatus::Renamed
                        };
                        (source.rela_path().to_string(), status)
                    }
                };

                let full_path = workdir.join(&rela_path);
                statuses.insert(full_path, file_status);
            }
        }

        Some(Self { statuses })
    }

    pub fn get_status(&self, path: &Path) -> Option<GitFileStatus> {
        self.statuses.get(path).copied()
    }

    pub fn get_indicator(&self, path: &Path) -> Option<&str> {
        self.get_status(path).map(|status| match status {
            GitFileStatus::Modified => "M",
            GitFileStatus::Added => "A",
            GitFileStatus::Deleted => "D",
            GitFileStatus::Renamed => "R",
            GitFileStatus::Untracked => "?",
            GitFileStatus::Ignored => "!",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper to create a git repo using git command
    fn init_git_repo(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .ok();
    }

    fn create_initial_commit(path: &Path) {
        use std::process::Command;
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .ok();
    }

    // ============================================
    // GitFileStatus Unit Tests
    // ============================================

    #[test]
    fn test_git_file_status_equality() {
        assert_eq!(GitFileStatus::Modified, GitFileStatus::Modified);
        assert_ne!(GitFileStatus::Modified, GitFileStatus::Added);
        assert_ne!(GitFileStatus::Untracked, GitFileStatus::Ignored);
    }

    #[test]
    fn test_git_file_status_clone() {
        let status = GitFileStatus::Modified;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_git_file_status_copy() {
        let status = GitFileStatus::Added;
        let copied: GitFileStatus = status;
        assert_eq!(status, copied);
    }

    // ============================================
    // GitStatus Unit Tests
    // ============================================

    #[test]
    fn test_git_status_from_non_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = GitStatus::from_repo(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_git_status_from_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());
        create_initial_commit(temp_dir.path());

        let result = GitStatus::from_repo(temp_dir.path());
        assert!(result.is_some());

        let status = result.unwrap();
        assert!(status.statuses.is_empty());
    }

    #[test]
    fn test_git_status_detects_untracked_file() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());
        create_initial_commit(temp_dir.path());

        // Create an untracked file
        let file_path = temp_dir.path().join("untracked.txt");
        fs::write(&file_path, "test content").unwrap();

        let result = GitStatus::from_repo(temp_dir.path());
        assert!(result.is_some());

        let status = result.unwrap();
        assert!(status
            .statuses
            .values()
            .any(|s| *s == GitFileStatus::Untracked));
    }

    #[test]
    fn test_git_status_detects_modified_file() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());

        // Create and commit a file
        let file_path = temp_dir.path().join("tracked.txt");
        fs::write(&file_path, "initial content").unwrap();

        use std::process::Command;
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add file"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Modify the file
        fs::write(&file_path, "modified content").unwrap();

        let result = GitStatus::from_repo(temp_dir.path());
        assert!(result.is_some());

        let status = result.unwrap();
        assert!(status
            .statuses
            .values()
            .any(|s| *s == GitFileStatus::Modified));
    }

    #[test]
    fn test_git_status_get_indicator() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());
        create_initial_commit(temp_dir.path());

        // Create an untracked file
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let result = GitStatus::from_repo(temp_dir.path());
        assert!(result.is_some());

        let status = result.unwrap();

        // Should have indicator for untracked file
        let indicator = status.get_indicator(&file_path);
        assert_eq!(indicator, Some("?"));
    }

    #[test]
    fn test_git_status_get_status_none() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());
        create_initial_commit(temp_dir.path());

        let result = GitStatus::from_repo(temp_dir.path());
        let status = result.unwrap();

        // Non-existent file should return None
        let file_status = status.get_status(Path::new("/nonexistent/file.txt"));
        assert!(file_status.is_none());
    }

    #[test]
    fn test_git_status_indicator_values() {
        // Test that all status indicators are correct
        let mut statuses = HashMap::new();
        let base = PathBuf::from("/test");

        statuses.insert(base.join("modified.txt"), GitFileStatus::Modified);
        statuses.insert(base.join("added.txt"), GitFileStatus::Added);
        statuses.insert(base.join("deleted.txt"), GitFileStatus::Deleted);
        statuses.insert(base.join("renamed.txt"), GitFileStatus::Renamed);
        statuses.insert(base.join("untracked.txt"), GitFileStatus::Untracked);
        statuses.insert(base.join("ignored.txt"), GitFileStatus::Ignored);

        let git_status = GitStatus { statuses };

        assert_eq!(
            git_status.get_indicator(&base.join("modified.txt")),
            Some("M")
        );
        assert_eq!(git_status.get_indicator(&base.join("added.txt")), Some("A"));
        assert_eq!(
            git_status.get_indicator(&base.join("deleted.txt")),
            Some("D")
        );
        assert_eq!(
            git_status.get_indicator(&base.join("renamed.txt")),
            Some("R")
        );
        assert_eq!(
            git_status.get_indicator(&base.join("untracked.txt")),
            Some("?")
        );
        assert_eq!(
            git_status.get_indicator(&base.join("ignored.txt")),
            Some("!")
        );
    }

    #[test]
    fn test_git_status_from_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());
        create_initial_commit(temp_dir.path());

        // Create a subdirectory
        let subdir = temp_dir.path().join("src").join("nested");
        fs::create_dir_all(&subdir).unwrap();

        // Should be able to find git repo from subdirectory
        let result = GitStatus::from_repo(&subdir);
        assert!(result.is_some());
    }
}
