//! Message routing logic for wezterm-utils-daemon
//!
//! Routes messages between utilities, handles daemon methods, and manages event broadcasting.

use crate::connections::ConnectionManager;
use crate::error::{DaemonError, Result};
use crate::protocol::{
    DaemonMethod, DaemonStatus, JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse,
    RequestId,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Message router handles all incoming messages and routes them appropriately
pub struct MessageRouter {
    connection_manager: Arc<ConnectionManager>,
    start_time: Instant,
    version: String,
}

impl MessageRouter {
    pub fn new(connection_manager: Arc<ConnectionManager>, version: String) -> Self {
        Self {
            connection_manager,
            start_time: Instant::now(),
            version,
        }
    }

    /// Start the router task
    pub async fn run(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<(String, JsonRpcMessage)>) {
        info!("Message router started");

        while let Some((connection_id, message)) = rx.recv().await {
            self.connection_manager.increment_messages();

            match message {
                JsonRpcMessage::Request(request) => {
                    if let Err(e) = self.handle_request(&connection_id, request).await {
                        error!(
                            connection_id = %connection_id,
                            error = %e,
                            "Failed to handle request"
                        );
                    }
                }
                JsonRpcMessage::Response(response) => {
                    if let Err(e) = self.handle_response(&connection_id, response).await {
                        error!(
                            connection_id = %connection_id,
                            error = %e,
                            "Failed to handle response"
                        );
                    }
                }
            }
        }

        warn!("Message router stopped");
    }

    async fn handle_request(&self, connection_id: &str, request: JsonRpcRequest) -> Result<()> {
        debug!(
            connection_id = %connection_id,
            method = %request.method,
            "Handling request"
        );

        // Check if this is a daemon method
        if request.method.starts_with("daemon/") {
            return self.handle_daemon_method(connection_id, request).await;
        }

        // Otherwise, route to target utility
        self.route_to_utility(connection_id, request).await
    }

    async fn handle_daemon_method(
        &self,
        connection_id: &str,
        request: JsonRpcRequest,
    ) -> Result<()> {
        let connection = self
            .connection_manager
            .get_connection(connection_id)
            .ok_or_else(|| {
                DaemonError::Connection(format!("Connection {} not found", connection_id))
            })?;

        let result = match self.parse_daemon_method(&request) {
            Ok(method) => match method {
                DaemonMethod::Register { name, capabilities } => {
                    self.handle_register(connection_id, name, capabilities)
                        .await
                }
                DaemonMethod::Unregister => self.handle_unregister(connection_id).await,
                DaemonMethod::Subscribe { subscriptions } => {
                    self.handle_subscribe(connection_id, subscriptions).await
                }
                DaemonMethod::Unsubscribe { event_types } => {
                    self.handle_unsubscribe(connection_id, event_types).await
                }
                DaemonMethod::Send { target, message } => {
                    self.handle_send(connection_id, target, message).await
                }
                DaemonMethod::Broadcast { event_type, data } => {
                    self.handle_broadcast(connection_id, event_type, data).await
                }
                DaemonMethod::Status => self.handle_status().await,
                DaemonMethod::Ping => Ok(json!({"status": "pong"})),
            },
            Err(e) => Err(e),
        };

        // Send response if request had an id
        if let Some(id) = request.id {
            let response = match result {
                Ok(value) => JsonRpcResponse::success(value, id),
                Err(e) => JsonRpcResponse::error(JsonRpcError::custom(-32000, e.to_string()), id),
            };

            connection
                .send(JsonRpcMessage::Response(response))
                .await
                .map_err(|e| {
                    DaemonError::Connection(format!("sending daemon method response: {e}"))
                })?;
        }

        Ok(())
    }

    fn parse_daemon_method(&self, request: &JsonRpcRequest) -> Result<DaemonMethod> {
        // DaemonMethod uses #[serde(tag = "method")] so it expects all fields
        // at the root level. JSON-RPC puts them under "params", so we merge
        // params into the root object before deserializing.
        let mut obj = serde_json::Map::new();
        obj.insert("method".to_string(), Value::String(request.method.clone()));
        if let Some(Value::Object(params)) = &request.params {
            for (k, v) in params {
                obj.insert(k.clone(), v.clone());
            }
        }
        serde_json::from_value(Value::Object(obj))
            .map_err(|e| DaemonError::Protocol(format!("Invalid daemon method: {}", e)))
    }

    async fn handle_register(
        &self,
        connection_id: &str,
        name: String,
        capabilities: Vec<String>,
    ) -> Result<Value> {
        let connection = self
            .connection_manager
            .get_connection(connection_id)
            .ok_or_else(|| DaemonError::UtilityNotFound(connection_id.to_string()))?;

        // Update connection metadata via interior mutability
        connection.register(name.clone(), capabilities.clone());

        info!(
            connection_id = %connection_id,
            name = %name,
            capabilities = ?capabilities,
            "Utility registered"
        );

        Ok(json!({
            "status": "registered",
            "connection_id": connection_id,
            "name": name,
        }))
    }

    async fn handle_unregister(&self, connection_id: &str) -> Result<Value> {
        self.connection_manager.remove_connection(connection_id);
        Ok(json!({"status": "unregistered"}))
    }

    async fn handle_subscribe(
        &self,
        connection_id: &str,
        subscriptions: Vec<crate::protocol::EventSubscription>,
    ) -> Result<Value> {
        let connection = self
            .connection_manager
            .get_connection(connection_id)
            .ok_or_else(|| DaemonError::UtilityNotFound(connection_id.to_string()))?;

        let count = subscriptions.len();
        connection.subscribe(subscriptions);

        info!(
            connection_id = %connection_id,
            count = count,
            "Subscriptions added"
        );

        Ok(json!({
            "status": "subscribed",
            "count": count,
        }))
    }

    async fn handle_unsubscribe(
        &self,
        connection_id: &str,
        event_types: Vec<String>,
    ) -> Result<Value> {
        let connection = self
            .connection_manager
            .get_connection(connection_id)
            .ok_or_else(|| DaemonError::UtilityNotFound(connection_id.to_string()))?;

        let count = event_types.len();
        connection.unsubscribe(&event_types);

        info!(
            connection_id = %connection_id,
            event_types = ?event_types,
            "Unsubscribed from events"
        );

        Ok(json!({
            "status": "unsubscribed",
            "count": count,
        }))
    }

    async fn handle_send(
        &self,
        _connection_id: &str,
        target: String,
        message: Value,
    ) -> Result<Value> {
        let target_conn = self
            .connection_manager
            .get_connection_by_name(&target)
            .ok_or_else(|| DaemonError::UtilityNotFound(target.clone()))?;

        let request = JsonRpcRequest::new(
            "utility/message",
            Some(message),
            Some(RequestId::new_uuid()),
        );

        target_conn
            .send(JsonRpcMessage::Request(request))
            .await
            .map_err(|e| DaemonError::Routing(format!("sending to utility '{target}': {e}")))?;

        Ok(json!({
            "status": "sent",
            "target": target,
        }))
    }

    async fn handle_broadcast(
        &self,
        connection_id: &str,
        event_type: String,
        data: Value,
    ) -> Result<Value> {
        let notification = JsonRpcRequest::new(
            format!("event/{}", event_type),
            Some(data),
            None, // Notifications have no id
        );

        let sent_to = self
            .connection_manager
            .broadcast_to_subscribers(&event_type, JsonRpcMessage::Request(notification));

        info!(
            connection_id = %connection_id,
            event_type = %event_type,
            recipients = sent_to.len(),
            "Event broadcast"
        );

        Ok(json!({
            "status": "broadcast",
            "recipients": sent_to.len(),
        }))
    }

    async fn handle_status(&self) -> Result<Value> {
        let status = DaemonStatus {
            version: self.version.clone(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            active_connections: self.connection_manager.get_active_count(),
            total_messages: self.connection_manager.get_total_messages(),
            max_connections: 10, // Should come from config
            oldest_connection_age_secs: self.connection_manager.oldest_connection_age_secs(),
            connection_uptimes: self.connection_manager.connection_ages(),
        };

        Ok(serde_json::to_value(status)?)
    }

    /// Route a request to the target utility based on method name prefix.
    ///
    /// Method names follow the pattern `<utility>/<action>`, e.g.:
    /// - `explorer/navigate` → routes to the connection named "explorer"
    /// - `watcher/subscribe` → routes to the connection named "watcher"
    ///
    /// If no prefix match is found, broadcasts to all registered utilities.
    /// Send an error response back to the requesting connection, if the request had an id.
    async fn send_error_response(
        &self,
        connection_id: &str,
        request_id: Option<RequestId>,
        error: JsonRpcError,
    ) -> Result<()> {
        if let Some(id) = request_id {
            if let Some(sender) = self.connection_manager.get_connection(connection_id) {
                sender
                    .send(JsonRpcMessage::Response(JsonRpcResponse::error(error, id)))
                    .await?;
            }
        }
        Ok(())
    }

    async fn route_to_utility(&self, connection_id: &str, request: JsonRpcRequest) -> Result<()> {
        // Extract target utility from method prefix (e.g., "explorer/navigate" -> "explorer")
        let target_name = request.method.split('/').next().unwrap_or("").to_string();

        if target_name.is_empty() {
            warn!(method = %request.method, "Cannot route: no utility prefix in method name");
            self.send_error_response(connection_id, request.id, JsonRpcError::method_not_found())
                .await?;
            return Ok(());
        }

        // Find target connection by registered name
        if let Some(target) = self.connection_manager.get_connection_by_name(&target_name) {
            debug!(
                from = %connection_id,
                to = %target_name,
                method = %request.method,
                "Routing request to utility"
            );
            target
                .send(JsonRpcMessage::Request(request))
                .await
                .map_err(|e| DaemonError::Routing(format!("routing to '{target_name}': {e}")))?;
        } else {
            warn!(
                method = %request.method,
                target = %target_name,
                "Target utility not connected"
            );
            self.send_error_response(
                connection_id,
                request.id,
                JsonRpcError::custom(-32001, format!("Utility '{target_name}' not connected")),
            )
            .await?;
        }

        Ok(())
    }

    /// Route a response back to the original requester.
    ///
    /// Uses the request_id to find the pending request tracker and
    /// forwards the response to the waiting connection.
    async fn handle_response(&self, connection_id: &str, response: JsonRpcResponse) -> Result<()> {
        debug!(
            connection_id = %connection_id,
            request_id = %response.id,
            "Routing response"
        );

        // For now, broadcast the response to all connections except the sender.
        // A full implementation would track pending requests in a HashMap<RequestId, ConnectionId>
        // and route directly to the waiting connection.
        let connections = self.connection_manager.get_all_connections();
        for conn in connections {
            if conn.id != connection_id {
                if let Err(e) = conn.send(JsonRpcMessage::Response(response.clone())).await {
                    debug!(
                        target_id = %conn.id,
                        error = %e,
                        "Failed to forward response"
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::Connection;
    use crate::protocol::EventSubscription;

    #[tokio::test]
    async fn test_router_creation() {
        let cm = Arc::new(ConnectionManager::new(10));
        let router = MessageRouter::new(cm, "0.1.0".to_string());

        assert_eq!(router.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_daemon_method_parsing() {
        let cm = Arc::new(ConnectionManager::new(10));
        let router = MessageRouter::new(cm, "0.1.0".to_string());

        let request = JsonRpcRequest::new("daemon/ping", None, Some(RequestId::Number(1)));

        let method = router
            .parse_daemon_method(&request)
            .expect("parse daemon/ping method in test");
        assert!(matches!(method, DaemonMethod::Ping));
    }

    #[tokio::test]
    async fn test_subscribe_and_broadcast_delivers_event() {
        let cm = Arc::new(ConnectionManager::new(10));
        let router = MessageRouter::new(Arc::clone(&cm), "0.1.0".to_string());

        // Create a connection with a channel we can read from
        let (tx, mut rx) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        let conn_id = conn.id.clone();
        let conn = cm.add_connection(conn).expect("add connection in test");

        // Subscribe through the router handler
        let subscriptions = vec![EventSubscription {
            event_type: "file.changed".to_string(),
            filter: None,
        }];
        let result = router
            .handle_subscribe(&conn_id, subscriptions)
            .await
            .expect("subscribe should succeed");
        assert_eq!(result["status"], "subscribed");
        assert_eq!(result["count"], 1);

        // Verify the connection is actually subscribed
        assert!(
            conn.is_subscribed_to("file.changed"),
            "connection should be subscribed after handle_subscribe"
        );

        // Broadcast an event through the connection manager
        let notification = JsonRpcRequest::new(
            "event/file.changed",
            Some(json!({"path": "/tmp/test.txt"})),
            None,
        );
        let recipients =
            cm.broadcast_to_subscribers("file.changed", JsonRpcMessage::Request(notification));

        assert_eq!(recipients.len(), 1, "broadcast should reach one subscriber");
        assert_eq!(recipients[0], conn_id);

        // Verify the message was actually delivered to the channel
        let delivered = rx
            .try_recv()
            .expect("channel should contain the broadcast message");
        match delivered {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "event/file.changed");
            }
            _ => panic!("expected a Request message from broadcast"),
        }
    }

    #[tokio::test]
    async fn test_unsubscribe_stops_delivery() {
        let cm = Arc::new(ConnectionManager::new(10));
        let router = MessageRouter::new(Arc::clone(&cm), "0.1.0".to_string());

        let (tx, mut rx) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        let conn_id = conn.id.clone();
        let conn = cm.add_connection(conn).expect("add connection in test");

        // Subscribe then unsubscribe
        let subscriptions = vec![EventSubscription {
            event_type: "file.changed".to_string(),
            filter: None,
        }];
        router
            .handle_subscribe(&conn_id, subscriptions)
            .await
            .expect("subscribe should succeed");
        assert!(conn.is_subscribed_to("file.changed"));

        let result = router
            .handle_unsubscribe(&conn_id, vec!["file.changed".to_string()])
            .await
            .expect("unsubscribe should succeed");
        assert_eq!(result["status"], "unsubscribed");

        // Connection should no longer be subscribed
        assert!(
            !conn.is_subscribed_to("file.changed"),
            "connection should not be subscribed after handle_unsubscribe"
        );

        // Broadcast should reach nobody
        let notification = JsonRpcRequest::new("event/file.changed", Some(json!({})), None);
        let recipients =
            cm.broadcast_to_subscribers("file.changed", JsonRpcMessage::Request(notification));
        assert!(
            recipients.is_empty(),
            "broadcast should reach no subscribers after unsubscribe"
        );

        // Channel should be empty
        assert!(
            rx.try_recv().is_err(),
            "channel should be empty — no event should have been delivered"
        );
    }

    #[tokio::test]
    async fn test_status_includes_oldest_connection_age() {
        let cm = Arc::new(ConnectionManager::new(10));
        let router = MessageRouter::new(Arc::clone(&cm), "0.1.0".to_string());

        // No connections — oldest_connection_age_secs should be null, uptimes empty
        let status_value = router
            .handle_status()
            .await
            .expect("handle_status should succeed");
        let status: DaemonStatus =
            serde_json::from_value(status_value).expect("status should deserialize");
        assert_eq!(status.oldest_connection_age_secs, None);
        assert!(status.connection_uptimes.is_empty());

        // Add a connection — oldest_connection_age_secs should be Some
        let (tx, _rx) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        let conn_id = conn.id.clone();
        cm.add_connection(conn).expect("add connection in test");

        let status_value = router
            .handle_status()
            .await
            .expect("handle_status should succeed");
        let status: DaemonStatus =
            serde_json::from_value(status_value).expect("status should deserialize");
        assert!(
            status.oldest_connection_age_secs.is_some(),
            "should report oldest connection age when connections exist"
        );
        assert!(
            status.oldest_connection_age_secs.expect("checked above") < 5,
            "freshly created connection should be < 5s old"
        );
        assert_eq!(status.active_connections, 1);
        assert_eq!(status.connection_uptimes.len(), 1);
        assert_eq!(status.connection_uptimes[0].0, conn_id);
    }
}
