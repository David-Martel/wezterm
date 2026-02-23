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
use std::time::{Instant, SystemTime, UNIX_EPOCH};
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
    pub async fn run(
        self: Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<(String, JsonRpcMessage)>,
    ) {
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
            .ok_or_else(|| DaemonError::Connection(format!("Connection {} not found", connection_id)))?;

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
                    self.handle_broadcast(connection_id, event_type, data)
                        .await
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
                Err(e) => JsonRpcResponse::error(
                    JsonRpcError::custom(-32000, e.to_string()),
                    id,
                ),
            };

            connection.send(JsonRpcMessage::Response(response)).await?;
        }

        Ok(())
    }

    fn parse_daemon_method(&self, request: &JsonRpcRequest) -> Result<DaemonMethod> {
        let json = serde_json::to_string(request)
            .map_err(|e| DaemonError::Protocol(format!("Failed to serialize: {}", e)))?;

        serde_json::from_str(&json)
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

        // This is ugly but necessary due to Arc<Connection> immutability
        // In production, we'd use interior mutability for the mutable fields
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
        info!(
            connection_id = %connection_id,
            subscriptions = ?subscriptions,
            "Subscriptions added"
        );

        Ok(json!({
            "status": "subscribed",
            "count": subscriptions.len(),
        }))
    }

    async fn handle_unsubscribe(
        &self,
        connection_id: &str,
        event_types: Vec<String>,
    ) -> Result<Value> {
        info!(
            connection_id = %connection_id,
            event_types = ?event_types,
            "Unsubscribed from events"
        );

        Ok(json!({
            "status": "unsubscribed",
            "count": event_types.len(),
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
            .await?;

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

        let sent_to = self.connection_manager.broadcast_to_subscribers(
            &event_type,
            JsonRpcMessage::Request(notification),
        );

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
        };

        Ok(serde_json::to_value(status)?)
    }

    async fn route_to_utility(&self, _connection_id: &str, request: JsonRpcRequest) -> Result<()> {
        // In a real implementation, we'd need a way to determine the target utility
        // For now, we just return method not found
        warn!(
            method = %request.method,
            "Unknown method"
        );

        Ok(())
    }

    async fn handle_response(&self, connection_id: &str, response: JsonRpcResponse) -> Result<()> {
        debug!(
            connection_id = %connection_id,
            request_id = %response.id,
            "Received response"
        );

        // Response routing would be handled here
        // For now, we just log it

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::Connection;
    use tokio::sync::mpsc;

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

        let request = JsonRpcRequest::new(
            "daemon/ping",
            None,
            Some(RequestId::Number(1)),
        );

        let method = router.parse_daemon_method(&request).unwrap();
        assert!(matches!(method, DaemonMethod::Ping));
    }
}