//! Mock client for testing wezterm-utils-daemon
//!
//! Usage: cargo run --example mock_client

use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mock Client for wezterm-utils-daemon\n");

    let pipe_name = r"\\.\pipe\wezterm-utils-ipc";
    println!("Connecting to: {}", pipe_name);

    // Connect to daemon
    let client = match ClientOptions::new().open(pipe_name) {
        Ok(c) => {
            println!("✅ Connected to daemon\n");
            c
        }
        Err(e) => {
            eprintln!("❌ Failed to connect: {}", e);
            eprintln!("\nMake sure the daemon is running:");
            eprintln!("  cargo run -- start");
            return Err(e.into());
        }
    };

    let (reader, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(reader);

    // Test 1: Ping
    println!("📤 Test 1: Ping");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/ping",
            "id": 1
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 2: Register utility
    println!("📤 Test 2: Register Utility");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/register",
            "params": {
                "name": "mock-client",
                "capabilities": ["test", "demo"]
            },
            "id": 2
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 3: Subscribe to events
    println!("📤 Test 3: Subscribe to Events");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/subscribe",
            "params": {
                "subscriptions": [
                    {
                        "event_type": "test.event",
                        "filter": null
                    }
                ]
            },
            "id": 3
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 4: Get status
    println!("📤 Test 4: Get Daemon Status");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/status",
            "id": 4
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 5: Broadcast event
    println!("📤 Test 5: Broadcast Event");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/broadcast",
            "params": {
                "event_type": "test.event",
                "data": {
                    "message": "Hello from mock client!",
                    "timestamp": chrono::Utc::now().timestamp()
                }
            },
            "id": 5
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 6: Notification (no response expected)
    println!("📤 Test 6: Send Notification");
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "test.notification",
        "params": {
            "message": "This is a notification"
        }
    });

    let json = format!("{}\n", serde_json::to_string(&notification)?);
    writer.write_all(json.as_bytes()).await?;
    writer.flush().await?;
    println!("   Notification sent (no response expected)\n");

    sleep(Duration::from_millis(500)).await;

    // Test 7: Unsubscribe
    println!("📤 Test 7: Unsubscribe");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/unsubscribe",
            "params": {
                "event_types": ["test.event"]
            },
            "id": 7
        }),
    )
    .await?;

    sleep(Duration::from_millis(500)).await;

    // Test 8: Final ping
    println!("📤 Test 8: Final Ping");
    send_request(
        &mut writer,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "method": "daemon/ping",
            "id": 8
        }),
    )
    .await?;

    println!("\n✅ All tests completed successfully!");

    Ok(())
}

async fn send_request(
    writer: &mut tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>,
    reader: &mut BufReader<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>>,
    request: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send request
    let json = format!("{}\n", serde_json::to_string(&request)?);
    writer.write_all(json.as_bytes()).await?;
    writer.flush().await?;

    println!("   Request: {}", request["method"]);

    // Read response
    let mut line = String::new();
    let bytes_read = tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await??;

    if bytes_read == 0 {
        eprintln!("   ❌ Connection closed");
        return Err("Connection closed".into());
    }

    let response: serde_json::Value = serde_json::from_str(&line)?;

    if let Some(error) = response.get("error") {
        println!("   ❌ Error: {}", serde_json::to_string_pretty(&error)?);
    } else if let Some(result) = response.get("result") {
        println!("   ✅ Response: {}", serde_json::to_string(&result)?);
    }

    println!();

    Ok(())
}