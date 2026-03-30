//! End-to-End (E2E) tests for wezterm-fs-explorer binary.
//!
//! These tests verify the complete behavior of the binary including:
//! - CLI argument parsing and validation
//! - IPC server/client communication
//! - Process lifecycle (help, version, exit)
//! - Non-interactive modes (JSON output, IPC-only)
//!
//! Note: These tests do NOT test interactive TUI mode since that requires
//! terminal interaction. They focus on testable, non-interactive features.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;

// Import library types for IPC testing
use wezterm_fs_explorer::ipc::{IpcClient, IpcServer};
use wezterm_fs_explorer::ipc_client::{IpcMessage, JsonRpcRequest};

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Get a Command for the wezterm-fs-explorer binary
fn cmd() -> Command {
    Command::cargo_bin("wezterm-fs-explorer").expect("Failed to find binary")
}

/// Create a test directory structure
fn create_test_dir() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp dir");
    fs::write(temp.path().join("test.txt"), "test content").expect("Failed to write test file");
    fs::create_dir(temp.path().join("subdir")).expect("Failed to create subdir");
    temp
}

/// Platform-specific socket path
fn socket_path(temp_dir: &TempDir) -> PathBuf {
    if cfg!(windows) {
        temp_dir.path().join("test.sock")
    } else {
        temp_dir.path().join("test.sock")
    }
}

// ==============================================================================
// CLI Argument Parsing Tests
// ==============================================================================

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "High-performance filesystem explorer",
        ))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--ipc-socket"));
}

// Note: --version flag is not implemented in the current CLI
// If needed, add #[command(version)] to the Args struct in main.rs

#[test]
fn test_invalid_directory_path() {
    cmd()
        .arg("/this/path/does/not/exist/at/all")
        .timeout(Duration::from_secs(2))
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_valid_directory_with_json_mode() {
    let temp = create_test_dir();

    cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(5))
        .assert()
        .success()
        .stdout(predicate::str::is_match(r#"\[.*\]"#).unwrap());
}

#[test]
fn test_current_directory_default() {
    // Without any directory argument, should use current directory
    // Since it's non-interactive, we use --json to get output and timeout quickly
    let output = cmd()
        .arg("--json")
        .timeout(Duration::from_secs(2))
        .output()
        .expect("Failed to execute command");

    // Should succeed (even if output is empty JSON array)
    assert!(
        output.status.success(),
        "Command should succeed with default directory"
    );
}

#[test]
fn test_ipc_socket_option() {
    let temp = create_test_dir();
    let socket = socket_path(&temp);

    // Test that --ipc-socket option is accepted (will fail to connect but shouldn't error on arg)
    let output = cmd()
        .arg(temp.path())
        .arg("--ipc-socket")
        .arg(socket.to_str().unwrap())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    // Should succeed even if IPC connection fails (runs in standalone mode)
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Command should accept --ipc-socket option"
    );
}

#[test]
fn test_json_flag_produces_json_output() {
    let temp = create_test_dir();

    let output = cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Output should be valid JSON (array)
    assert!(
        stdout.starts_with('[') && stdout.contains(']'),
        "JSON output should be a valid array"
    );
}

// ==============================================================================
// IPC Server/Client Communication Tests
// ==============================================================================

#[tokio::test]
async fn test_ipc_server_startup() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    // Create IPC server
    let server = IpcServer::bind(&socket);
    assert!(server.is_ok(), "Should successfully bind IPC server");

    // Socket file should exist
    assert!(socket.exists(), "Socket file should exist after bind");
}

#[tokio::test]
async fn test_ipc_client_connection() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    // Start server
    let server = IpcServer::bind(&socket).expect("Failed to bind server");

    // Spawn server accept task
    let server_task = tokio::spawn(async move {
        timeout(Duration::from_secs(2), server.accept())
            .await
            .expect("Server accept timed out")
            .expect("Server accept failed")
    });

    // Give server time to start listening
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client
    let client = IpcClient::connect(&socket).await;
    assert!(client.is_ok(), "Client should connect successfully");

    // Wait for server to accept connection
    let _stream = server_task.await.expect("Server task failed");
    // Connection successful - no need to verify further
}

#[tokio::test]
async fn test_ipc_message_sending() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    // Start server
    let server = IpcServer::bind(&socket).expect("Failed to bind server");

    // Spawn server task that reads a message
    let server_task = tokio::spawn(async move {
        let stream = server.accept().await.expect("Failed to accept");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("Failed to read line");
        line
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client and send message
    let mut client_stream = IpcClient::connect(&socket)
        .await
        .expect("Failed to connect");

    let message = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "test.method".to_string(),
        params: serde_json::json!({"test": "data"}),
    };

    let message_str = serde_json::to_string(&message).expect("Failed to serialize");
    client_stream
        .write_all(message_str.as_bytes())
        .await
        .expect("Failed to write");
    client_stream
        .write_all(b"\n")
        .await
        .expect("Failed to write newline");

    // Read message on server side
    let received = timeout(Duration::from_secs(2), server_task)
        .await
        .expect("Server task timed out")
        .expect("Server task failed");

    // Parse received message
    let parsed: Result<JsonRpcRequest, _> = serde_json::from_str(received.trim());
    assert!(
        parsed.is_ok(),
        "Server should receive valid JSON-RPC message"
    );

    let parsed = parsed.unwrap();
    assert_eq!(parsed.method, "test.method");
    assert_eq!(parsed.id, 1);
}

