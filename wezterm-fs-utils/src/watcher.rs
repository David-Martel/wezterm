//! File watching with debouncing using notify crate.
//!
//! This module borrows patterns from wezterm-watch/src/watcher.rs,
//! using crossbeam_channel for thread-safe event delivery.

use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Receiver, Sender};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
// Use the notify types re-exported from notify-debouncer-full to ensure compatibility
use notify_debouncer_full::notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::notify::Watcher as NotifyWatcher;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The kind of file system event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    /// A file or directory was created.
    Create,
    /// A file or directory was modified.
    Modify,
    /// A file or directory was deleted.
    Delete,
    /// A file or directory was renamed.
    Rename { from: PathBuf, to: PathBuf },
    /// An error occurred during watching.
    Error(String),
}

/// A file system watch event.
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// The kind of event.
    pub kind: WatchEventKind,
    /// The primary affected path.
    pub path: Option<PathBuf>,
}

impl WatchEvent {
    /// Get the affected path if available.
    pub fn path(&self) -> Option<&Path> {
        match &self.kind {
            WatchEventKind::Rename { to, .. } => Some(to.as_path()),
            _ => self.path.as_deref(),
        }
    }

    /// Create a create event.
    pub fn created(path: PathBuf) -> Self {
        Self {
            kind: WatchEventKind::Create,
            path: Some(path),
        }
    }

    /// Create a modify event.
    pub fn modified(path: PathBuf) -> Self {
        Self {
            kind: WatchEventKind::Modify,
            path: Some(path),
        }
    }

    /// Create a delete event.
    pub fn deleted(path: PathBuf) -> Self {
        Self {
            kind: WatchEventKind::Delete,
            path: Some(path),
        }
    }

    /// Create an error event.
    pub fn error(msg: String) -> Self {
        Self {
            kind: WatchEventKind::Error(msg),
            path: None,
        }
    }
}

/// Handle to a file watcher that can be used to stop watching.
#[derive(Debug)]
pub struct WatchHandle {
    path: PathBuf,
}

impl WatchHandle {
    /// Get the watched path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// File watcher with debouncing and gitignore support.
///
/// Pattern borrowed from wezterm-watch/src/watcher.rs.
pub struct Watcher {
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    receiver: Receiver<WatchEvent>,
    gitignore: Option<Gitignore>,
    watch_path: PathBuf,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watcher")
            .field("watch_path", &self.watch_path)
            .field("has_gitignore", &self.gitignore.is_some())
            .finish_non_exhaustive()
    }
}

impl Watcher {
    /// Create a new file watcher.
    ///
    /// # Arguments
    /// * `path` - The path to watch
    /// * `debounce_ms` - Debounce duration in milliseconds
    /// * `use_gitignore` - Whether to respect .gitignore files
    /// * `custom_ignores` - Additional patterns to ignore
    pub fn new(
        path: PathBuf,
        debounce_ms: u64,
        use_gitignore: bool,
        custom_ignores: Vec<String>,
    ) -> Result<Self> {
        let (tx, rx) = unbounded();

        // Load gitignore rules (pattern from wezterm-watch)
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
            debouncer,
            receiver: rx,
            gitignore,
            watch_path: path,
        })
    }

    /// Start watching the path.
    pub fn watch(&mut self, recursive: bool) -> Result<WatchHandle> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.debouncer
            .watcher()
            .watch(&self.watch_path, mode)
            .context("Failed to start watching")?;

        Ok(WatchHandle {
            path: self.watch_path.clone(),
        })
    }

    /// Stop watching (unwatch the path).
    pub fn unwatch(&mut self) -> Result<()> {
        self.debouncer
            .watcher()
            .unwatch(&self.watch_path)
            .context("Failed to unwatch")?;
        Ok(())
    }

    /// Get a reference to the event receiver.
    ///
    /// Use this to receive events in your event loop.
    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        &self.receiver
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.receiver.try_recv().ok()
    }

    /// Receive an event, blocking until one is available.
    pub fn recv(&self) -> Option<WatchEvent> {
        self.receiver.recv().ok()
    }

    /// Receive an event with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<WatchEvent> {
        self.receiver.recv_timeout(timeout).ok()
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
                    let _ = sender.send(WatchEvent::error(error.to_string()));
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
            EventKind::Create(_) => event.paths.first().map(|p| WatchEvent::created(p.clone())),
            EventKind::Modify(_) => event.paths.first().map(|p| WatchEvent::modified(p.clone())),
            EventKind::Remove(_) => event.paths.first().map(|p| WatchEvent::deleted(p.clone())),
            EventKind::Any => event.paths.first().map(|p| WatchEvent::modified(p.clone())),
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

        // Add common ignore patterns (from wezterm-watch)
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
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_watch_event_path() {
        let event = WatchEvent::created(PathBuf::from("/test/file.txt"));
        assert_eq!(event.path(), Some(Path::new("/test/file.txt")));

        let event = WatchEvent::error("test error".to_string());
        assert_eq!(event.path(), None);
    }

    #[test]
    fn test_watcher_creation() {
        let dir = TempDir::new().unwrap();
        let watcher = Watcher::new(dir.path().to_path_buf(), 100, false, vec![]);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_with_gitignore() {
        let dir = TempDir::new().unwrap();

        // Create a .gitignore file
        fs::write(dir.path().join(".gitignore"), "*.log\n*.tmp\n").unwrap();

        let watcher = Watcher::new(dir.path().to_path_buf(), 100, true, vec![]);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_with_custom_ignores() {
        let dir = TempDir::new().unwrap();
        let watcher = Watcher::new(
            dir.path().to_path_buf(),
            100,
            false,
            vec!["*.bak".to_string(), "cache/".to_string()],
        );
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_watch() {
        let dir = TempDir::new().unwrap();
        let mut watcher = Watcher::new(dir.path().to_path_buf(), 100, false, vec![]).unwrap();

        let handle = watcher.watch(true);
        assert!(handle.is_ok());
        assert_eq!(handle.unwrap().path(), dir.path());
    }

    #[test]
    #[ignore] // Integration test - run with --ignored
    fn test_watcher_detects_file_creation() {
        let dir = TempDir::new().unwrap();
        let mut watcher = Watcher::new(dir.path().to_path_buf(), 50, false, vec![]).unwrap();

        let _handle = watcher.watch(true).unwrap();

        // Create a file
        let file_path = dir.path().join("new_file.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        file.sync_all().unwrap();

        // Wait for event
        if let Some(event) = watcher.recv_timeout(Duration::from_secs(2)) {
            match event.kind {
                WatchEventKind::Create | WatchEventKind::Modify => {
                    assert!(event.path().is_some());
                }
                WatchEventKind::Error(e) => panic!("Got error event: {}", e),
                _ => {}
            }
        }
    }
}
