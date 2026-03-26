//! IPC client for connecting to the wezterm-utils-daemon.
//!
//! Provides a lightweight async client that utilities and the module framework
//! can use to communicate with the daemon via Named Pipes (Windows) or
//! Unix Domain Sockets.
//!
//! ## Panel State Sync
//!
//! The primary use case is synchronizing panel state (open/closed, sizes)
//! across multiple WezTerm windows:
//!
//! ```rust,ignore
//! let client = DaemonClient::connect().await?;
//! client.register("panels", vec!["state-sync"]).await?;
//! client.subscribe(&["panel-state"]).await?;
//! client.broadcast("panel-state", &json!({"explorer": true})).await?;
//! ```

use crate::error::{DaemonError, Result};
use crate::protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, RequestId};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Default pipe name for the daemon on Windows.
#[cfg(windows)]
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\wezterm-utils-daemon";

/// Default socket path for the daemon on Unix.
#[cfg(unix)]
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/wezterm-utils-daemon.sock";

/// Monotonically increasing request ID generator.
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> RequestId {
    RequestId::Number(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed) as i64)
}

/// Async IPC client for the wezterm-utils-daemon.
///
/// Thread-safe via internal Mutex on the pipe stream.
/// Designed for use from the module framework or directly from Lua FFI.
#[derive(Debug)]
pub struct DaemonClient {
    #[cfg(windows)]
    writer: Mutex<tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>>,
    #[cfg(windows)]
    reader: Mutex<BufReader<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>>>,

    #[cfg(unix)]
    writer: Mutex<tokio::io::WriteHalf<tokio::net::UnixStream>>,
    #[cfg(unix)]
    reader: Mutex<BufReader<tokio::io::ReadHalf<tokio::net::UnixStream>>>,

    /// The name this client registered with, if any.
    registered_name: Mutex<Option<String>>,
}

impl DaemonClient {
    /// Connect to the daemon using the default pipe name or socket path.
    pub async fn connect() -> Result<Self> {
        #[cfg(windows)]
        {
            Self::connect_to(DEFAULT_PIPE_NAME).await
        }
        #[cfg(unix)]
        {
            Self::connect_to(DEFAULT_SOCKET_PATH).await
        }
    }

