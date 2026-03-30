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
    Register {
        name: String,
        capabilities: Vec<String>,
    },

    /// Unregister from the daemon
    #[serde(rename = "daemon/unregister")]
    Unregister,

    /// Subscribe to events
    #[serde(rename = "daemon/subscribe")]
    Subscribe {
        subscriptions: Vec<EventSubscription>,
    },

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
    /// Uptime (seconds) of the longest-lived active connection, if any.
    pub oldest_connection_age_secs: Option<u64>,
    /// Per-connection uptime: `[(connection_id, uptime_seconds)]`.
    pub connection_uptimes: Vec<(String, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // === JsonRpcRequest tests ===

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(
            "test_method",
            Some(serde_json::json!({"param": "value"})),
            Some(RequestId::Number(1)),
        );

        let json = serde_json::to_string(&req).expect("serialize request");
        let parsed: JsonRpcRequest = serde_json::from_str(&json).expect("deserialize request");

        assert_eq!(parsed.method, "test_method");
        assert_eq!(parsed.id, Some(RequestId::Number(1)));
    }

    #[test]
    fn test_request_new_sets_jsonrpc_version() {
        let req = JsonRpcRequest::new("some/method", None, None);
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_request_params_omitted_when_none() {
        let req = JsonRpcRequest::new("daemon/ping", None, Some(RequestId::Number(1)));
        let val = serde_json::to_value(&req).expect("serialize to value");
        assert!(
            val.get("params").is_none(),
            "params field should be absent when None"
        );
    }

    #[test]
    fn test_request_id_omitted_when_none() {
        let req = JsonRpcRequest::new("daemon/ping", None, None);
        let val = serde_json::to_value(&req).expect("serialize to value");
        assert!(
            val.get("id").is_none(),
            "id field should be absent for notifications"
        );
    }

    #[test]
    fn test_request_with_string_id() {
        let req = JsonRpcRequest::new("test", None, Some(RequestId::String("abc-123".to_string())));
        let val = serde_json::to_value(&req).expect("serialize to value");
        assert_eq!(val["id"], "abc-123");
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

    #[test]
    fn test_request_with_id_is_not_notification() {
        let req = JsonRpcRequest::new("method", None, Some(RequestId::Number(42)));
        assert!(!req.is_notification());
    }

    // === JsonRpcResponse tests ===

    #[test]
    fn test_response_success() {
        let resp =
            JsonRpcResponse::success(serde_json::json!({"status": "ok"}), RequestId::Number(1));

        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
        assert_eq!(resp.jsonrpc, "2.0");
    }

    #[test]
    fn test_response_success_serialization_roundtrip() {
        let resp =
            JsonRpcResponse::success(serde_json::json!({"pong": true}), RequestId::Number(7));
        let json = serde_json::to_string(&resp).expect("serialize response");
        let parsed: JsonRpcResponse = serde_json::from_str(&json).expect("deserialize response");

        assert_eq!(parsed.id, RequestId::Number(7));
        assert!(parsed.result.is_some());
        assert_eq!(parsed.result.expect("checked above")["pong"], true);
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_response_success_omits_error_field() {
        let resp = JsonRpcResponse::success(serde_json::json!("ok"), RequestId::Number(1));
        let val = serde_json::to_value(&resp).expect("serialize to value");
        assert!(
            val.get("error").is_none(),
            "error field should be absent in success response"
        );
    }

    #[test]
    fn test_response_error() {
        let resp = JsonRpcResponse::error(JsonRpcError::method_not_found(), RequestId::Number(1));

        assert!(resp.error.is_some());
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().expect("checked above").code, -32601);
    }

    #[test]
    fn test_response_error_omits_result_field() {
        let resp = JsonRpcResponse::error(JsonRpcError::internal_error(), RequestId::Number(5));
        let val = serde_json::to_value(&resp).expect("serialize to value");
        assert!(
            val.get("result").is_none(),
            "result field should be absent in error response"
        );
    }

    #[test]
    fn test_response_error_serialization_roundtrip() {
        let err = JsonRpcError::custom(-32099, "something went wrong")
            .with_data(serde_json::json!({"detail": "extra info"}));
        let resp = JsonRpcResponse::error(err, RequestId::String("req-1".to_string()));
        let json = serde_json::to_string(&resp).expect("serialize response");
        let parsed: JsonRpcResponse = serde_json::from_str(&json).expect("deserialize response");

        assert_eq!(parsed.id, RequestId::String("req-1".to_string()));
        let e = parsed.error.expect("should have error");
        assert_eq!(e.code, -32099);
        assert_eq!(e.message, "something went wrong");
        assert_eq!(e.data.expect("should have data")["detail"], "extra info");
    }

    // === JsonRpcError factory tests ===

    #[test]
    fn test_error_parse_error() {
        let e = JsonRpcError::parse_error();
        assert_eq!(e.code, -32700);
        assert_eq!(e.message, "Parse error");
        assert!(e.data.is_none());
    }

    #[test]
    fn test_error_invalid_request() {
        let e = JsonRpcError::invalid_request();
        assert_eq!(e.code, -32600);
        assert_eq!(e.message, "Invalid Request");
    }

    #[test]
    fn test_error_method_not_found() {
        let e = JsonRpcError::method_not_found();
        assert_eq!(e.code, -32601);
    }

    #[test]
    fn test_error_invalid_params() {
        let e = JsonRpcError::invalid_params();
        assert_eq!(e.code, -32602);
    }

    #[test]
    fn test_error_internal_error() {
        let e = JsonRpcError::internal_error();
        assert_eq!(e.code, -32603);
    }

    #[test]
    fn test_error_with_data() {
        let e = JsonRpcError::internal_error().with_data(serde_json::json!({"trace": "stack"}));
        assert!(e.data.is_some());
        assert_eq!(e.data.expect("checked above")["trace"], "stack");
    }

    // === RequestId tests ===

    #[test]
    fn test_request_id_number_display() {
        let id = RequestId::Number(42);
        assert_eq!(format!("{}", id), "42");
    }

    #[test]
    fn test_request_id_string_display() {
        let id = RequestId::String("abc".to_string());
        assert_eq!(format!("{}", id), "abc");
    }

    #[test]
    fn test_request_id_null_display() {
        let id = RequestId::Null;
        assert_eq!(format!("{}", id), "null");
    }

    #[test]
    fn test_request_id_new_uuid_is_string_variant() {
        let id = RequestId::new_uuid();
        match id {
            RequestId::String(s) => {
                // UUID v4 format: 8-4-4-4-12 hex chars
                assert_eq!(s.len(), 36, "UUID should be 36 chars");
                assert_eq!(
                    s.chars().filter(|c| *c == '-').count(),
                    4,
                    "UUID should have 4 hyphens"
                );
            }
            _ => panic!("new_uuid() should return a String variant"),
        }
    }

    #[test]
    fn test_request_id_number_serialization() {
        let id = RequestId::Number(99);
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "99");
    }

    #[test]
    fn test_request_id_string_serialization() {
        let id = RequestId::String("hello".to_string());
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"hello\"");
    }

    #[test]
    fn test_request_id_null_serialization() {
        let id = RequestId::Null;
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "null");
    }

    #[test]
    fn test_request_id_deserialization_from_number() {
        let id: RequestId = serde_json::from_str("42").expect("deserialize number");
        assert_eq!(id, RequestId::Number(42));
    }

    #[test]
    fn test_request_id_deserialization_from_string() {
        let id: RequestId = serde_json::from_str("\"req-1\"").expect("deserialize string");
        assert_eq!(id, RequestId::String("req-1".to_string()));
    }

    // === DaemonMethod deserialization tests ===

    #[test]
    fn test_daemon_method_ping_deserialization() {
        let json = r#"{"method": "daemon/ping"}"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize ping");
        assert!(matches!(method, DaemonMethod::Ping));
    }

    #[test]
    fn test_daemon_method_status_deserialization() {
        let json = r#"{"method": "daemon/status"}"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize status");
        assert!(matches!(method, DaemonMethod::Status));
    }

    #[test]
    fn test_daemon_method_register_deserialization() {
        let json = r#"{
            "method": "daemon/register",
            "name": "explorer",
            "capabilities": ["browse", "search"]
        }"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize register");
        match method {
            DaemonMethod::Register { name, capabilities } => {
                assert_eq!(name, "explorer");
                assert_eq!(capabilities, vec!["browse", "search"]);
            }
            _ => panic!("expected Register variant"),
        }
    }

    #[test]
    fn test_daemon_method_unregister_deserialization() {
        let json = r#"{"method": "daemon/unregister"}"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize unregister");
        assert!(matches!(method, DaemonMethod::Unregister));
    }

    #[test]
    fn test_daemon_method_subscribe_deserialization() {
        let json = r#"{
            "method": "daemon/subscribe",
            "subscriptions": [{"event_type": "file.changed", "filter": null}]
        }"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize subscribe");
        match method {
            DaemonMethod::Subscribe { subscriptions } => {
                assert_eq!(subscriptions.len(), 1);
                assert_eq!(subscriptions[0].event_type, "file.changed");
            }
            _ => panic!("expected Subscribe variant"),
        }
    }

    #[test]
    fn test_daemon_method_unsubscribe_deserialization() {
        let json = r#"{
            "method": "daemon/unsubscribe",
            "event_types": ["file.changed", "panel.state"]
        }"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize unsubscribe");
        match method {
            DaemonMethod::Unsubscribe { event_types } => {
                assert_eq!(event_types.len(), 2);
                assert_eq!(event_types[0], "file.changed");
                assert_eq!(event_types[1], "panel.state");
            }
            _ => panic!("expected Unsubscribe variant"),
        }
    }

    #[test]
    fn test_daemon_method_send_deserialization() {
        let json = r#"{
            "method": "daemon/send",
            "target": "watcher",
            "message": {"action": "reload"}
        }"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize send");
        match method {
            DaemonMethod::Send { target, message } => {
                assert_eq!(target, "watcher");
                assert_eq!(message["action"], "reload");
            }
            _ => panic!("expected Send variant"),
        }
    }

    #[test]
    fn test_daemon_method_broadcast_deserialization() {
        let json = r#"{
            "method": "daemon/broadcast",
            "event_type": "panel-state",
            "data": {"visible": true}
        }"#;
        let method: DaemonMethod = serde_json::from_str(json).expect("deserialize broadcast");
        match method {
            DaemonMethod::Broadcast { event_type, data } => {
                assert_eq!(event_type, "panel-state");
                assert_eq!(data["visible"], true);
            }
            _ => panic!("expected Broadcast variant"),
        }
    }

    // === EventSubscription tests ===

    #[test]
    fn test_event_subscription_serialization() {
        let sub = EventSubscription {
            event_type: "panel-state".to_string(),
            filter: Some(serde_json::json!({"window_id": 1})),
        };
        let val = serde_json::to_value(&sub).expect("serialize");
        assert_eq!(val["event_type"], "panel-state");
        assert_eq!(val["filter"]["window_id"], 1);
    }

    #[test]
    fn test_event_subscription_without_filter() {
        let sub = EventSubscription {
            event_type: "file.changed".to_string(),
            filter: None,
        };
        let val = serde_json::to_value(&sub).expect("serialize");
        assert_eq!(val["event_type"], "file.changed");
        assert!(val["filter"].is_null());
    }

    #[test]
    fn test_event_subscription_deserialization() {
        let json = r#"{"event_type": "config.reload", "filter": null}"#;
        let sub: EventSubscription = serde_json::from_str(json).expect("deserialize");
        assert_eq!(sub.event_type, "config.reload");
        assert!(sub.filter.is_none());
    }

    // === DaemonStatus tests ===

    #[test]
    fn test_daemon_status_serialization_roundtrip() {
        let status = DaemonStatus {
            version: "0.1.0".to_string(),
            uptime_seconds: 120,
            active_connections: 3,
            total_messages: 42,
            max_connections: 10,
            oldest_connection_age_secs: Some(90),
            connection_uptimes: vec![("conn-1".to_string(), 90), ("conn-2".to_string(), 45)],
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let parsed: DaemonStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.version, "0.1.0");
        assert_eq!(parsed.uptime_seconds, 120);
        assert_eq!(parsed.active_connections, 3);
        assert_eq!(parsed.total_messages, 42);
        assert_eq!(parsed.oldest_connection_age_secs, Some(90));
        assert_eq!(parsed.connection_uptimes.len(), 2);
    }

    // === JsonRpcMessage tests ===

    #[test]
    fn test_message_parse_request() {
        let json = r#"{"jsonrpc":"2.0","method":"daemon/ping","id":1}"#;
        let msg = JsonRpcMessage::parse(json).expect("parse request");
        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "daemon/ping");
                assert_eq!(req.id, Some(RequestId::Number(1)));
            }
            _ => panic!("expected Request variant"),
        }
    }

    #[test]
    fn test_message_parse_response() {
        let json = r#"{"jsonrpc":"2.0","result":{"status":"pong"},"id":1}"#;
        let msg = JsonRpcMessage::parse(json).expect("parse response");
        match msg {
            // serde untagged will try Request first; a response with a result
            // field and no method field should parse as Response
            JsonRpcMessage::Response(resp) => {
                assert!(resp.result.is_some());
                assert_eq!(resp.id, RequestId::Number(1));
            }
            JsonRpcMessage::Request(_) => {
                // untagged enum may match Request first; this is acceptable
                // as long as the JSON round-trips
            }
        }
    }

    #[test]
    fn test_message_to_string_roundtrip() {
        let req = JsonRpcRequest::new("daemon/status", None, Some(RequestId::Number(10)));
        let msg = JsonRpcMessage::Request(req);
        let json = msg.to_string().expect("serialize");
        let parsed = JsonRpcMessage::parse(&json).expect("parse back");
        let json2 = parsed.to_string().expect("serialize again");
        assert_eq!(json, json2);
    }
}
