//! Integration tests for wezterm-watch
//!
//! These tests verify the interaction between FileWatcher, GitMonitor, and the filesystem.
//! They use real filesystem operations and git repositories to ensure correct behavior.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// Import directly from the library
use wezterm_watch::git::{FileStatus, GitMonitor};
use wezterm_watch::watcher::{FileWatcher, WatchEvent};

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Helper to create a git repository with initial commit
fn init_git_repo(path: &Path) -> Result<()> {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()?;

    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit"])
        .current_dir(path)
        .output()?;

    Ok(())
}

/// Helper to wait for and collect events with timeout
fn collect_events(
    watcher: &FileWatcher,
    timeout: Duration,
    max_events: usize,
) -> Vec<WatchEvent> {
    let receiver = watcher.receiver();
    let mut events = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < timeout && events.len() < max_events {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => events.push(event),
            Err(_) => continue,
        }
    }

    events
}

/// Helper to wait for a specific event type
fn wait_for_event_type(
    watcher: &FileWatcher,
    timeout: Duration,
    expected_type: &str,
) -> Option<WatchEvent> {
    let receiver = watcher.receiver();
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if event.event_type() == expected_type {
                    return Some(event);
                }
            }
            Err(_) => continue,
        }
    }

    None
}

// ==============================================================================
// Integration Test 1: File Change Detection
// ==============================================================================

#[test]
fn test_file_change_detection() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Create watcher with short debounce for faster tests
    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, vec![]).unwrap();

    watcher.watch(true).unwrap();

    // Wait a moment for watcher to initialize
    thread::sleep(Duration::from_millis(100));

    // Test 1: Create a file
    let file1 = watch_path.join("test_file.txt");
    fs::write(&file1, "test content").unwrap();

    // Wait for create event
    let event = wait_for_event_type(&watcher, Duration::from_secs(2), "created");
    assert!(
        event.is_some() || wait_for_event_type(&watcher, Duration::from_secs(1), "modified").is_some(),
        "Should detect file creation"
    );

    // Wait for filesystem to settle
    thread::sleep(Duration::from_millis(200));

    // Test 2: Modify the file
    fs::write(&file1, "modified content").unwrap();

    let event = wait_for_event_type(&watcher, Duration::from_secs(2), "modified");
    assert!(event.is_some(), "Should detect file modification");

    // Wait for filesystem to settle
    thread::sleep(Duration::from_millis(200));

    // Test 3: Delete the file
    fs::remove_file(&file1).unwrap();

    let event = wait_for_event_type(&watcher, Duration::from_secs(2), "deleted");
    assert!(event.is_some(), "Should detect file deletion");
}

#[test]
fn test_directory_operations() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create a directory
    let subdir = watch_path.join("subdir");
    fs::create_dir(&subdir).unwrap();

    // Create a file in the subdirectory
    let file_in_subdir = subdir.join("nested_file.txt");
    fs::write(&file_in_subdir, "nested content").unwrap();

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(2), 5);

    // Should detect at least one event (directory or file creation)
    assert!(!events.is_empty(), "Should detect nested file operations");

    // Verify at least one event relates to our file
    let has_relevant_event = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.ends_with("nested_file.txt") || path.ends_with("subdir")
        } else {
            false
        }
    });

    assert!(has_relevant_event, "Should detect events in subdirectory");
}

#[test]
fn test_multiple_file_changes() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create multiple files
    for i in 0..5 {
        let file = watch_path.join(format!("file_{}.txt", i));
        fs::write(&file, format!("content {}", i)).unwrap();
        thread::sleep(Duration::from_millis(50));
    }

    // Collect all events
    let events = collect_events(&watcher, Duration::from_secs(3), 10);

    // Should detect multiple events (at least 5 for the 5 files)
    assert!(
        events.len() >= 5,
        "Should detect multiple file creations, got {}",
        events.len()
    );
}

// ==============================================================================
// Integration Test 2: Debouncing Behavior
// ==============================================================================