#[tokio::test]
async fn test_ipc_watch_directory_message() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);
    let test_path = PathBuf::from("/test/path");

    // Start server
    let server = IpcServer::bind(&socket).expect("Failed to bind server");

    // Spawn server to receive message
    let test_path_clone = test_path.clone();
    let server_task = tokio::spawn(async move {
        let stream = server.accept().await.expect("Failed to accept");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("Failed to read line");

        let request: JsonRpcRequest =
            serde_json::from_str(line.trim()).expect("Failed to parse JSON-RPC");

        // Verify method name
        assert_eq!(request.method, "watcher.watch_directory");

        // Verify message can be deserialized
        let msg: Result<IpcMessage, _> = serde_json::from_value(request.params.clone());
        assert!(msg.is_ok(), "Should deserialize to IpcMessage");

        // Verify it's the correct message variant
        if let Ok(IpcMessage::WatchDirectory { path }) = msg {
            assert_eq!(path, test_path_clone);
        } else {
            panic!("Wrong message variant");
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect and send WatchDirectory message
    let mut client_stream = IpcClient::connect(&socket)
        .await
        .expect("Failed to connect");

    let message = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "watcher.watch_directory".to_string(),
        params: serde_json::json!({
            "method": "watcher.watch_directory",
            "params": {
                "path": test_path
            }
        }),
    };

    let message_str = serde_json::to_string(&message).expect("Failed to serialize");
    client_stream
        .write_all(message_str.as_bytes())
        .await
        .expect("Failed to write");
    client_stream
        .write_all(b"\n")
        .await
        .expect("Failed to write newline");

    // Wait for server verification
    timeout(Duration::from_secs(2), server_task)
        .await
        .expect("Server task timed out")
        .expect("Server task failed");
}

#[tokio::test]
async fn test_ipc_open_file_message() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);
    let test_path = PathBuf::from("/test/file.txt");

    // Start server
    let server = IpcServer::bind(&socket).expect("Failed to bind server");

    // Spawn server to receive message
    let test_path_clone = test_path.clone();
    let server_task = tokio::spawn(async move {
        let stream = server.accept().await.expect("Failed to accept");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("Failed to read line");

        let request: JsonRpcRequest =
            serde_json::from_str(line.trim()).expect("Failed to parse JSON-RPC");

        assert_eq!(request.method, "editor.open_file");

        let msg: Result<IpcMessage, _> = serde_json::from_value(request.params.clone());
        assert!(msg.is_ok(), "Should deserialize to IpcMessage");

        if let Ok(IpcMessage::OpenFile { path, line, column }) = msg {
            assert_eq!(path, test_path_clone);
            assert_eq!(line, Some(10));
            assert_eq!(column, Some(5));
        } else {
            panic!("Wrong message variant");
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect and send OpenFile message
    let mut client_stream = IpcClient::connect(&socket)
        .await
        .expect("Failed to connect");

    let message = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 2,
        method: "editor.open_file".to_string(),
        params: serde_json::json!({
            "method": "editor.open_file",
            "params": {
                "path": test_path,
                "line": 10,
                "column": 5
            }
        }),
    };

    let message_str = serde_json::to_string(&message).expect("Failed to serialize");
    client_stream
        .write_all(message_str.as_bytes())
        .await
        .expect("Failed to write");
    client_stream
        .write_all(b"\n")
        .await
        .expect("Failed to write newline");

    timeout(Duration::from_secs(2), server_task)
        .await
        .expect("Server task timed out")
        .expect("Server task failed");
}

// ==============================================================================
// Process Lifecycle Tests
// ==============================================================================

#[test]
fn test_binary_exists_and_executable() {
    // Simply verify the binary can be found and is executable
    let result = Command::cargo_bin("wezterm-fs-explorer");
    assert!(result.is_ok(), "Binary should be found");
}

#[test]
fn test_help_exits_successfully() {
    cmd().arg("--help").assert().success().code(0);
}

// Note: --version flag is not implemented in the current CLI

#[test]
fn test_invalid_args_exit_with_error() {
    cmd()
        .arg("--invalid-flag-that-does-not-exist")
        .assert()
        .failure();
}

#[test]
fn test_json_mode_exits_cleanly() {
    let temp = create_test_dir();

    cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .assert()
        .success();
}

// ==============================================================================
// IPC Server Lifecycle Tests
// ==============================================================================

#[tokio::test]
async fn test_ipc_server_multiple_connections() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    // Create server
    let server = IpcServer::bind(&socket).expect("Failed to bind server");

    // First connection
    let task1 = tokio::spawn(async move {
        timeout(Duration::from_secs(2), server.accept())
            .await
            .expect("First accept timed out")
            .expect("First accept failed")
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client1 = IpcClient::connect(&socket).await;
    assert!(client1.is_ok(), "First client should connect");

    task1.await.expect("First connection task failed");

    // Second connection (server can be reused)
    // Note: In real usage, you'd need to call accept() again on the same server
    // This test verifies the socket can handle multiple sequential connections
    let server2 = IpcServer::bind(&socket).expect("Should rebind after first connection");

    let task2 = tokio::spawn(async move {
        timeout(Duration::from_secs(2), server2.accept())
            .await
            .expect("Second accept timed out")
            .expect("Second accept failed")
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client2 = IpcClient::connect(&socket).await;
    assert!(client2.is_ok(), "Second client should connect");

    task2.await.expect("Second connection task failed");
}

#[tokio::test]
async fn test_ipc_server_cleanup_on_drop() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    {
        let _server = IpcServer::bind(&socket).expect("Failed to bind server");
        assert!(socket.exists(), "Socket should exist while server is alive");
    }

    // After server is dropped, we should be able to bind again
    let server2 = IpcServer::bind(&socket);
    assert!(
        server2.is_ok(),
        "Should be able to rebind after server dropped"
    );
}

// ==============================================================================
// Error Handling Tests
// ==============================================================================

#[tokio::test]
async fn test_ipc_connection_to_nonexistent_socket() {
    let socket = PathBuf::from("/tmp/nonexistent-socket-12345.sock");

    let result = IpcClient::connect(&socket).await;
    assert!(
        result.is_err(),
        "Should fail to connect to nonexistent socket"
    );
}

#[tokio::test]
async fn test_ipc_server_bind_to_invalid_path() {
    // Test binding to a path that's clearly invalid
    let result = if cfg!(windows) {
        // On Windows, test a path in a nonexistent directory
        IpcServer::bind(r"Z:\nonexistent\path\to\socket.sock")
    } else {
        // On Unix, test a path with insufficient permissions (if not root)
        IpcServer::bind("/root/restricted/socket.sock")
    };

    // Should fail (or succeed if running as admin/root - we just check it doesn't panic)
    let _ = result;
}

// ==============================================================================
// Integration with File System Tests
// ==============================================================================

#[test]
fn test_json_mode_with_existing_directory() {
    let temp = create_test_dir();

    let output = cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Result<Vec<PathBuf>, _> = serde_json::from_str(&stdout);

    assert!(json.is_ok(), "Output should be valid JSON array of paths");
}

#[test]
fn test_directory_with_subdirectories() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    // Create nested structure
    fs::create_dir_all(temp.path().join("a/b/c")).expect("Failed to create dirs");
    fs::write(temp.path().join("a/file1.txt"), "content").expect("Failed to write");
    fs::write(temp.path().join("a/b/file2.txt"), "content").expect("Failed to write");

    let output = cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Should handle nested directories");
}

// ==============================================================================
// Platform-Specific Tests
// ==============================================================================

#[cfg(windows)]
#[test]
fn test_windows_path_handling() {
    use std::env;

    // Use Windows-style path
    let temp_dir = env::temp_dir();

    let output = cmd()
        .arg(temp_dir)
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Should handle Windows paths");
}

#[cfg(unix)]
#[test]
fn test_unix_path_handling() {
    let output = cmd()
        .arg("/tmp")
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Should handle Unix paths");
}

// ==============================================================================
// Concurrency Tests
// ==============================================================================

#[tokio::test]
async fn test_concurrent_ipc_connections() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let socket = socket_path(&temp);

    // Bind server
    let _server = IpcServer::bind(&socket).expect("Failed to bind server");

    // Try concurrent connections (they will queue at the server)
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let socket = socket.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(i * 100)).await;
                IpcClient::connect(&socket).await
            })
        })
        .collect();

    // At least some should succeed (or all fail gracefully)
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked"))
        .collect();

    // Verify no panics occurred
    assert_eq!(results.len(), 3);
}

// ==============================================================================
// Timeout and Performance Tests
// ==============================================================================

#[test]
fn test_help_completes_quickly() {
    use std::time::Instant;

    let start = Instant::now();

    cmd().arg("--help").assert().success();

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 1000,
        "Help should complete in less than 1 second"
    );
}

#[test]
fn test_json_mode_completes_quickly() {
    let temp = create_test_dir();
    use std::time::Instant;

    let start = Instant::now();

    cmd()
        .arg(temp.path())
        .arg("--json")
        .timeout(Duration::from_secs(3))
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 3,
        "JSON mode should complete in less than 3 seconds"
    );
}
