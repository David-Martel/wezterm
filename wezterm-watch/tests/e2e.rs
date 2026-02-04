//! End-to-End (E2E) tests for wezterm-watch CLI
//!
//! These tests verify the CLI binary behavior by spawning the actual executable
//! and testing its command-line interface, output formats, and real-world usage.
//! All tests use timeouts to prevent hanging and are CI-compatible.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Get the binary name for the CLI
fn get_binary_name() -> &'static str {
    "wezterm-watch"
}

/// Get the path to the binary for spawning with std::process::Command
fn get_binary_path() -> PathBuf {
    // Use the same lookup logic as assert_cmd
    let mut path = std::env::current_exe()
        .expect("Failed to get current exe path")
        .parent()
        .expect("Failed to get parent dir")
        .parent()
        .expect("Failed to get deps dir")
        .to_path_buf();

    #[cfg(windows)]
    path.push("wezterm-watch.exe");
    #[cfg(not(windows))]
    path.push("wezterm-watch");

    // If not found, try target/debug or target/release
    if !path.exists() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| ".".to_string());
        let manifest_path = std::path::Path::new(&manifest_dir);

        #[cfg(windows)]
        {
            path = manifest_path.join("../target/debug/wezterm-watch.exe");
            if !path.exists() {
                path = manifest_path.join("../target/release/wezterm-watch.exe");
            }
        }
        #[cfg(not(windows))]
        {
            path = manifest_path.join("../target/debug/wezterm-watch");
            if !path.exists() {
                path = manifest_path.join("../target/release/wezterm-watch");
            }
        }
    }

    path
}

/// Helper to create a git repository with initial commit
fn init_git_repo(path: &std::path::Path) -> anyhow::Result<()> {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output()?;

    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()?;

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()?;

    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit"])
        .current_dir(path)
        .output()?;

    Ok(())
}

// ==============================================================================
// Test 1: CLI Output Formats with --status Flag
// ==============================================================================

#[test]
fn test_cli_status_json_format_in_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    // Initialize git repo
    init_git_repo(watch_path).unwrap();

    // Run CLI with --status and JSON format
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("json")
        .timeout(Duration::from_secs(5));

    let assert = cmd.assert().success();

    // Verify JSON output structure
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify expected fields
    assert!(json.get("git_branch").is_some(), "Should have git_branch field");
    assert!(json.get("git_ahead").is_some(), "Should have git_ahead field");
    assert!(json.get("git_behind").is_some(), "Should have git_behind field");
    assert!(json.get("has_conflicts").is_some(), "Should have has_conflicts field");
    assert!(json.get("modified_files").is_some(), "Should have modified_files field");
    assert!(json.get("untracked_files").is_some(), "Should have untracked_files field");
    assert!(json.get("staged_files").is_some(), "Should have staged_files field");
    assert!(json.get("total_files").is_some(), "Should have total_files field");
}

#[test]
fn test_cli_status_pretty_format_in_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    // Initialize git repo
    init_git_repo(watch_path).unwrap();

    // Run CLI with --status and pretty format (default)
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("pretty")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Branch:"))
        .stdout(predicate::str::contains("Files:"));
}

#[test]
fn test_cli_status_in_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    // Don't initialize git - just use empty directory

    // Run CLI with --status
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Not a git repository").or(predicate::str::contains("git disabled")));
}

#[test]
fn test_cli_status_with_invalid_format() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    init_git_repo(watch_path).unwrap();

    // Run CLI with invalid format
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("invalid_format")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid output format"));
}

#[test]
fn test_cli_status_summary_format() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    init_git_repo(watch_path).unwrap();

    // Run CLI with --status and summary format
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("summary")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[").and(predicate::str::contains("]")));
}

#[test]
fn test_cli_status_events_format() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    init_git_repo(watch_path).unwrap();

    // Run CLI with --status and events format
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("events")
        .timeout(Duration::from_secs(5));

    // Events format doesn't output git info, so output should be empty or whitespace only
    cmd.assert()
        .success()
        .stdout(predicate::function(|s: &str| s.trim().is_empty()));
}

// ==============================================================================
// Test 2: Directory Watching Basics (with timeout)
// ==============================================================================

#[test]
fn test_cli_watch_directory_with_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Spawn watcher in background using std::process::Command
    let mut child = StdCommand::new(get_binary_path())
        .arg(&watch_path)
        .arg("--format")
        .arg("json")
        .arg("--interval")
        .arg("50")
        .arg("--no-git")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn watcher");

    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(300));

    // Create a file
    let test_file = watch_path.join("test_file.txt");
    fs::write(&test_file, "test content").unwrap();

    // Wait for event to be processed
    thread::sleep(Duration::from_millis(500));

    // Kill the watcher
    child.kill().ok();

    // Wait for process to terminate and get output
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain at least one event
    assert!(
        stdout.contains(r#""event_type""#) || stdout.contains("created") || stdout.contains("modified"),
        "Should detect file creation event. Output: {}",
        stdout
    );
}