    /// Connect to the daemon at a specific pipe path or socket path.
    pub async fn connect_to(path: &str) -> Result<Self> {
        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;

            let client = ClientOptions::new().open(path).map_err(|e| {
                DaemonError::Connection(format!("Failed to connect to {}: {}", path, e))
            })?;

            let (read_half, write_half) = tokio::io::split(client);

            debug!(pipe = %path, "Connected to daemon");

            Ok(Self {
                writer: Mutex::new(write_half),
                reader: Mutex::new(BufReader::new(read_half)),
                registered_name: Mutex::new(None),
            })
        }
        #[cfg(unix)]
        {
            let stream = tokio::net::UnixStream::connect(path).await.map_err(|e| {
                DaemonError::Connection(format!("Failed to connect to {}: {}", path, e))
            })?;

            let (read_half, write_half) = tokio::io::split(stream);

            debug!(socket = %path, "Connected to daemon");

            Ok(Self {
                writer: Mutex::new(write_half),
                reader: Mutex::new(BufReader::new(read_half)),
                registered_name: Mutex::new(None),
            })
        }
    }

    /// Send a JSON-RPC request and wait for the response.
    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = next_id();
        let request = JsonRpcRequest::new(method, Some(params), Some(id.clone()));
        let msg = JsonRpcMessage::Request(request);

        let json = msg
            .to_string()
            .map_err(|e| DaemonError::Protocol(format!("Serialize failed: {}", e)))?;

        // Send
        {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(format!("{}\n", json).as_bytes())
                .await
                .map_err(|e| DaemonError::Connection(format!("Write failed: {}", e)))?;
            writer
                .flush()
                .await
                .map_err(|e| DaemonError::Connection(format!("Flush failed: {}", e)))?;
        }

        // Read response with timeout
        let mut line = String::new();
        let read_future = async {
            let mut reader = self.reader.lock().await;
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| DaemonError::Connection(format!("Read failed: {}", e)))
        };

        tokio::time::timeout(Duration::from_secs(5), read_future)
            .await
            .map_err(|_| {
                DaemonError::Timeout(format!("Request '{}' timed out after 5s", method))
            })??;

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| DaemonError::Protocol(format!("Parse response failed: {}", e)))?;

        if let Some(error) = response.error {
            return Err(DaemonError::Protocol(format!(
                "Daemon error {}: {}",
                error.code, error.message
            )));
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    /// Send a fire-and-forget notification (no response expected).
    #[expect(dead_code, reason = "reserved for event notification protocol")]
    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let request = JsonRpcRequest::new(method, Some(params), None);
        let msg = JsonRpcMessage::Request(request);

        let json = msg
            .to_string()
            .map_err(|e| DaemonError::Protocol(format!("Serialize failed: {}", e)))?;

        let mut writer = self.writer.lock().await;
        writer
            .write_all(format!("{}\n", json).as_bytes())
            .await
            .map_err(|e| DaemonError::Connection(format!("Write failed: {}", e)))?;
        writer
            .flush()
            .await
            .map_err(|e| DaemonError::Connection(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    // === High-Level API ===

    /// Register this client as a named utility with the daemon.
    pub async fn register(&self, name: &str, capabilities: Vec<String>) -> Result<Value> {
        let result = self
            .request(
                "daemon/register",
                json!({
                    "name": name,
                    "capabilities": capabilities,
                }),
            )
            .await?;

        *self.registered_name.lock().await = Some(name.to_string());
        debug!(name = %name, "Registered with daemon");
        Ok(result)
    }

    /// Subscribe to event types.
    pub async fn subscribe(&self, event_types: &[&str]) -> Result<Value> {
        let subscriptions: Vec<Value> = event_types
            .iter()
            .map(|et| json!({"event_type": et}))
            .collect();

        self.request(
            "daemon/subscribe",
            json!({
                "subscriptions": subscriptions,
            }),
        )
        .await
    }

    /// Broadcast an event to all subscribers of the given type.
    pub async fn broadcast(&self, event_type: &str, data: &Value) -> Result<Value> {
        self.request(
            "daemon/broadcast",
            json!({
                "event_type": event_type,
                "data": data,
            }),
        )
        .await
    }

    /// Send a message to a specific named utility.
    pub async fn send_to(&self, target: &str, message: &Value) -> Result<Value> {
        self.request(
            "daemon/send",
            json!({
                "target": target,
                "message": message,
            }),
        )
        .await
    }

    /// Ping the daemon (keep-alive).
    pub async fn ping(&self) -> Result<Value> {
        self.request("daemon/ping", json!({})).await
    }

    /// Get daemon status.
    pub async fn status(&self) -> Result<Value> {
        self.request("daemon/status", json!({})).await
    }

    /// Broadcast panel state update.
    ///
    /// Convenience method for the panel sync use case.
    pub async fn sync_panel_state(&self, window_id: u64, panels: &Value) -> Result<Value> {
        self.broadcast(
            "panel-state",
            &json!({
                "window_id": window_id,
                "panels": panels,
                "timestamp": chrono::Utc::now().timestamp(),
            }),
        )
        .await
    }

    /// Check if the daemon is reachable.
    pub async fn is_available(&self) -> bool {
        self.ping().await.is_ok()
    }
}

/// Try to connect to the daemon, returning None if unavailable.
///
/// This is the recommended entry point — it won't block or error
/// if the daemon isn't running.
pub async fn try_connect() -> Option<DaemonClient> {
    match DaemonClient::connect().await {
        Ok(client) => {
            if client.is_available().await {
                Some(client)
            } else {
                warn!("Daemon connected but not responding to ping");
                None
            }
        }
        Err(e) => {
            debug!(error = %e, "Daemon not available");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_id_increments() {
        let id1 = next_id();
        let id2 = next_id();
        match (id1, id2) {
            (RequestId::Number(a), RequestId::Number(b)) => assert!(b > a),
            _ => panic!("Expected numeric IDs"),
        }
    }

    #[test]
    fn test_default_pipe_name() {
        assert!(DEFAULT_PIPE_NAME.contains("wezterm-utils-daemon"));
    }
}
