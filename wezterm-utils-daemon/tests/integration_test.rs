//! Integration tests for wezterm-utils-daemon

use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::time::timeout;

#[cfg(windows)]
mod windows_tests {
    use super::*;

    /// Test connecting to daemon and sending ping
    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_daemon_ping() {
        let pipe_name = r"\\.\pipe\wezterm-utils-test";

        // Connect to daemon
        let client = ClientOptions::new()
            .open(pipe_name)
            .expect("Failed to connect to daemon");

        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);

        // Send ping request
        let request = json!({
            "jsonrpc": "2.0",
            "method": "daemon/ping",
            "id": 1
        });

        let json = format!("{}\n", serde_json::to_string(&request).unwrap());
        writer
            .write_all(json.as_bytes())
            .await
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        // Read response
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("Timeout waiting for response")
            .expect("Failed to read response");

        let response: serde_json::Value =
            serde_json::from_str(&line).expect("Failed to parse response");

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["status"], "pong");
    }

    /// Test daemon status request
    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_daemon_status() {
        let pipe_name = r"\\.\pipe\wezterm-utils-test";

        let client = ClientOptions::new()
            .open(pipe_name)
            .expect("Failed to connect to daemon");

        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);

        // Send status request
        let request = json!({
            "jsonrpc": "2.0",
            "method": "daemon/status",
            "id": 2
        });

        let json = format!("{}\n", serde_json::to_string(&request).unwrap());
        writer.write_all(json.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        // Read response
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();

        let response: serde_json::Value = serde_json::from_str(&line).unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 2);

        let result = &response["result"];
        assert!(result["version"].is_string());
        assert!(result["uptime_seconds"].is_number());
        assert!(result["active_connections"].is_number());
    }

    /// Test multiple concurrent connections
    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_concurrent_connections() {
        let pipe_name = r"\\.\pipe\wezterm-utils-test";
        let num_clients = 5;

        let mut handles = Vec::new();

        for i in 0..num_clients {
            let pipe_name = pipe_name.to_string();
            let handle = tokio::spawn(async move {
                let client = ClientOptions::new()
                    .open(&pipe_name)
                    .expect("Failed to connect");

                let (reader, mut writer) = tokio::io::split(client);
                let mut reader = BufReader::new(reader);

                // Send ping
                let request = json!({
                    "jsonrpc": "2.0",
                    "method": "daemon/ping",
                    "id": i
                });

                let json = format!("{}\n", serde_json::to_string(&request).unwrap());
                writer.write_all(json.as_bytes()).await.unwrap();
                writer.flush().await.unwrap();

                // Read response
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();

                let response: serde_json::Value = serde_json::from_str(&line).unwrap();
                assert_eq!(response["id"], i);
            });

            handles.push(handle);
        }

        // Wait for all clients to complete
        for handle in handles {
            handle.await.expect("Client task failed");
        }
    }
}

#[cfg(not(windows))]
mod non_windows {
    #[test]
    fn test_not_supported() {
        // Named pipes are Windows-only
        println!("Integration tests require Windows");
    }
}
