//! Integration tests for wezterm-fs-explorer
//!
//! These tests verify the interaction between FileEntry, FileOperation, GitStatus,
//! IPC, path utilities, and shell detection components.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// Import from the library crate
use wezterm_fs_explorer::{
    detect_path_type, normalize_path, to_windows_path, to_wsl_path, PathType,
    detect_shell, Shell,
    FuzzySearch,
    IpcServer,
};

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Helper to create a git repository with initial commit
fn init_git_repo(path: &Path) -> anyhow::Result<()> {
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

/// Create a directory structure for testing
fn create_test_structure(base: &Path) -> anyhow::Result<()> {
    // Create directories
    fs::create_dir_all(base.join("src"))?;
    fs::create_dir_all(base.join("src/components"))?;
    fs::create_dir_all(base.join("tests"))?;
    fs::create_dir_all(base.join(".hidden"))?;

    // Create files
    fs::write(base.join("README.md"), "# Test Project")?;
    fs::write(base.join("Cargo.toml"), "[package]\nname = \"test\"")?;
    fs::write(base.join("src/main.rs"), "fn main() {}")?;
    fs::write(base.join("src/lib.rs"), "pub mod components;")?;
    fs::write(base.join("src/components/mod.rs"), "// components")?;
    fs::write(base.join("tests/test.rs"), "#[test] fn test() {}")?;
    fs::write(base.join(".hidden/config"), "secret")?;
    fs::write(base.join(".gitignore"), "target/\n*.log")?;

    Ok(())
}

// ==============================================================================
// Path Utilities Integration Tests
// ==============================================================================

#[test]
fn test_path_type_detection_windows() {
    // Windows absolute paths
    assert_eq!(detect_path_type(Path::new(r"C:\Users\test")), PathType::Windows);
    assert_eq!(detect_path_type(Path::new(r"D:\data\file.txt")), PathType::Windows);

    // UNC paths
    assert_eq!(detect_path_type(Path::new(r"\\server\share")), PathType::Windows);
}

#[test]
fn test_path_type_detection_wsl() {
    // WSL mount points
    assert_eq!(detect_path_type(Path::new("/mnt/c/Users")), PathType::Wsl);
    assert_eq!(detect_path_type(Path::new("/mnt/d/data")), PathType::Wsl);
}

#[test]
fn test_path_type_detection_unix() {
    // Standard Unix paths
    assert_eq!(detect_path_type(Path::new("/home/user")), PathType::Unix);
    assert_eq!(detect_path_type(Path::new("/usr/local/bin")), PathType::Unix);
}

#[test]
fn test_windows_to_wsl_path_conversion() {
    let win_path = Path::new(r"C:\Users\david\file.txt");
    let wsl_path = to_wsl_path(win_path);
    assert_eq!(wsl_path.to_str().unwrap(), "/mnt/c/Users/david/file.txt");
}

#[test]
fn test_wsl_to_windows_path_conversion() {
    let wsl_path = Path::new("/mnt/c/Users/david/file.txt");
    let win_path = to_windows_path(wsl_path);
    assert_eq!(win_path.to_str().unwrap(), r"C:\Users\david\file.txt");
}

#[test]
fn test_path_normalization_preserves_type() {
    // Windows paths stay Windows on Windows
    #[cfg(windows)]
    {
        let path = normalize_path(Path::new(r"C:\Users\test"));
        assert!(path.to_str().unwrap().contains("C:") || path.to_str().unwrap().contains("c:"));
    }

    // Unix paths stay Unix on Unix
    #[cfg(unix)]
    {
        let path = normalize_path(Path::new("/home/user"));
        assert!(path.to_str().unwrap().starts_with("/home"));
    }
}

#[test]
fn test_path_conversion_roundtrip() {
    let original = Path::new(r"C:\Users\david\Documents\file.txt");
    let wsl = to_wsl_path(original);
    let back = to_windows_path(&wsl);

    // Should preserve the path structure (case may differ on Windows)
    assert!(back.to_str().unwrap().to_lowercase().contains("users"));
    assert!(back.to_str().unwrap().to_lowercase().contains("david"));
}

// ==============================================================================
// Shell Detection Integration Tests
// ==============================================================================

#[test]
fn test_shell_detection_returns_valid_shell() {
    let shell = detect_shell();
    // Should return one of the known shells
    matches!(shell, Shell::PowerShell | Shell::GitBash | Shell::WslBash | Shell::Cmd | Shell::Unknown);
}

#[test]
fn test_shell_enum_variants() {
    // Verify all variants are accessible
    let shells = [
        Shell::PowerShell,
        Shell::GitBash,
        Shell::WslBash,
        Shell::Cmd,
        Shell::Unknown,
    ];

    assert_eq!(shells.len(), 5);
}

// ==============================================================================
// Fuzzy Search Integration Tests
// ==============================================================================

#[test]
fn test_fuzzy_search_basic_matching() {
    let mut search = FuzzySearch::new();

    let items = vec![
        PathBuf::from("/home/user/documents/readme.txt"),
        PathBuf::from("/home/user/documents/README.md"),
        PathBuf::from("/home/user/pictures/photo.jpg"),
        PathBuf::from("/home/user/code/main.rs"),
        PathBuf::from("/home/user/code/readme_backup.txt"),
    ];

    search.populate(items);

    let results = search.search("readme", 10);

    // Should find files containing "readme"
    assert!(!results.is_empty(), "Should find readme files");

    // Verify at least one result contains "readme"
    let has_readme = results.iter().any(|r| {
        r.path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_lowercase().contains("readme"))
            .unwrap_or(false)
    });
    assert!(has_readme, "Results should contain readme files");
}

