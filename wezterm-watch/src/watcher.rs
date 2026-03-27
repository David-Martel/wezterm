use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
    Error(String),
}

impl WatchEvent {
    pub fn path(&self) -> Option<&Path> {
        match self {
            WatchEvent::Created(p) | WatchEvent::Modified(p) | WatchEvent::Deleted(p) => Some(p),
            WatchEvent::Renamed { to, .. } => Some(to),
            WatchEvent::Error(_) => None,
        }
    }

    /// Get the event type as a string (primarily for testing and debugging)
    pub fn event_type(&self) -> &str {
        match self {
            WatchEvent::Created(_) => "created",
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Deleted(_) => "deleted",
            WatchEvent::Renamed { .. } => "renamed",
            WatchEvent::Error(_) => "error",
        }
    }
}

pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    receiver: Receiver<WatchEvent>,
    #[expect(
        dead_code,
        reason = "stored for potential future use in dynamic filter updates"
    )]
    gitignore: Option<Gitignore>,
    watch_path: PathBuf,
}

impl FileWatcher {
    pub fn new(
        path: PathBuf,
        debounce_ms: u64,
        use_gitignore: bool,
        custom_ignores: Vec<String>,
    ) -> Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();

        // Load gitignore rules
        let gitignore = if use_gitignore {
            Self::load_gitignore(&path, custom_ignores)?
        } else if !custom_ignores.is_empty() {
            Self::build_custom_ignore(&path, custom_ignores)?
        } else {
            None
        };

        let tx_clone = tx.clone();
        let gitignore_clone = gitignore.clone();
        let watch_path_clone = path.clone();