#[test]
fn test_cli_watch_pretty_format_output() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Spawn watcher with pretty format using std::process::Command
    let mut child = StdCommand::new(get_binary_path())
        .arg(&watch_path)
        .arg("--format")
        .arg("pretty")
        .arg("--no-git")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn watcher");

    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(300));

    // Create a file
    let test_file = watch_path.join("pretty_test.txt");
    fs::write(&test_file, "content").unwrap();

    // Wait for event
    thread::sleep(Duration::from_millis(500));

    // Kill and check output
    child.kill().ok();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Pretty format should contain "CREATED" or "MODIFIED"
    assert!(
        stdout.contains("CREATED") || stdout.contains("MODIFIED"),
        "Should show pretty formatted event. Output: {}",
        stdout
    );
}

#[test]
fn test_cli_watch_events_format_output() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Spawn watcher with events format using std::process::Command
    let mut child = StdCommand::new(get_binary_path())
        .arg(&watch_path)
        .arg("--format")
        .arg("events")
        .arg("--no-git")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn watcher");

    thread::sleep(Duration::from_millis(300));

    // Create a file
    let test_file = watch_path.join("events_test.txt");
    fs::write(&test_file, "content").unwrap();

    thread::sleep(Duration::from_millis(500));

    child.kill().ok();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Events format uses symbols: + (create), ~ (modify)
    assert!(
        stdout.contains("+") || stdout.contains("~"),
        "Should show events format output. Output: {}",
        stdout
    );
}

// ==============================================================================
// Test 3: Command-line Argument Parsing
// ==============================================================================

#[test]
fn test_cli_argument_format() {
    let temp_dir = TempDir::new().unwrap();

    // Test valid formats
    for format in &["json", "pretty", "events", "summary"] {
        let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
        cmd.arg(temp_dir.path())
            .arg("--status")
            .arg("--format")
            .arg(format)
            .arg("--no-git")
            .timeout(Duration::from_secs(5));

        cmd.assert().success();
    }

    // Test invalid format
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--format")
        .arg("xml")
        .timeout(Duration::from_secs(5));

    cmd.assert().failure();
}

#[test]
fn test_cli_argument_interval() {
    let temp_dir = TempDir::new().unwrap();

    // Valid interval values
    for interval in &["50", "100", "500", "1000"] {
        let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
        cmd.arg(temp_dir.path())
            .arg("--status")
            .arg("--interval")
            .arg(interval)
            .arg("--no-git")
            .timeout(Duration::from_secs(5));

        cmd.assert().success();
    }
}

#[test]
fn test_cli_argument_git_flags() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path()).unwrap();

    // Test --git flag
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--git")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Branch:").or(predicate::str::contains("git_branch")));

    // Test --no-git flag
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Not a git repository").or(predicate::str::contains("git disabled")));
}

#[test]
fn test_cli_argument_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Test --ignore flag with multiple patterns
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("--ignore")
        .arg("*.log")
        .arg("--ignore")
        .arg("*.tmp")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_argument_no_gitignore() {
    let temp_dir = TempDir::new().unwrap();

    // Test --no-gitignore flag
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("--no-gitignore")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_argument_recursive() {
    let temp_dir = TempDir::new().unwrap();

    // Test --recursive flag with different values
    for depth in &["0", "1", "5", "10"] {
        let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
        cmd.arg(temp_dir.path())
            .arg("--status")
            .arg("--no-git")
            .arg("--recursive")
            .arg(depth)
            .timeout(Duration::from_secs(5));

        cmd.assert().success();
    }
}

#[test]
fn test_cli_argument_verbose() {
    let temp_dir = TempDir::new().unwrap();

    // Test --verbose flag
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("--verbose")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();

    // Test short form -v
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("-v")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_argument_short_forms() {
    let temp_dir = TempDir::new().unwrap();

    // Test short form flags
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("-f")
        .arg("json")
        .arg("-d")
        .arg("100")
        .arg("-v")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_missing_required_path_argument() {
    // Run without path argument (should fail)
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("--status").timeout(Duration::from_secs(5));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("PATH").or(predicate::str::contains("required")));
}

#[test]
fn test_cli_invalid_path_argument() {
    // Run with non-existent path
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("/nonexistent/path/that/does/not/exist")
        .arg("--status")
        .timeout(Duration::from_secs(5));

    cmd.assert().failure();
}

#[test]
fn test_cli_help_flag() {
    // Test --help
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("--help").timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("High-performance file watcher"))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Options:"));

    // Test -h
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("-h").timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version_flag() {
    // Test --version
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("--version").timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("wezterm-watch"));

    // Test -V
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg("-V").timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("wezterm-watch"));
}

