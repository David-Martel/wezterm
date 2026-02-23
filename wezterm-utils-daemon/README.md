# wezterm-utils-daemon

High-performance IPC Router Daemon for WezTerm utilities using Windows Named Pipes and JSON-RPC 2.0.

## Features

- **Named Pipe Server**: Windows native IPC using `\\.\pipe\wezterm-utils-ipc`
- **JSON-RPC 2.0**: Standard protocol for message routing
- **Event Broadcasting**: Publish/subscribe pattern for utility communication
- **Connection Pooling**: Manage up to configurable concurrent connections (default: 10)
- **Low Latency**: Target <50ms p99 latency for message routing
- **Resource Limits**: Configurable connection limits and timeouts
- **Graceful Shutdown**: Clean connection handling on Ctrl+C
- **Process Validation**: Windows PID validation for security (optional)

## Architecture

```
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│  Utility A  │       │  Utility B  │       │  Utility C  │
└──────┬──────┘       └──────┬──────┘       └──────┬──────┘
       │                     │                     │
       │   JSON-RPC 2.0      │                     │
       └─────────┬───────────┴─────────────────────┘
                 │
                 │  Named Pipe: \\.\pipe\wezterm-utils-ipc
                 │
         ┌───────▼────────┐
         │     Daemon     │
         │                │
         │  - Router      │
         │  - Conn Pool   │
         │  - Broadcast   │
         └────────────────┘
```

## Installation

### Build from Source

```powershell
# Clone the repository
cd T:\projects\wezterm-utils-daemon

# Build release binary
.\build.ps1 -Profile release

# Install to PATH
.\build.ps1 -Profile release -Install
```

### Build Profiles

- `debug`: Fast compilation, debug symbols, no optimizations
- `release`: Full optimizations, LTO, stripped binary
- `release-fast`: Thin LTO, faster compilation than release

## Usage

### Start the Daemon

```powershell
# Start with default configuration
wezterm-utils-daemon start

# Start with custom pipe name
wezterm-utils-daemon start --pipe "\\.\pipe\my-custom-pipe"

# Start with custom connection limit
wezterm-utils-daemon start --max-connections 20

# Start with config file
wezterm-utils-daemon --config config.toml start
```

### Configuration

Generate a default configuration file:

```powershell
wezterm-utils-daemon generate-config --output config.toml
```

Example configuration (`config.toml`):

```toml
# Named pipe path
pipe_name = "\\\\.\\pipe\\wezterm-utils-ipc"

# Maximum concurrent connections
max_connections = 10

# Log level (trace, debug, info, warn, error)
log_level = "info"

# Enable JSON logging
json_logging = false

# Keep-alive interval in seconds
keep_alive_seconds = 30

# Connection timeout in seconds
timeout_seconds = 120

# Process ID validation (Windows only)
validate_process_ids = true
```

### Validate Configuration

```powershell
wezterm-utils-daemon validate-config --file config.toml
```

### Check Status

```powershell
# Query running daemon status
wezterm-utils-daemon status
```

## JSON-RPC 2.0 Protocol

### Daemon Methods

#### Register Utility

Register a utility with the daemon:

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/register",
  "params": {
    "name": "wezterm-terminal",
    "capabilities": ["terminal", "multiplexer"]
  },
  "id": 1
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "registered",
    "connection_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "wezterm-terminal"
  },
  "id": 1
}
```

#### Subscribe to Events

Subscribe to specific event types:

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/subscribe",
  "params": {
    "subscriptions": [
      {
        "event_type": "terminal.output",
        "filter": null
      },
      {
        "event_type": "pane.created",
        "filter": null
      }
    ]
  },
  "id": 2
}
```

#### Broadcast Event

Broadcast an event to all subscribers:

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/broadcast",
  "params": {
    "event_type": "terminal.output",
    "data": {
      "terminal_id": "term-001",
      "content": "Hello, World!",
      "timestamp": 1234567890
    }
  },
  "id": 3
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "broadcast",
    "recipients": 3
  },
  "id": 3
}
```

#### Send to Specific Utility

Send a message to a specific utility by name:

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/send",
  "params": {
    "target": "wezterm-terminal",
    "message": {
      "action": "create_pane",
      "direction": "right"
    }
  },
  "id": 4
}
```

#### Ping (Keep-Alive)

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/ping",
  "id": 5
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "pong"
  },
  "id": 5
}
```

#### Get Status

```json
{
  "jsonrpc": "2.0",
  "method": "daemon/status",
  "id": 6
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "active_connections": 5,
    "total_messages": 12345,
    "max_connections": 10
  },
  "id": 6
}
```

## Client Implementation Example

### PowerShell Client

```powershell
# Connect to daemon
$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(
    ".", "wezterm-utils-ipc", [System.IO.Pipes.PipeDirection]::InOut
)
$pipe.Connect(5000)

$writer = New-Object System.IO.StreamWriter($pipe)
$reader = New-Object System.IO.StreamReader($pipe)

# Send ping request
$request = @{
    jsonrpc = "2.0"
    method = "daemon/ping"
    id = 1
} | ConvertTo-Json -Compress