#[test]
fn test_debouncing_rapid_changes() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Use longer debounce interval to test debouncing
    let mut watcher = FileWatcher::new(watch_path.clone(), 300, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    let file = watch_path.join("rapid_change.txt");

    // Create the file
    fs::write(&file, "initial").unwrap();

    // Make rapid modifications within debounce window
    for i in 1..=10 {
        thread::sleep(Duration::from_millis(20));
        fs::write(&file, format!("update {}", i)).unwrap();
    }

    // Wait for debounce to complete
    thread::sleep(Duration::from_millis(500));

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(1), 20);

    // Due to debouncing, we should have fewer events than modifications
    // With 300ms debounce and 20ms between changes, the 10 rapid changes
    // should be debounced into 1-3 events
    assert!(
        events.len() < 10,
        "Debouncing should reduce events, got {} events for 10+ changes",
        events.len()
    );
}

#[test]
fn test_debouncing_with_delays() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    let mut watcher = FileWatcher::new(watch_path.clone(), 150, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    let file = watch_path.join("delayed_change.txt");

    // Make changes with delays longer than debounce interval
    fs::write(&file, "change 1").unwrap();
    thread::sleep(Duration::from_millis(300)); // Wait for debounce

    fs::write(&file, "change 2").unwrap();
    thread::sleep(Duration::from_millis(300));

    fs::write(&file, "change 3").unwrap();
    thread::sleep(Duration::from_millis(300));

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(1), 10);

    // Should get separate events for each change since we waited
    assert!(
        events.len() >= 3,
        "Should detect separate events when delays exceed debounce, got {}",
        events.len()
    );
}

// ==============================================================================
// Integration Test 3: Gitignore Filtering
// ==============================================================================

#[test]
fn test_gitignore_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Create a .gitignore file
    let gitignore_content = "*.log\n*.tmp\nignored_dir/\n";
    fs::write(watch_path.join(".gitignore"), gitignore_content).unwrap();

    // Create watcher with gitignore enabled
    let mut watcher = FileWatcher::new(watch_path.clone(), 50, true, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create a file that should be ignored
    let ignored_file = watch_path.join("test.log");
    fs::write(&ignored_file, "log content").unwrap();

    // Create a file that should NOT be ignored
    let tracked_file = watch_path.join("test.txt");
    fs::write(&tracked_file, "tracked content").unwrap();

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(2), 10);

    // Verify that ignored file didn't generate an event
    let has_ignored = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.ends_with("test.log")
        } else {
            false
        }
    });

    // Verify that tracked file did generate an event
    let has_tracked = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.ends_with("test.txt")
        } else {
            false
        }
    });

    assert!(!has_ignored, "Should NOT detect ignored .log file");
    assert!(has_tracked, "Should detect tracked .txt file");
}

#[test]
fn test_custom_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Create watcher with custom ignore patterns (no gitignore)
    let custom_ignores = vec!["*.bak".to_string(), "*.cache".to_string()];
    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, custom_ignores).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create files matching custom ignore patterns
    fs::write(watch_path.join("file.bak"), "backup").unwrap();
    fs::write(watch_path.join("data.cache"), "cache").unwrap();

    // Create a normal file
    fs::write(watch_path.join("normal.txt"), "content").unwrap();

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(2), 10);

    // Count ignored vs tracked files
    let ignored_count = events
        .iter()
        .filter(|e| {
            if let Some(path) = e.path() {
                path.ends_with(".bak") || path.ends_with(".cache")
            } else {
                false
            }
        })
        .count();

    let tracked_count = events
        .iter()
        .filter(|e| {
            if let Some(path) = e.path() {
                path.ends_with(".txt")
            } else {
                false
            }
        })
        .count();

    assert_eq!(ignored_count, 0, "Should ignore custom patterns");
    assert!(tracked_count > 0, "Should track normal files");
}