// ==============================================================================
// Test 4: Git Integration via CLI
// ==============================================================================

#[test]
fn test_cli_git_status_with_modified_file() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    // Initialize git repo with a tracked file
    init_git_repo(watch_path).unwrap();

    let tracked_file = watch_path.join("tracked.txt");
    fs::write(&tracked_file, "initial").unwrap();

    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(watch_path)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(["commit", "-m", "Add file"])
        .current_dir(watch_path)
        .output()
        .unwrap();

    // Modify the file
    fs::write(&tracked_file, "modified").unwrap();

    // Check status
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("json")
        .timeout(Duration::from_secs(5));

    let assert = cmd.assert().success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should detect modified file
    let modified = json["modified_files"].as_u64().unwrap();
    assert!(modified > 0, "Should detect modified file");
}

#[test]
fn test_cli_git_status_with_untracked_file() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    init_git_repo(watch_path).unwrap();

    // Create untracked file
    let untracked = watch_path.join("untracked.txt");
    fs::write(&untracked, "untracked content").unwrap();

    // Check status
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("json")
        .timeout(Duration::from_secs(5));

    let assert = cmd.assert().success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should detect untracked file
    let untracked_count = json["untracked_files"].as_u64().unwrap();
    assert!(untracked_count > 0, "Should detect untracked file");
}

#[test]
fn test_cli_git_auto_detection() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path();

    init_git_repo(watch_path).unwrap();

    // Run without --git or --no-git (should auto-detect)
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(watch_path)
        .arg("--status")
        .arg("--format")
        .arg("pretty")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Branch:"));
}

// ==============================================================================
// Test 5: Edge Cases and Error Handling
// ==============================================================================

#[test]
fn test_cli_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Run watcher on empty directory
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_conflicting_git_flags() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path()).unwrap();

    // Both --git and --no-git (--no-git should take precedence based on main.rs logic)
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--git")
        .arg("--no-git")
        .timeout(Duration::from_secs(5));

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Not a git repository").or(predicate::str::contains("git disabled")));
}

#[test]
fn test_cli_multiple_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Multiple --ignore flags
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("-i")
        .arg("*.log")
        .arg("-i")
        .arg("*.tmp")
        .arg("--ignore")
        .arg("*.bak")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_zero_interval() {
    let temp_dir = TempDir::new().unwrap();

    // Test interval=0 (should work, might mean no debounce or minimal)
    let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
    cmd.arg(temp_dir.path())
        .arg("--status")
        .arg("--no-git")
        .arg("--interval")
        .arg("0")
        .timeout(Duration::from_secs(5));

    cmd.assert().success();
}

#[test]
fn test_cli_case_insensitive_format() {
    let temp_dir = TempDir::new().unwrap();

    // Test case-insensitive format parsing
    for format in &["JSON", "Pretty", "EVENTS", "Summary", "JsOn"] {
        let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
        cmd.arg(temp_dir.path())
            .arg("--status")
            .arg("--no-git")
            .arg("--format")
            .arg(format)
            .timeout(Duration::from_secs(5));

        cmd.assert().success();
    }
}

// ==============================================================================
// Test 6: CI Compatibility and Timeout Verification
// ==============================================================================

#[test]
fn test_cli_terminates_with_timeout() {
    let temp_dir = TempDir::new().unwrap();

    // Start watcher using std::process::Command for spawning
    let mut child = StdCommand::new(get_binary_path())
        .arg(temp_dir.path())
        .arg("--no-git")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn watcher");

    // Wait briefly
    thread::sleep(Duration::from_millis(500));

    // Kill should work without hanging
    let kill_result = child.kill();
    assert!(kill_result.is_ok(), "Should be able to kill watcher");

    // Wait should complete
    let output = child.wait_with_output();
    assert!(output.is_ok(), "Should wait for process to complete");
}

#[test]
fn test_cli_handles_rapid_invocations() {
    let temp_dir = TempDir::new().unwrap();

    // Rapidly invoke CLI multiple times (tests for race conditions)
    for _ in 0..5 {
        let mut cmd = Command::cargo_bin(get_binary_name()).unwrap();
        cmd.arg(temp_dir.path())
            .arg("--status")
            .arg("--no-git")
            .timeout(Duration::from_secs(5));

        cmd.assert().success();
    }
}

#[test]
fn test_cli_no_panic_on_interruption() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();

    // Start watcher using std::process::Command for spawning
    let mut child = StdCommand::new(get_binary_path())
        .arg(&watch_path)
        .arg("--no-git")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn watcher");

    thread::sleep(Duration::from_millis(200));

    // Kill immediately (tests signal handling)
    child.kill().ok();

    // Should not panic
    let output = child.wait_with_output().unwrap();

    // Exit code might be signal-based, but should not contain panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panic") && !stderr.contains("thread panicked"),
        "Should not panic on interruption. Stderr: {}",
        stderr
    );
}