$writer.WriteLine($request)
$writer.Flush()

# Read response
$response = $reader.ReadLine() | ConvertFrom-Json
Write-Host "Response: $($response | ConvertTo-Json)"

$pipe.Close()
```

### Rust Client

```rust
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::json;

#[tokio::main]
async fn main() {
    // Connect to daemon
    let client = ClientOptions::new()
        .open(r"\\.\pipe\wezterm-utils-ipc")
        .unwrap();

    let (reader, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(reader);

    // Send request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "daemon/ping",
        "id": 1
    });

    let json = format!("{}\n", serde_json::to_string(&request).unwrap());
    writer.write_all(json.as_bytes()).await.unwrap();
    writer.flush().await.unwrap();

    // Read response
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    let response: serde_json::Value = serde_json::from_str(&line).unwrap();
    println!("Response: {}", serde_json::to_string_pretty(&response).unwrap());
}
```

## Testing

### Unit Tests

```powershell
cargo test --all-features
```

### Integration Tests

Integration tests require a running daemon:

```powershell
# Terminal 1: Start daemon
wezterm-utils-daemon start --pipe "\\.\pipe\wezterm-utils-test"

# Terminal 2: Run integration tests
cargo test --test integration_test -- --ignored
```

### Benchmarks

```powershell
cargo bench
```

Expected performance metrics:
- JSON serialization: ~1-5 μs
- Round-trip latency: <50ms p99 (target)
- Message throughput: 10,000+ msg/s

## Development

### Project Structure

```
wezterm-utils-daemon/
├── src/
│   ├── main.rs           # Entry point and CLI
│   ├── config.rs         # Configuration management
│   ├── protocol.rs       # JSON-RPC 2.0 types
│   ├── error.rs          # Error types
│   ├── connections.rs    # Connection management
│   ├── router.rs         # Message routing
│   └── server.rs         # Named pipe server
├── tests/
│   └── integration_test.rs
├── benches/
│   └── latency_benchmark.rs
├── build.ps1             # Build script
├── Cargo.toml
└── README.md
```

### Build Script Options

```powershell
# Debug build
.\build.ps1 -Profile debug

# Release build with tests
.\build.ps1 -Profile release -Test

# Release build with benchmarks
.\build.ps1 -Profile release -Bench

# Clean build
.\build.ps1 -Clean -Profile release

# Build and install
.\build.ps1 -Profile release -Install

# Custom install location
.\build.ps1 -Profile release -Install -InstallPath "C:\tools\bin"
```

## Troubleshooting

### Daemon Won't Start

1. **Check if pipe is already in use:**
   ```powershell
   Get-ChildItem \\.\pipe\ | Where-Object Name -eq "wezterm-utils-ipc"
   ```

2. **Try a different pipe name:**
   ```powershell
   wezterm-utils-daemon start --pipe "\\.\pipe\wezterm-utils-alt"
   ```

### Connection Refused

1. **Verify daemon is running:**
   ```powershell
   Get-Process wezterm-utils-daemon
   ```

2. **Check logs:**
   ```powershell
   wezterm-utils-daemon start --log-level debug
   ```

### High Latency

1. **Check active connections:**
   ```powershell
   wezterm-utils-daemon status
   ```

2. **Increase connection limit:**
   ```powershell
   wezterm-utils-daemon start --max-connections 20
   ```

3. **Run benchmarks:**
   ```powershell
   cargo bench
   ```

## Performance Considerations

- **Named Pipes**: Windows named pipes provide low-latency IPC (~1-10ms)
- **Connection Pooling**: Reuse connections instead of creating new ones
- **Message Batching**: Batch multiple messages when possible
- **Keep-Alive**: Use ping method to keep connections alive
- **Timeouts**: Configure appropriate timeouts for your use case

## Security

- **Process Validation**: Enable `validate_process_ids` to verify connecting processes
- **Local Only**: Named pipe server rejects remote connections
- **No Authentication**: Currently no authentication mechanism (assumes trusted local environment)

## Roadmap

- [ ] Unix domain socket support for WSL
- [ ] Authentication and authorization
- [ ] Message persistence and replay
- [ ] Metrics and monitoring endpoints
- [ ] Windows service installation
- [ ] Dynamic routing rules
- [ ] Message filtering and transformation

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please ensure:
- Code passes `cargo clippy`
- Tests pass (`cargo test`)
- Benchmarks don't regress significantly
- Documentation is updated

## Related Projects

- [WezTerm](https://wezfurlong.org/wezterm/) - GPU-accelerated terminal emulator
- [rust-commander](https://github.com/david-t-martel/rust-commander) - Desktop command execution
- [rust-fs](https://github.com/david-t-martel/rust-fs) - Filesystem MCP server

## Acknowledgments

Built with:
- [tokio](https://tokio.rs/) - Async runtime
- [serde](https://serde.rs/) - Serialization framework
- [tracing](https://tracing.rs/) - Structured logging
- [clap](https://clap.rs/) - CLI parsing