#[test]
fn test_default_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Create watcher with gitignore (which adds default patterns)
    let mut watcher = FileWatcher::new(watch_path.clone(), 50, true, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create .git directory (should be ignored by default)
    let git_dir = watch_path.join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(git_dir.join("config"), "git config").unwrap();

    // Create target directory (should be ignored by default)
    let target_dir = watch_path.join("target");
    fs::create_dir(&target_dir).unwrap();
    fs::write(target_dir.join("build.txt"), "build").unwrap();

    // Create normal file
    fs::write(watch_path.join("src.rs"), "code").unwrap();

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(2), 10);

    // Should NOT see .git or target events
    let has_git = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.components().any(|c| c.as_os_str() == ".git")
        } else {
            false
        }
    });

    let has_target = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.components().any(|c| c.as_os_str() == "target")
        } else {
            false
        }
    });

    // Should see src.rs
    let has_source = events.iter().any(|e| {
        if let Some(path) = e.path() {
            path.ends_with("src.rs")
        } else {
            false
        }
    });

    assert!(!has_git, "Should ignore .git directory by default");
    assert!(!has_target, "Should ignore target directory by default");
    assert!(has_source, "Should detect source files");
}

// ==============================================================================
// Integration Test 4: Git Status Updates
// ==============================================================================

#[test]
fn test_git_status_integration() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Initialize git repository
    init_git_repo(&watch_path).unwrap();

    // Create and commit a file
    let tracked_file = watch_path.join("tracked.txt");
    fs::write(&tracked_file, "initial content").unwrap();

    Command::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&watch_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Add tracked file"])
        .current_dir(&watch_path)
        .output()
        .unwrap();

    // Wait for git operations to complete
    thread::sleep(Duration::from_millis(200));

    // Initialize GitMonitor
    let monitor = GitMonitor::new(&watch_path);
    assert!(monitor.is_git_repo(), "Should detect git repository");

    // Verify initial status (no changes)
    let status = monitor.get_status().unwrap();
    assert_eq!(status.branch, "master".to_string());
    assert_eq!(status.file_statuses.len(), 0, "No changes initially");

    // Modify the tracked file
    fs::write(&tracked_file, "modified content").unwrap();

    // Invalidate cache and check status
    monitor.invalidate_cache();
    thread::sleep(Duration::from_millis(100));

    let status = monitor.get_status().unwrap();
    assert!(
        status.file_statuses.len() > 0,
        "Should detect modified file"
    );

    // Verify file status is Modified
    let file_status = monitor.get_file_status(Path::new("tracked.txt")).unwrap();
    assert!(file_status.is_some(), "Should have status for modified file");
    assert_eq!(file_status.unwrap(), FileStatus::Modified);
}

#[test]
fn test_git_status_untracked_files() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    init_git_repo(&watch_path).unwrap();

    let monitor = GitMonitor::new(&watch_path);

    // Create an untracked file
    let untracked_file = watch_path.join("untracked.txt");
    fs::write(&untracked_file, "untracked content").unwrap();

    // Force cache invalidation
    monitor.invalidate_cache();
    thread::sleep(Duration::from_millis(100));

    let status = monitor.get_status().unwrap();

    // Check if we have untracked files
    let untracked_count = status
        .file_statuses
        .values()
        .filter(|s| **s == FileStatus::Untracked)
        .count();

    assert!(untracked_count > 0, "Should detect untracked files");
}

#[test]
fn test_git_watcher_integration() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    init_git_repo(&watch_path).unwrap();

    // Create watcher (without gitignore to see all events)
    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    // Create GitMonitor
    let monitor = GitMonitor::new(&watch_path);

    thread::sleep(Duration::from_millis(100));

    // Create a new file
    let new_file = watch_path.join("new_file.txt");
    fs::write(&new_file, "new content").unwrap();

    // Wait for file system event
    thread::sleep(Duration::from_millis(300));

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(1), 5);
    assert!(!events.is_empty(), "Should detect file creation");

    // Check git status
    monitor.invalidate_cache();
    let git_status = monitor.get_file_status(Path::new("new_file.txt")).unwrap();
    assert!(git_status.is_some(), "Should have git status");
    assert_eq!(git_status.unwrap(), FileStatus::Untracked);
}

#[test]
fn test_git_branch_detection() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    init_git_repo(&watch_path).unwrap();

    let monitor = GitMonitor::new(&watch_path);
    let status = monitor.get_status().unwrap();

    // Should detect master or main branch
    assert!(
        status.branch == "master" || status.branch == "main",
        "Should detect default branch, got: {}",
        status.branch
    );
}