        let debouncer = new_debouncer(
            Duration::from_millis(debounce_ms),
            None,
            move |result: DebounceEventResult| {
                Self::handle_events(result, &tx_clone, &gitignore_clone, &watch_path_clone);
            },
        )
        .context("Failed to create debouncer")?;

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
            gitignore,
            watch_path: path,
        })
    }

    pub fn watch(&mut self, recursive: bool) -> Result<()> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self._debouncer
            .watcher()
            .watch(&self.watch_path, mode)
            .context("Failed to start watching")?;

        Ok(())
    }

    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        &self.receiver
    }

    fn handle_events(
        result: DebounceEventResult,
        sender: &Sender<WatchEvent>,
        gitignore: &Option<Gitignore>,
        base_path: &Path,
    ) {
        match result {
            Ok(events) => {
                for event in events {
                    if let Some(watch_event) =
                        Self::convert_event(event.event, gitignore, base_path)
                    {
                        let _ = sender.send(watch_event);
                    }
                }
            }
            Err(errors) => {
                for error in errors {
                    let _ = sender.send(WatchEvent::Error(error.to_string()));
                }
            }
        }
    }

    fn convert_event(
        event: Event,
        gitignore: &Option<Gitignore>,
        base_path: &Path,
    ) -> Option<WatchEvent> {
        // Filter ignored files
        if let Some(gi) = gitignore {
            for path in &event.paths {
                if let Ok(rel_path) = path.strip_prefix(base_path) {
                    if gi.matched(rel_path, path.is_dir()).is_ignore() {
                        return None;
                    }
                }
            }
        }

        match event.kind {
            EventKind::Create(_) => event
                .paths
                .first()
                .map(|path| WatchEvent::Created(path.clone())),
            EventKind::Modify(_) => event
                .paths
                .first()
                .map(|path| WatchEvent::Modified(path.clone())),
            EventKind::Remove(_) => event
                .paths
                .first()
                .map(|path| WatchEvent::Deleted(path.clone())),
            EventKind::Any => event
                .paths
                .first()
                .map(|path| WatchEvent::Modified(path.clone())),
            _ => None,
        }
    }

    fn load_gitignore(path: &Path, custom_ignores: Vec<String>) -> Result<Option<Gitignore>> {
        let mut builder = GitignoreBuilder::new(path);

        // Add .gitignore if it exists
        let gitignore_path = path.join(".gitignore");
        if gitignore_path.exists() {
            builder.add(gitignore_path);
        }

        // Add common ignore patterns
        builder.add_line(None, ".git")?;
        builder.add_line(None, "target/")?;
        builder.add_line(None, "node_modules/")?;
        builder.add_line(None, "*.swp")?;
        builder.add_line(None, "*.tmp")?;
        builder.add_line(None, ".DS_Store")?;

        // Add custom patterns
        for pattern in custom_ignores {
            builder.add_line(None, &pattern)?;
        }

        Ok(Some(builder.build()?))
    }

    fn build_custom_ignore(path: &Path, patterns: Vec<String>) -> Result<Option<Gitignore>> {
        let mut builder = GitignoreBuilder::new(path);

        for pattern in patterns {
            builder.add_line(None, &pattern)?;
        }

        Ok(Some(builder.build()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    // ============================================
    // WatchEvent Unit Tests
    // ============================================

    #[test]
    fn test_watch_event_type_created() {
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        assert_eq!(event.event_type(), "created");
    }

    #[test]
    fn test_watch_event_type_modified() {
        let event = WatchEvent::Modified(PathBuf::from("test.txt"));
        assert_eq!(event.event_type(), "modified");
    }

    #[test]
    fn test_watch_event_type_deleted() {
        let event = WatchEvent::Deleted(PathBuf::from("test.txt"));
        assert_eq!(event.event_type(), "deleted");
    }

    #[test]
    fn test_watch_event_type_renamed() {
        let event = WatchEvent::Renamed {
            from: PathBuf::from("old.txt"),
            to: PathBuf::from("new.txt"),
        };
        assert_eq!(event.event_type(), "renamed");
    }

    #[test]
    fn test_watch_event_type_error() {
        let event = WatchEvent::Error("error message".to_string());
        assert_eq!(event.event_type(), "error");
    }

    #[test]
    fn test_watch_event_path_created() {
        let event = WatchEvent::Created(PathBuf::from("/path/to/file.txt"));
        let path = event.path();
        assert!(path.is_some());
        assert_eq!(path.unwrap(), Path::new("/path/to/file.txt"));
    }

    #[test]
    fn test_watch_event_path_modified() {
        let event = WatchEvent::Modified(PathBuf::from("test.rs"));
        let path = event.path();
        assert!(path.is_some());
        assert_eq!(path.unwrap(), Path::new("test.rs"));
    }

    #[test]
    fn test_watch_event_path_deleted() {
        let event = WatchEvent::Deleted(PathBuf::from("deleted_file.txt"));
        let path = event.path();
        assert!(path.is_some());
        assert_eq!(path.unwrap(), Path::new("deleted_file.txt"));
    }

    #[test]
    fn test_watch_event_path_renamed() {
        let event = WatchEvent::Renamed {
            from: PathBuf::from("old_name.txt"),
            to: PathBuf::from("new_name.txt"),
        };
        let path = event.path();
        assert!(path.is_some());
        // Renamed events return the 'to' path
        assert_eq!(path.unwrap(), Path::new("new_name.txt"));
    }

    #[test]
    fn test_watch_event_path_error() {
        let event = WatchEvent::Error("some error".to_string());
        let path = event.path();
        assert!(path.is_none());
    }

    #[test]
    fn test_watch_event_clone() {
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let cloned = event.clone();

        assert_eq!(event.event_type(), cloned.event_type());
        assert_eq!(event.path(), cloned.path());
    }

    #[test]
    fn test_watch_event_debug() {
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Created"));
        assert!(debug_str.contains("test.txt"));
    }

    // ============================================
    // FileWatcher Creation Tests
    // ============================================

    #[test]
    fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path().to_path_buf(), 100, false, vec![]);

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_with_custom_debounce() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(
            temp_dir.path().to_path_buf(),
            500, // 500ms debounce
            false,
            vec![],
        );

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_with_gitignore() {
        let temp_dir = TempDir::new().unwrap();

        // Create a .gitignore file
        fs::write(temp_dir.path().join(".gitignore"), "*.log\n*.tmp\n").unwrap();

        let watcher = FileWatcher::new(
            temp_dir.path().to_path_buf(),
            100,
            true, // use gitignore
            vec![],
        );

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_with_custom_ignores() {
        let temp_dir = TempDir::new().unwrap();

        let watcher = FileWatcher::new(
            temp_dir.path().to_path_buf(),
            100,
            false,
            vec!["*.bak".to_string(), "cache/".to_string()],
        );

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_with_gitignore_and_custom_ignores() {
        let temp_dir = TempDir::new().unwrap();

        // Create a .gitignore file
        fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

        let watcher = FileWatcher::new(
            temp_dir.path().to_path_buf(),
            100,
            true,                         // use gitignore
            vec!["*.backup".to_string()], // additional ignores
        );

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_receiver_available() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path().to_path_buf(), 100, false, vec![]).unwrap();

        // Receiver should be accessible
        let _receiver = watcher.receiver();
    }

    // ============================================
    // FileWatcher Watch Tests
    // ============================================

    #[test]
    fn test_file_watcher_watch_recursive() {
        let temp_dir = TempDir::new().unwrap();

        // Create subdirectory
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let mut watcher =
            FileWatcher::new(temp_dir.path().to_path_buf(), 100, false, vec![]).unwrap();

        let result = watcher.watch(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_watcher_watch_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher =
            FileWatcher::new(temp_dir.path().to_path_buf(), 100, false, vec![]).unwrap();

        let result = watcher.watch(false);
        assert!(result.is_ok());
    }

    // ============================================
    // FileWatcher Integration Tests
    // ============================================

    #[test]
    #[ignore] // Ignore by default as this is a slow integration test
    fn test_file_watcher_detects_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(
            temp_dir.path().to_path_buf(),
            50, // Short debounce for faster test
            false,
            vec![],
        )
        .unwrap();

        watcher.watch(true).unwrap();
        let receiver = watcher.receiver().clone();

        // Create a file
        let file_path = temp_dir.path().join("new_file.txt");
        fs::write(&file_path, "test content").unwrap();

        // Wait for event with timeout
        let event = receiver.recv_timeout(Duration::from_secs(2));
        assert!(event.is_ok());

        match event.unwrap() {
            WatchEvent::Created(path) | WatchEvent::Modified(path) => {
                assert!(path.ends_with("new_file.txt"));
            }
            other => panic!("Unexpected event: {:?}", other),
        }
    }

    #[test]
    #[ignore] // Ignore by default as this is a slow integration test
    fn test_file_watcher_detects_file_modification() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial file
        let file_path = temp_dir.path().join("existing.txt");
        fs::write(&file_path, "initial content").unwrap();

        // Wait a bit for filesystem to settle
        thread::sleep(Duration::from_millis(100));

        let mut watcher =
            FileWatcher::new(temp_dir.path().to_path_buf(), 50, false, vec![]).unwrap();

        watcher.watch(true).unwrap();
        let receiver = watcher.receiver().clone();

        // Modify the file
        fs::write(&file_path, "modified content").unwrap();

        // Wait for event
        let event = receiver.recv_timeout(Duration::from_secs(2));
        assert!(event.is_ok());

        match event.unwrap() {
            WatchEvent::Modified(path) => {
                assert!(path.ends_with("existing.txt"));
            }
            WatchEvent::Created(path) => {
                // Some systems report modification as creation
                assert!(path.ends_with("existing.txt"));
            }
            other => panic!("Unexpected event: {:?}", other),
        }
    }

    #[test]
    #[ignore] // Ignore by default as this is a slow integration test
    fn test_file_watcher_detects_file_deletion() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial file
        let file_path = temp_dir.path().join("to_delete.txt");
        fs::write(&file_path, "will be deleted").unwrap();

        thread::sleep(Duration::from_millis(100));

        let mut watcher =
            FileWatcher::new(temp_dir.path().to_path_buf(), 50, false, vec![]).unwrap();

        watcher.watch(true).unwrap();
        let receiver = watcher.receiver().clone();

        // Delete the file
        fs::remove_file(&file_path).unwrap();

        // Wait for event
        let event = receiver.recv_timeout(Duration::from_secs(2));
        assert!(event.is_ok());

        match event.unwrap() {
            WatchEvent::Deleted(path) => {
                assert!(path.ends_with("to_delete.txt"));
            }
            other => panic!("Unexpected event: {:?}", other),
        }
    }

    // ============================================
    // Gitignore Pattern Tests
    // ============================================

    #[test]
    fn test_load_gitignore_default_patterns() {
        let temp_dir = TempDir::new().unwrap();

        // Even without a .gitignore file, default patterns should be added
        let gitignore = FileWatcher::load_gitignore(temp_dir.path(), vec![]);
        assert!(gitignore.is_ok());

        let gi = gitignore.unwrap();
        assert!(gi.is_some());

        let gi = gi.unwrap();
        // Should ignore .git directory
        assert!(gi.matched(Path::new(".git"), true).is_ignore());
        // Should ignore target directory
        assert!(gi.matched(Path::new("target"), true).is_ignore());
        // Should ignore node_modules
        assert!(gi.matched(Path::new("node_modules"), true).is_ignore());
    }

    #[test]
    fn test_load_gitignore_with_custom_patterns() {
        let temp_dir = TempDir::new().unwrap();

        let gitignore = FileWatcher::load_gitignore(
            temp_dir.path(),
            vec!["*.backup".to_string(), "temp/".to_string()],
        );

        assert!(gitignore.is_ok());
        let gi = gitignore.unwrap().unwrap();

        // Should ignore custom patterns
        assert!(gi.matched(Path::new("file.backup"), false).is_ignore());
    }

    #[test]
    fn test_build_custom_ignore() {
        let temp_dir = TempDir::new().unwrap();

        let gitignore = FileWatcher::build_custom_ignore(
            temp_dir.path(),
            vec!["*.log".to_string(), "cache/".to_string()],
        );

        assert!(gitignore.is_ok());
        let gi = gitignore.unwrap().unwrap();

        assert!(gi.matched(Path::new("debug.log"), false).is_ignore());
    }

    #[test]
    fn test_load_gitignore_from_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create a .gitignore file
        fs::write(
            temp_dir.path().join(".gitignore"),
            "*.custom\nbuild/\n!important.custom\n",
        )
        .unwrap();

        let gitignore = FileWatcher::load_gitignore(temp_dir.path(), vec![]);
        assert!(gitignore.is_ok());

        let gi = gitignore.unwrap().unwrap();
        assert!(gi.matched(Path::new("test.custom"), false).is_ignore());
        assert!(gi.matched(Path::new("build"), true).is_ignore());
    }
}
