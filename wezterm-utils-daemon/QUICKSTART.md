# Quick Start Guide

Get up and running with wezterm-utils-daemon in 5 minutes.

## Step 1: Build the Daemon

```powershell
cd T:\projects\wezterm-utils-daemon
.\build.ps1 -Profile release
```

Expected output:
```
🦀 Building wezterm-utils-daemon
Profile: release
🔨 Building...
✅ Build complete!
   Binary: target\release\wezterm-utils-daemon.exe
   Size: 2.5 MB
```

## Step 2: Start the Daemon

In a new PowerShell window:

```powershell
.\target\release\wezterm-utils-daemon.exe start
```

Expected output:
```
2025-09-30T12:00:00.000Z  INFO wezterm_utils_daemon: wezterm-utils-daemon v0.1.0
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon: Using default configuration
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon: Configuration:
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon:   Pipe name: \\.\pipe\wezterm-utils-ipc
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon:   Max connections: 10
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon:   Keep-alive: 30s
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon:   Timeout: 120s
2025-09-30T12:00:00.001Z  INFO wezterm_utils_daemon: Starting daemon...
2025-09-30T12:00:00.002Z  INFO wezterm_utils_daemon::server: Starting named pipe server
2025-09-30T12:00:00.002Z  INFO wezterm_utils_daemon::router: Message router started
```

## Step 3: Test with Mock Client

In another PowerShell window:

```powershell
cargo run --example mock_client
```

Expected output:
```
🚀 Mock Client for wezterm-utils-daemon

Connecting to: \\.\pipe\wezterm-utils-ipc
✅ Connected to daemon

📤 Test 1: Ping
   Request: daemon/ping
   ✅ Response: {"status":"pong"}

📤 Test 2: Register Utility
   Request: daemon/register
   ✅ Response: {"connection_id":"550e8400-e29b-41d4-a716-446655440000","name":"mock-client","status":"registered"}

📤 Test 3: Subscribe to Events
   Request: daemon/subscribe
   ✅ Response: {"count":1,"status":"subscribed"}

📤 Test 4: Get Daemon Status
   Request: daemon/status
   ✅ Response: {"active_connections":1,"max_connections":10,"total_messages":4,"uptime_seconds":10,"version":"0.1.0"}

📤 Test 5: Broadcast Event
   Request: daemon/broadcast
   ✅ Response: {"recipients":1,"status":"broadcast"}

📤 Test 6: Send Notification
   Notification sent (no response expected)

📤 Test 7: Unsubscribe
   Request: daemon/unsubscribe
   ✅ Response: {"count":1,"status":"unsubscribed"}

📤 Test 8: Final Ping
   Request: daemon/ping
   ✅ Response: {"status":"pong"}

✅ All tests completed successfully!
```

## Step 4: Check Daemon Status

```powershell
.\target\release\wezterm-utils-daemon.exe status
```

Expected output:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "version": "0.1.0",
    "uptime_seconds": 120,
    "active_connections": 0,
    "total_messages": 8,
    "max_connections": 10
  },
  "id": 1
}
```

## Step 5: Connect Your Own Client

### PowerShell Example

```powershell
# Create client connection
$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(
    ".", "wezterm-utils-ipc", [System.IO.Pipes.PipeDirection]::InOut
)
$pipe.Connect(5000)

$writer = New-Object System.IO.StreamWriter($pipe)
$reader = New-Object System.IO.StreamReader($pipe)

# Send request
$request = @{
    jsonrpc = "2.0"
    method = "daemon/ping"
    id = 1
} | ConvertTo-Json -Compress

$writer.WriteLine($request)
$writer.Flush()

# Read response
$response = $reader.ReadLine() | ConvertFrom-Json
Write-Host ($response | ConvertTo-Json -Depth 10)

# Clean up
$pipe.Close()
```

### Rust Example

```rust
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::json;

#[tokio::main]
async fn main() {
    let client = ClientOptions::new()
        .open(r"\\.\pipe\wezterm-utils-ipc")
        .unwrap();

    let (reader, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(reader);

    let request = json!({
        "jsonrpc": "2.0",
        "method": "daemon/ping",
        "id": 1
    });

    let json = format!("{}\n", serde_json::to_string(&request).unwrap());
    writer.write_all(json.as_bytes()).await.unwrap();
    writer.flush().await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    println!("Response: {}", line);
}
```

## Troubleshooting

### "Failed to connect to pipe"

Make sure the daemon is running:

```powershell
Get-Process wezterm-utils-daemon
```

If not running, start it:

```powershell
.\target\release\wezterm-utils-daemon.exe start
```

### "Pipe already in use"

Another instance is running. Stop it with Ctrl+C or use a different pipe name:

```powershell
.\target\release\wezterm-utils-daemon.exe start --pipe "\\.\pipe\my-custom-pipe"
```

### Connection timeout

Increase timeout in configuration:

```toml
timeout_seconds = 300  # 5 minutes
```

## Next Steps

1. **Generate Config**: `wezterm-utils-daemon generate-config`
2. **Validate Config**: `wezterm-utils-daemon validate-config`
3. **Run Tests**: `cargo test --all-features`
4. **Run Benchmarks**: `cargo bench`
5. **Install**: `.\build.ps1 -Profile release -Install`

## Performance Testing

Run the latency benchmark:

```powershell
# Start daemon in terminal 1
.\target\release\wezterm-utils-daemon.exe start --pipe "\\.\pipe\wezterm-utils-test"

# Run benchmark in terminal 2
cargo bench
```

Expected results:
- JSON serialization: 1-5 μs
- Protocol parsing: 1-3 μs
- Round-trip latency: <50ms p99

## Advanced Usage

### Run as Background Service

```powershell
Start-Process -FilePath ".\target\release\wezterm-utils-daemon.exe" `
              -ArgumentList "start" `
              -WindowStyle Hidden
```

### Enable Debug Logging

```powershell
.\target\release\wezterm-utils-daemon.exe start --log-level debug
```

### JSON Logging

```powershell
.\target\release\wezterm-utils-daemon.exe start --json-logs
```

Output format:
```json
{"timestamp":"2025-09-30T12:00:00.000Z","level":"INFO","message":"Starting daemon..."}
```

## Documentation

- Full documentation: [README.md](README.md)
- Protocol specification: JSON-RPC 2.0 section in README
- API reference: `cargo doc --open`