#[test]
fn test_git_cache_behavior() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    init_git_repo(&watch_path).unwrap();

    let monitor = GitMonitor::new(&watch_path);

    // First call - populates cache
    let status1 = monitor.get_status().unwrap();

    // Second call - should return cached (very fast)
    let start = std::time::Instant::now();
    let status2 = monitor.get_status().unwrap();
    let elapsed = start.elapsed();

    // Cached call should be very fast (< 50ms)
    assert!(elapsed < Duration::from_millis(50), "Cache should be fast");
    assert_eq!(status1.branch, status2.branch);

    // Invalidate cache
    monitor.invalidate_cache();

    // Next call should fetch fresh data
    let status3 = monitor.get_status().unwrap();
    assert_eq!(status1.branch, status3.branch);
}

// ==============================================================================
// Edge Cases and Error Handling
// ==============================================================================

#[test]
fn test_watcher_on_nonexistent_path() {
    let nonexistent = PathBuf::from("/nonexistent/path/that/does/not/exist");
    let result = FileWatcher::new(nonexistent, 100, false, vec![]);

    // Should handle gracefully (might succeed or fail depending on implementation)
    // If it succeeds, watching should fail
    if let Ok(mut watcher) = result {
        let watch_result = watcher.watch(true);
        assert!(
            watch_result.is_err(),
            "Watching nonexistent path should fail"
        );
    }
}

#[test]
fn test_git_monitor_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let monitor = GitMonitor::new(temp_dir.path());

    assert!(!monitor.is_git_repo());
    assert!(monitor.repo_root().is_none());

    let result = monitor.get_status();
    assert!(result.is_err(), "Should fail on non-git directory");
}

#[test]
fn test_watcher_with_permission_errors() {
    // This test is platform-specific and may not work on all systems
    // Skip on Windows where permissions work differently
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();

        // Create a file with restricted permissions
        let restricted_file = watch_path.join("restricted.txt");
        fs::write(&restricted_file, "content").unwrap();

        let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
        perms.set_mode(0o000); // Remove all permissions
        fs::set_permissions(&restricted_file, perms).unwrap();

        // Watcher should still work, just might not access restricted file
        let mut watcher = FileWatcher::new(watch_path, 50, false, vec![]).unwrap();
        assert!(watcher.watch(true).is_ok());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&restricted_file, perms).unwrap();
    }
}

// ==============================================================================
// Concurrency and Stress Tests
// ==============================================================================

#[test]
fn test_concurrent_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    let mut watcher = FileWatcher::new(watch_path.clone(), 100, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create files concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let path = watch_path.clone();
            thread::spawn(move || {
                let file = path.join(format!("concurrent_{}.txt", i));
                fs::write(&file, format!("content {}", i)).unwrap();
                thread::sleep(Duration::from_millis(50));
                fs::write(&file, format!("updated {}", i)).unwrap();
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Collect events
    let events = collect_events(&watcher, Duration::from_secs(3), 20);

    // Should detect multiple concurrent operations
    assert!(
        events.len() >= 5,
        "Should detect concurrent file operations"
    );
}

#[test]
fn test_watcher_stability_under_load() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    let mut watcher = FileWatcher::new(watch_path.clone(), 50, false, vec![]).unwrap();
    watcher.watch(true).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Create many files rapidly
    for i in 0..50 {
        let file = watch_path.join(format!("stress_{}.txt", i));
        fs::write(&file, format!("content {}", i)).unwrap();

        if i % 10 == 0 {
            thread::sleep(Duration::from_millis(50));
        }
    }

    // Collect events (may not get all due to debouncing)
    let events = collect_events(&watcher, Duration::from_secs(5), 100);

    // Should detect a significant number of events without crashing
    assert!(events.len() >= 20, "Should handle high load");

    // Verify no error events
    let error_count = events
        .iter()
        .filter(|e| matches!(e, WatchEvent::Error(_)))
        .count();
    assert_eq!(error_count, 0, "Should not have errors under load");
}
