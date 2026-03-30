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
    use crate::protocol::{JsonRpcMessage, JsonRpcRequest, RequestId, JSONRPC_VERSION};

    // === next_id() tests ===

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
    fn test_next_id_returns_number_variant() {
        let id = next_id();
        assert!(
            matches!(id, RequestId::Number(_)),
            "next_id should always return RequestId::Number"
        );
    }

    #[test]
    fn test_next_id_is_positive() {
        let id = next_id();
        match id {
            RequestId::Number(n) => assert!(n > 0, "ID should be positive"),
            _ => panic!("Expected numeric ID"),
        }
    }

    #[test]
    fn test_next_id_monotonic_across_multiple_calls() {
        let mut prev = match next_id() {
            RequestId::Number(n) => n,
            _ => panic!("Expected numeric ID"),
        };
        for _ in 0..10 {
            let current = match next_id() {
                RequestId::Number(n) => n,
                _ => panic!("Expected numeric ID"),
            };
            assert!(
                current > prev,
                "IDs should be strictly increasing: {} should be > {}",
                current,
                prev
            );
            prev = current;
        }
    }

    // === Default pipe/socket path tests ===

    #[cfg(windows)]
    #[test]
    fn test_default_pipe_name() {
        assert!(DEFAULT_PIPE_NAME.contains("wezterm-utils-daemon"));
        assert!(
            DEFAULT_PIPE_NAME.starts_with(r"\\.\pipe\"),
            "Windows pipe should start with named pipe prefix"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_default_socket_path() {
        assert!(DEFAULT_SOCKET_PATH.contains("wezterm-utils-daemon"));
        assert!(
            DEFAULT_SOCKET_PATH.ends_with(".sock"),
            "Unix socket should end with .sock"
        );
    }

    // === Request JSON structure tests ===

    #[test]
    fn test_ping_request_json_structure() {
        let id = RequestId::Number(1);
        let req = JsonRpcRequest::new("daemon/ping", Some(json!({})), Some(id));
        let val = serde_json::to_value(&req).expect("serialize ping request");

        assert_eq!(val["jsonrpc"], JSONRPC_VERSION);
        assert_eq!(val["method"], "daemon/ping");
        assert_eq!(val["id"], 1);
        assert!(val.get("params").is_some());
    }

    #[test]
    fn test_register_request_json_structure() {
        let id = RequestId::Number(5);
        let params = json!({
            "name": "test-utility",
            "capabilities": ["state-sync", "file-watch"],
        });
        let req = JsonRpcRequest::new("daemon/register", Some(params), Some(id));
        let val = serde_json::to_value(&req).expect("serialize register request");

        assert_eq!(val["jsonrpc"], JSONRPC_VERSION);
        assert_eq!(val["method"], "daemon/register");
        assert_eq!(val["params"]["name"], "test-utility");
        assert_eq!(
            val["params"]["capabilities"]
                .as_array()
                .expect("capabilities should be array")
                .len(),
            2
        );
    }

    #[test]
    fn test_subscribe_request_json_structure() {
        let subscriptions: Vec<Value> = vec!["panel-state", "file.changed"]
            .iter()
            .map(|et| json!({"event_type": et}))
            .collect();
        let params = json!({"subscriptions": subscriptions});
        let req = JsonRpcRequest::new("daemon/subscribe", Some(params), Some(RequestId::Number(3)));
        let val = serde_json::to_value(&req).expect("serialize subscribe request");

        assert_eq!(val["method"], "daemon/subscribe");
        let subs = val["params"]["subscriptions"]
            .as_array()
            .expect("subscriptions should be array");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0]["event_type"], "panel-state");
        assert_eq!(subs[1]["event_type"], "file.changed");
    }

    #[test]
    fn test_broadcast_request_json_structure() {
        let params = json!({
            "event_type": "panel-state",
            "data": {"explorer": true, "window_id": 1},
        });
        let req = JsonRpcRequest::new("daemon/broadcast", Some(params), Some(RequestId::Number(7)));
        let val = serde_json::to_value(&req).expect("serialize broadcast request");

        assert_eq!(val["method"], "daemon/broadcast");
        assert_eq!(val["params"]["event_type"], "panel-state");
        assert_eq!(val["params"]["data"]["explorer"], true);
    }

    #[test]
    fn test_send_to_request_json_structure() {
        let params = json!({
            "target": "watcher",
            "message": {"action": "restart"},
        });
        let req = JsonRpcRequest::new("daemon/send", Some(params), Some(RequestId::Number(8)));
        let val = serde_json::to_value(&req).expect("serialize send request");

        assert_eq!(val["method"], "daemon/send");
        assert_eq!(val["params"]["target"], "watcher");
        assert_eq!(val["params"]["message"]["action"], "restart");
    }

    #[test]
    fn test_status_request_json_structure() {
        let req = JsonRpcRequest::new("daemon/status", Some(json!({})), Some(RequestId::Number(9)));
        let val = serde_json::to_value(&req).expect("serialize status request");

        assert_eq!(val["method"], "daemon/status");
        assert_eq!(val["id"], 9);
    }

    // === Notification (no id) tests ===

    #[test]
    fn test_notify_creates_request_without_id() {
        let req = JsonRpcRequest::new("event/panel-state", Some(json!({"visible": true})), None);
        let msg = JsonRpcMessage::Request(req);
        let json = msg.to_string().expect("serialize notification");
        let val: Value = serde_json::from_str(&json).expect("parse json");

        assert!(
            val.get("id").is_none(),
            "notification should not have an id field"
        );
        assert_eq!(val["method"], "event/panel-state");
        assert_eq!(val["params"]["visible"], true);
    }

    #[test]
    fn test_notification_is_identified_correctly() {
        let notification = JsonRpcRequest::new("event/test", Some(json!({})), None);
        assert!(
            notification.is_notification(),
            "request without id should be a notification"
        );

        let request =
            JsonRpcRequest::new("daemon/ping", Some(json!({})), Some(RequestId::Number(1)));
        assert!(
            !request.is_notification(),
            "request with id should NOT be a notification"
        );
    }

    // === Message envelope tests (request vs notification) ===

    #[test]
    fn test_request_message_roundtrip() {
        let req = JsonRpcRequest::new("daemon/ping", Some(json!({})), Some(RequestId::Number(100)));
        let msg = JsonRpcMessage::Request(req);
        let json = msg.to_string().expect("serialize");
        let parsed = JsonRpcMessage::parse(&json).expect("parse back");
        match parsed {
            JsonRpcMessage::Request(r) => {
                assert_eq!(r.method, "daemon/ping");
                assert_eq!(r.id, Some(RequestId::Number(100)));
            }
            _ => panic!("expected Request after roundtrip"),
        }
    }
}