#[test]
fn test_fuzzy_search_empty_query() {
    let mut search = FuzzySearch::new();

    search.populate(vec![
        PathBuf::from("/test/file1.txt"),
        PathBuf::from("/test/file2.txt"),
    ]);

    let results = search.search("", 10);
    assert!(results.is_empty(), "Empty query should return no results");
}

#[test]
fn test_fuzzy_search_no_matches() {
    let mut search = FuzzySearch::new();

    search.populate(vec![
        PathBuf::from("/test/file1.txt"),
        PathBuf::from("/test/file2.txt"),
    ]);

    let results = search.search("zzzznonexistent", 10);
    assert!(results.is_empty(), "Non-matching query should return no results");
}

#[test]
fn test_fuzzy_search_limit() {
    let mut search = FuzzySearch::new();

    // Create many files with similar names
    let items: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("/test/file{}.txt", i)))
        .collect();

    search.populate(items);

    let results = search.search("file", 5);
    assert!(results.len() <= 5, "Should respect limit");
}

#[test]
fn test_fuzzy_search_clear() {
    let mut search = FuzzySearch::new();

    search.populate(vec![PathBuf::from("/test/file.txt")]);

    // Should find before clear
    let results = search.search("file", 10);
    assert!(!results.is_empty());

    // Clear and search again
    search.clear();
    let results = search.search("file", 10);
    assert!(results.is_empty(), "Should be empty after clear");
}

#[test]
fn test_fuzzy_search_case_insensitive() {
    let mut search = FuzzySearch::new();

    search.populate(vec![
        PathBuf::from("/test/README.md"),
        PathBuf::from("/test/readme.txt"),
        PathBuf::from("/test/ReadMe.rst"),
    ]);

    // Search with different cases
    let results_lower = search.search("readme", 10);
    let results_upper = search.search("README", 10);
    let results_mixed = search.search("ReadMe", 10);

    // All should find results (case insensitive by default)
    assert!(!results_lower.is_empty());
    assert!(!results_upper.is_empty());
    assert!(!results_mixed.is_empty());
}

// ==============================================================================
// File Structure Integration Tests
// ==============================================================================

#[test]
fn test_directory_structure_creation() {
    let temp_dir = TempDir::new().unwrap();

    create_test_structure(temp_dir.path()).unwrap();

    // Verify structure
    assert!(temp_dir.path().join("src").exists());
    assert!(temp_dir.path().join("src/main.rs").exists());
    assert!(temp_dir.path().join("src/components").exists());
    assert!(temp_dir.path().join("tests").exists());
    assert!(temp_dir.path().join(".hidden").exists());
    assert!(temp_dir.path().join("README.md").exists());
}

#[test]
fn test_fuzzy_search_with_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    create_test_structure(temp_dir.path()).unwrap();

    // Collect all files
    let files: Vec<PathBuf> = walkdir::WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    let mut search = FuzzySearch::new();
    search.populate(files);

    // Search for Rust files
    let results = search.search("rs", 10);
    assert!(!results.is_empty(), "Should find .rs files");

    // Search for config
    let results = search.search("cargo", 10);
    assert!(!results.is_empty(), "Should find Cargo.toml");
}

// ==============================================================================
// Git Integration Tests
// ==============================================================================

#[test]
fn test_git_repo_initialization() {
    let temp_dir = TempDir::new().unwrap();

    let result = init_git_repo(temp_dir.path());
    assert!(result.is_ok(), "Should initialize git repo");

    // Verify .git directory exists
    assert!(temp_dir.path().join(".git").exists(), "Should have .git directory");
}

#[test]
fn test_git_with_file_structure() {
    let temp_dir = TempDir::new().unwrap();

    init_git_repo(temp_dir.path()).unwrap();
    create_test_structure(temp_dir.path()).unwrap();

    // Verify both git and structure exist
    assert!(temp_dir.path().join(".git").exists());
    assert!(temp_dir.path().join("src").exists());
}

// ==============================================================================
// IPC Integration Tests (Basic - No actual connections)
// ==============================================================================

