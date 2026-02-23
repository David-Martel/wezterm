//! JSON-RPC 2.0 protocol types for wezterm-utils-daemon
//!
//! Defines the message types used for communication between utilities and the daemon.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// JSON-RPC 2.0 version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// Request ID can be string, number, or null
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::Null => write!(f, "null"),
        }
    }
}

impl RequestId {
    pub fn new_uuid() -> Self {
        RequestId::String(Uuid::new_v4().to_string())
    }
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id,
        }
    }

    /// Check if this is a notification (no id field)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }
    }

    pub fn invalid_params() -> Self {
        Self {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        }
    }

    pub fn internal_error() -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: None,
        }
    }

    pub fn custom(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: RequestId,
}

impl JsonRpcResponse {
    pub fn success(result: Value, id: RequestId) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(error: JsonRpcError, id: RequestId) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 Message (can be request or response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
}

impl JsonRpcMessage {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Event subscription for utilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub event_type: String,
    pub filter: Option<Value>,
}

/// Daemon-specific method types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum DaemonMethod {
    /// Register a utility with the daemon
    #[serde(rename = "daemon/register")]
    Register { name: String, capabilities: Vec<String> },

    /// Unregister from the daemon
    #[serde(rename = "daemon/unregister")]
    Unregister,

    /// Subscribe to events
    #[serde(rename = "daemon/subscribe")]
    Subscribe { subscriptions: Vec<EventSubscription> },

    /// Unsubscribe from events
    #[serde(rename = "daemon/unsubscribe")]
    Unsubscribe { event_types: Vec<String> },

    /// Send a message to another utility
    #[serde(rename = "daemon/send")]
    Send { target: String, message: Value },

    /// Broadcast an event to all subscribers
    #[serde(rename = "daemon/broadcast")]
    Broadcast { event_type: String, data: Value },

    /// Get daemon status
    #[serde(rename = "daemon/status")]
    Status,

    /// Ping for keep-alive
    #[serde(rename = "daemon/ping")]
    Ping,
}

/// Daemon status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub version: String,
    pub uptime_seconds: u64,
    pub active_connections: usize,
    pub total_messages: u64,
    pub max_connections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(
            "test_method",
            Some(serde_json::json!({"param": "value"})),
            Some(RequestId::Number(1)),
        );

        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.method, "test_method");
        assert_eq!(parsed.id, Some(RequestId::Number(1)));
    }

    #[test]
    fn test_response_success() {
        let resp = JsonRpcResponse::success(
            serde_json::json!({"status": "ok"}),
            RequestId::Number(1),
        );

        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_response_error() {
        let resp = JsonRpcResponse::error(
            JsonRpcError::method_not_found(),
            RequestId::Number(1),
        );

        assert!(resp.error.is_some());
        assert!(resp.result.is_none());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn test_notification() {
        let req = JsonRpcRequest::new(
            "notification",
            Some(serde_json::json!({"event": "test"})),
            None,
        );

        assert!(req.is_notification());
    }
}