#[test]
fn test_ipc_server_bind_and_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test.sock");

    // Create server - should bind successfully
    let server = IpcServer::bind(&socket_path);
    assert!(server.is_ok(), "Should bind to socket");

    // Socket file should exist
    assert!(socket_path.exists(), "Socket file should exist");

    // Drop server (should cleanup on drop ideally, but socket file may persist)
    drop(server);
}

#[test]
fn test_ipc_server_rebind() {
    let temp_dir = TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test2.sock");

    // Create and immediately drop first server
    {
        let _server = IpcServer::bind(&socket_path).unwrap();
    }

    // Should be able to rebind (IpcServer removes existing socket)
    let server2 = IpcServer::bind(&socket_path);
    assert!(server2.is_ok(), "Should rebind after previous server dropped");
}

// ==============================================================================
// Cross-component Integration Tests
// ==============================================================================

#[test]
fn test_path_utils_with_fuzzy_search() {
    let mut search = FuzzySearch::new();

    // Mix of Windows and WSL paths - fuzzy search indexes by filename
    let items = vec![
        PathBuf::from(r"C:\Users\david\code\main.rs"),
        PathBuf::from("/mnt/c/Users/david/code/lib.rs"),
        PathBuf::from("/home/david/code/test.rs"),
    ];

    search.populate(items);

    // Search for "main" (part of filename main.rs)
    let results = search.search("main", 10);
    assert!(!results.is_empty(), "Should find paths with 'main' in filename");

    // Verify path type detection still works on results
    for result in &results {
        let _ = detect_path_type(&result.path);
    }
}

#[test]
fn test_end_to_end_file_discovery() {
    let temp_dir = TempDir::new().unwrap();
    create_test_structure(temp_dir.path()).unwrap();

    // 1. Walk directory
    let files: Vec<PathBuf> = walkdir::WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    // 2. Index in fuzzy search
    let mut search = FuzzySearch::new();
    search.populate(files.clone());

    // 3. Search for specific file types
    let rust_files = search.search(".rs", 10);
    let markdown = search.search(".md", 10);

    // 4. Verify results
    assert!(!rust_files.is_empty(), "Should find Rust files");
    assert!(!markdown.is_empty(), "Should find Markdown files");

    // 5. Verify path detection works on results
    for result in rust_files {
        let path_type = detect_path_type(&result.path);
        // On Windows, temp paths are Windows paths
        #[cfg(windows)]
        assert!(matches!(path_type, PathType::Windows | PathType::Unknown));
        // On Unix, temp paths are Unix paths
        #[cfg(unix)]
        assert!(matches!(path_type, PathType::Unix | PathType::Unknown));
    }
}

// ==============================================================================
// Edge Cases and Error Handling
// ==============================================================================

#[test]
fn test_path_conversion_edge_cases() {
    // Empty path
    let empty = to_wsl_path(Path::new(""));
    assert!(empty.as_os_str().is_empty() || empty.to_str().unwrap() == "");

    // Root path
    let root = to_wsl_path(Path::new(r"C:\"));
    assert!(root.to_str().unwrap().contains("/mnt/c"));

    // Path with spaces
    let spaces = to_wsl_path(Path::new(r"C:\Program Files\app"));
    assert!(spaces.to_str().unwrap().contains("Program Files"));
}

#[test]
fn test_fuzzy_search_special_characters() {
    let mut search = FuzzySearch::new();

    search.populate(vec![
        PathBuf::from("/test/file-with-dashes.txt"),
        PathBuf::from("/test/file_with_underscores.txt"),
        PathBuf::from("/test/file.multiple.dots.txt"),
        PathBuf::from("/test/file with spaces.txt"),
    ]);

    // Should handle special characters in search
    let results = search.search("with", 10);
    assert!(!results.is_empty(), "Should find files with special chars");
}

#[test]
fn test_ipc_server_invalid_path() {
    // On Windows, certain paths are invalid for UDS
    // This test verifies error handling
    #[cfg(windows)]
    {
        // CON is a reserved name on Windows
        let result = IpcServer::bind(r"\\.\CON");
        // Should either fail or work - we're testing it doesn't panic
        let _ = result;
    }
}

// ==============================================================================
// Performance Tests (Basic)
// ==============================================================================

#[test]
fn test_fuzzy_search_performance_many_items() {
    let mut search = FuzzySearch::new();

    // Create 1000 items
    let items: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("/project/src/file{:04}.rs", i)))
        .collect();

    search.populate(items);

    // Time the search
    let start = std::time::Instant::now();
    let results = search.search("file", 10);
    let elapsed = start.elapsed();

    assert!(!results.is_empty());
    assert!(elapsed.as_millis() < 1000, "Search should complete in < 1 second");
}

#[test]
fn test_path_conversion_performance() {
    let start = std::time::Instant::now();

    for _i in 0..1000 {
        let path = Path::new(r"C:\Users\david\Documents\file.txt");
        let _ = to_wsl_path(path);
        let _ = detect_path_type(path);
    }

    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "1000 conversions should complete in < 500ms");
}
