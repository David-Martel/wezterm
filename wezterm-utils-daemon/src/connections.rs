//! Connection management for wezterm-utils-daemon
//!
//! Manages active connections to utilities, connection pooling, and keep-alive.

use crate::error::{DaemonError, Result};
use crate::protocol::{EventSubscription, JsonRpcMessage};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{split, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
#[cfg(windows)]
#[expect(
    unused_imports,
    reason = "reserved for future named pipe listener implementation"
)]
use tokio::net::windows::named_pipe::NamedPipeServer;

/// Trait to allow abstraction over different stream types
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Stream for T {}
use tokio::sync::mpsc;
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Maximum message size (1MB)
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Connection keep-alive interval — used by the periodic heartbeat cleanup task
pub(crate) const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// Connection timeout after no activity
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(120);

/// Represents a single utility connection.
///
/// Fields that can change after creation use interior mutability (RwLock)
/// so they can be updated through `Arc<Connection>` without &mut self.
#[derive(Debug)]
pub struct Connection {
    pub id: String,
    pub name: RwLock<Option<String>>,
    pub capabilities: RwLock<Vec<String>>,
    pub subscriptions: RwLock<Vec<EventSubscription>>,
    pub connected_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    pub tx: mpsc::UnboundedSender<JsonRpcMessage>,
}

impl Connection {
    pub fn new(tx: mpsc::UnboundedSender<JsonRpcMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: RwLock::new(None),
            capabilities: RwLock::new(Vec::new()),
            subscriptions: RwLock::new(Vec::new()),
            connected_at: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            tx,
        }
    }

    /// Register the connection with a name and capabilities.
    /// Uses interior mutability — safe to call through Arc<Connection>.
    pub fn register(&self, name: String, capabilities: Vec<String>) {
        *self.name.write() = Some(name);
        *self.capabilities.write() = capabilities;
        self.update_activity();
    }

    pub fn subscribe(&self, subscriptions: Vec<EventSubscription>) {
        self.subscriptions.write().extend(subscriptions);
        self.update_activity();
    }

    pub fn unsubscribe(&self, event_types: &[String]) {
        self.subscriptions
            .write()
            .retain(|sub| !event_types.contains(&sub.event_type));
        self.update_activity();
    }

    pub fn update_activity(&self) {
        *self.last_activity.write() = Instant::now();
    }

    pub fn is_timed_out(&self) -> bool {
        self.last_activity.read().elapsed() > CONNECTION_TIMEOUT
    }

    pub fn is_subscribed_to(&self, event_type: &str) -> bool {
        self.subscriptions
            .read()
            .iter()
            .any(|sub| sub.event_type == event_type)
    }

    pub async fn send(&self, message: JsonRpcMessage) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|_| DaemonError::Connection("Failed to send message".to_string()))?;
        self.update_activity();
        Ok(())
    }
}

/// Manages all active connections
pub struct ConnectionManager {
    connections: Arc<DashMap<String, Arc<Connection>>>,
    max_connections: usize,
    total_messages: Arc<RwLock<u64>>,
}

impl ConnectionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            max_connections,
            total_messages: Arc::new(RwLock::new(0)),
        }
    }

    pub fn add_connection(&self, connection: Connection) -> Result<Arc<Connection>> {
        if self.connections.len() >= self.max_connections {
            return Err(DaemonError::ConnectionLimitReached(self.max_connections));
        }

        let id = connection.id.clone();
        let conn = Arc::new(connection);
        self.connections.insert(id.clone(), conn.clone());

        info!(
            connection_id = %id,
            total_connections = self.connections.len(),
            "Connection added"
        );

        Ok(conn)
    }

    pub fn remove_connection(&self, id: &str) {
        if let Some((_, conn)) = self.connections.remove(id) {
            info!(
                connection_id = %id,
                name = ?conn.name,
                total_connections = self.connections.len(),
                "Connection removed"
            );
        }
    }

    pub fn get_connection(&self, id: &str) -> Option<Arc<Connection>> {
        self.connections.get(id).map(|entry| entry.value().clone())
    }

    pub fn get_connection_by_name(&self, name: &str) -> Option<Arc<Connection>> {
        self.connections
            .iter()
            .find(|entry| {
                entry
                    .value()
                    .name
                    .read()
                    .as_ref()
                    .map(|n| n == name)
                    .unwrap_or(false)
            })
            .map(|entry| entry.value().clone())
    }

    pub fn get_all_connections(&self) -> Vec<Arc<Connection>> {
        self.connections
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_active_count(&self) -> usize {
        self.connections.len()
    }

    pub fn increment_messages(&self) {
        *self.total_messages.write() += 1;
    }

    pub fn get_total_messages(&self) -> u64 {
        *self.total_messages.read()
    }

    pub fn broadcast_to_subscribers(
        &self,
        event_type: &str,
        message: JsonRpcMessage,
    ) -> Vec<String> {
        let mut sent_to = Vec::new();

        for entry in self.connections.iter() {
            let conn = entry.value();
            if conn.is_subscribed_to(event_type) {
                if let Err(e) = conn.tx.send(message.clone()) {
                    warn!(
                        connection_id = %conn.id,
                        error = %e,
                        "Failed to send broadcast"
                    );
                } else {
                    conn.update_activity();
                    sent_to.push(conn.id.clone());
                }
            }
        }

        debug!(
            event_type = %event_type,
            subscribers = sent_to.len(),
            "Broadcast sent"
        );

        sent_to
    }

    pub fn cleanup_stale_connections(&self) -> usize {
        let mut removed = 0;
        let stale: Vec<String> = self
            .connections
            .iter()
            .filter(|entry| entry.value().is_timed_out())
            .map(|entry| entry.key().clone())
            .collect();

        for id in stale {
            self.remove_connection(&id);
            removed += 1;
        }

        if removed > 0 {
            info!(removed = removed, "Cleaned up stale connections");
        }

        removed
    }

    /// Start periodic heartbeat cleanup task.
    ///
    /// Ticks every [`KEEP_ALIVE_INTERVAL`] and removes connections that have
    /// exceeded [`CONNECTION_TIMEOUT`] without activity.
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(KEEP_ALIVE_INTERVAL);
            loop {
                interval.tick().await;
                self.cleanup_stale_connections();
            }
        });
    }

    /// Return `(connection_id, uptime_seconds)` pairs for every active connection.
    ///
    /// Uptime is measured from `connected_at` (creation time), not from last activity.
    pub fn connection_ages(&self) -> Vec<(String, u64)> {
        self.connections
            .iter()
            .map(|entry| {
                let conn = entry.value();
                (conn.id.clone(), conn.connected_at.elapsed().as_secs())
            })
            .collect()
    }

    /// Age (in seconds) of the longest-lived active connection, or `None` if
    /// there are no connections.
    pub fn oldest_connection_age_secs(&self) -> Option<u64> {
        self.connections
            .iter()
            .map(|entry| entry.value().connected_at.elapsed().as_secs())
            .max()
    }
}

/// Handle a single connection (reads messages from pipe/socket).
///
/// # Channel wiring (Tier 3.K audit — verified correct)
///
/// A **single** `mpsc::unbounded_channel()` is created per connection in
/// [`IpcServer::accept_connection`](crate::server::IpcServer). The `tx` end is
/// stored in [`Connection.tx`] so the router/broadcast system can push
/// responses and events. The `rx` end is passed here as `outbound_rx` and
/// forwarded to the writer task, which serialises messages and writes them
/// to the socket. Because both sides originate from the same channel,
/// anything sent via `Connection::send()` or
/// `ConnectionManager::broadcast_to_subscribers()` is guaranteed to reach
/// the client's socket writer.
pub async fn handle_connection<S>(
    stream: S,
    connection: Arc<Connection>,
    router_tx: mpsc::UnboundedSender<(String, JsonRpcMessage)>,
    mut outbound_rx: mpsc::UnboundedReceiver<JsonRpcMessage>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Split stream into independent read/write halves
    let (read_half, mut write_half) = split(stream);

    // Spawn writer task — receives messages from Connection.tx via outbound_rx
    let connection_id = connection.id.clone();
    tokio::spawn(async move {
        while let Some(message) = outbound_rx.recv().await {
            if let Ok(json) = message.to_string() {
                let data = format!("{}\n", json);
                if let Err(e) = write_half.write_all(data.as_bytes()).await {
                    error!(
                        connection_id = %connection_id,
                        error = %e,
                        "Failed to write to stream"
                    );
                    break;
                }
                if let Err(e) = write_half.flush().await {
                    error!(
                        connection_id = %connection_id,
                        error = %e,
                        "Failed to flush stream"
                    );
                    break;
                }
            }
        }
    });

    // Read messages from pipe
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.len() > MAX_MESSAGE_SIZE {
            warn!(
                connection_id = %connection.id,
                size = line.len(),
                "Message exceeds maximum size"
            );
            continue;
        }

        match JsonRpcMessage::parse(&line) {
            Ok(message) => {
                connection.update_activity();
                if let Err(e) = router_tx.send((connection.id.clone(), message)) {
                    error!(
                        connection_id = %connection.id,
                        error = %e,
                        "Failed to send to router"
                    );
                    break;
                }
            }
            Err(e) => {
                warn!(
                    connection_id = %connection.id,
                    error = %e,
                    "Failed to parse message"
                );
            }
        }
    }

    info!(
        connection_id = %connection.id,
        "Connection handler exiting"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{JsonRpcRequest, JsonRpcResponse, RequestId};

    // === Connection::new() tests ===

    #[test]
    fn test_connection_new_has_uuid_id() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        // UUID v4 format: 8-4-4-4-12 hex chars (36 total)
        assert_eq!(conn.id.len(), 36, "connection id should be a UUID");
        assert_eq!(
            conn.id.chars().filter(|c| *c == '-').count(),
            4,
            "UUID should have 4 hyphens"
        );
    }

    #[test]
    fn test_connection_new_starts_unregistered() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        assert!(
            conn.name.read().is_none(),
            "new connection should have no name"
        );
        assert!(
            conn.capabilities.read().is_empty(),
            "new connection should have no capabilities"
        );
        assert!(
            conn.subscriptions.read().is_empty(),
            "new connection should have no subscriptions"
        );
    }

    #[test]
    fn test_connected_at_is_set_on_creation() {
        let before = Instant::now();
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        let after = Instant::now();

        assert!(conn.connected_at >= before);
        assert!(conn.connected_at <= after);
    }

    #[test]
    fn test_connection_ids_are_unique() {
        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();
        let conn1 = Connection::new(tx1);
        let conn2 = Connection::new(tx2);
        assert_ne!(conn1.id, conn2.id, "each connection should get a unique id");
    }

    // === Connection::register() tests ===

    #[test]
    fn test_connection_register_sets_name_and_capabilities() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.register(
            "explorer".to_string(),
            vec!["browse".to_string(), "search".to_string()],
        );

        assert_eq!(conn.name.read().as_deref(), Some("explorer"),);
        assert_eq!(*conn.capabilities.read(), vec!["browse", "search"],);
    }

    #[test]
    fn test_connection_register_overwrites_previous() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.register("old-name".to_string(), vec!["cap1".to_string()]);
        conn.register(
            "new-name".to_string(),
            vec!["cap2".to_string(), "cap3".to_string()],
        );

        assert_eq!(conn.name.read().as_deref(), Some("new-name"));
        assert_eq!(conn.capabilities.read().len(), 2);
    }

    // === Connection::subscribe() / unsubscribe() tests ===

    #[test]
    fn test_connection_subscription() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.subscribe(vec![EventSubscription {
            event_type: "test.event".to_string(),
            filter: None,
        }]);

        assert!(conn.is_subscribed_to("test.event"));
        assert!(!conn.is_subscribed_to("other.event"));
    }

    #[test]
    fn test_connection_subscribe_multiple_events() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.subscribe(vec![
            EventSubscription {
                event_type: "file.changed".to_string(),
                filter: None,
            },
            EventSubscription {
                event_type: "panel.state".to_string(),
                filter: None,
            },
        ]);

        assert!(conn.is_subscribed_to("file.changed"));
        assert!(conn.is_subscribed_to("panel.state"));
        assert!(!conn.is_subscribed_to("config.reload"));
    }

    #[test]
    fn test_connection_subscribe_extends_existing() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.subscribe(vec![EventSubscription {
            event_type: "event-a".to_string(),
            filter: None,
        }]);
        conn.subscribe(vec![EventSubscription {
            event_type: "event-b".to_string(),
            filter: None,
        }]);

        assert!(conn.is_subscribed_to("event-a"));
        assert!(conn.is_subscribed_to("event-b"));
        assert_eq!(conn.subscriptions.read().len(), 2);
    }

    #[test]
    fn test_connection_unsubscribe_removes_matching() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.subscribe(vec![
            EventSubscription {
                event_type: "keep-this".to_string(),
                filter: None,
            },
            EventSubscription {
                event_type: "remove-this".to_string(),
                filter: None,
            },
        ]);

        conn.unsubscribe(&["remove-this".to_string()]);

        assert!(conn.is_subscribed_to("keep-this"));
        assert!(!conn.is_subscribed_to("remove-this"));
        assert_eq!(conn.subscriptions.read().len(), 1);
    }

    #[test]
    fn test_connection_unsubscribe_nonexistent_is_noop() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        conn.subscribe(vec![EventSubscription {
            event_type: "existing".to_string(),
            filter: None,
        }]);

        conn.unsubscribe(&["nonexistent".to_string()]);

        assert!(conn.is_subscribed_to("existing"));
        assert_eq!(conn.subscriptions.read().len(), 1);
    }

    // === Connection::is_timed_out() tests ===

    #[test]
    fn test_fresh_connection_is_not_timed_out() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        assert!(
            !conn.is_timed_out(),
            "freshly created connection should not be timed out"
        );
    }

    #[test]
    fn test_stale_connection_is_timed_out() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);
        // Backdate last_activity well past the 120s timeout
        *conn.last_activity.write() = Instant::now() - Duration::from_secs(200);
        assert!(
            conn.is_timed_out(),
            "connection with 200s of inactivity should be timed out"
        );
    }

    // === Connection::update_activity() tests ===

    #[test]
    fn test_update_activity_refreshes_timestamp() {
        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        // Backdate then refresh
        *conn.last_activity.write() = Instant::now() - Duration::from_secs(200);
        assert!(conn.is_timed_out());

        conn.update_activity();
        assert!(
            !conn.is_timed_out(),
            "update_activity should prevent timeout"
        );
    }

    // === ConnectionManager tests ===

    #[tokio::test]
    async fn test_connection_manager() {
        let manager = ConnectionManager::new(10);
        let (tx, _rx) = mpsc::unbounded_channel();

        let conn = Connection::new(tx);
        let id = conn.id.clone();

        let _added = manager
            .add_connection(conn)
            .expect("add connection to manager in test");
        assert_eq!(manager.get_active_count(), 1);

        assert!(manager.get_connection(&id).is_some());

        manager.remove_connection(&id);
        assert_eq!(manager.get_active_count(), 0);
    }

    #[tokio::test]
    async fn test_connection_limit() {
        let manager = ConnectionManager::new(2);

        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();
        let (tx3, _) = mpsc::unbounded_channel();

        assert!(manager.add_connection(Connection::new(tx1)).is_ok());
        assert!(manager.add_connection(Connection::new(tx2)).is_ok());
        assert!(manager.add_connection(Connection::new(tx3)).is_err());
    }

    #[test]
    fn test_connection_manager_new_respects_max_connections() {
        let manager = ConnectionManager::new(5);
        assert_eq!(manager.get_active_count(), 0);
        assert_eq!(manager.get_total_messages(), 0);

        // Fill to capacity
        for _ in 0..5 {
            let (tx, _) = mpsc::unbounded_channel();
            manager
                .add_connection(Connection::new(tx))
                .expect("should accept up to max_connections");
        }
        assert_eq!(manager.get_active_count(), 5);

        // One more should fail
        let (tx, _) = mpsc::unbounded_channel();
        let err = manager.add_connection(Connection::new(tx));
        assert!(err.is_err(), "should reject when at capacity");
    }

    #[test]
    fn test_connection_manager_zero_max_connections() {
        let manager = ConnectionManager::new(0);
        let (tx, _) = mpsc::unbounded_channel();
        let err = manager.add_connection(Connection::new(tx));
        assert!(
            err.is_err(),
            "max_connections=0 should reject all connections"
        );
    }

    #[test]
    fn test_get_connection_returns_none_for_unknown_id() {
        let manager = ConnectionManager::new(10);
        assert!(
            manager.get_connection("nonexistent-id").is_none(),
            "should return None for unknown connection id"
        );
    }

    #[test]
    fn test_get_connection_by_name_returns_correct_connection() {
        let manager = ConnectionManager::new(10);

        let (tx1, _) = mpsc::unbounded_channel();
        let conn1 = Connection::new(tx1);
        let id1 = conn1.id.clone();
        let arc1 = manager
            .add_connection(conn1)
            .expect("add connection in test");
        arc1.register("alpha".to_string(), vec![]);

        let (tx2, _) = mpsc::unbounded_channel();
        let conn2 = Connection::new(tx2);
        let id2 = conn2.id.clone();
        let arc2 = manager
            .add_connection(conn2)
            .expect("add connection in test");
        arc2.register("beta".to_string(), vec![]);

        let found_alpha = manager
            .get_connection_by_name("alpha")
            .expect("should find alpha");
        assert_eq!(found_alpha.id, id1);

        let found_beta = manager
            .get_connection_by_name("beta")
            .expect("should find beta");
        assert_eq!(found_beta.id, id2);
    }

    #[test]
    fn test_get_connection_by_name_returns_none_for_unregistered() {
        let manager = ConnectionManager::new(10);

        let (tx, _) = mpsc::unbounded_channel();
        manager
            .add_connection(Connection::new(tx))
            .expect("add connection in test");

        assert!(
            manager.get_connection_by_name("ghost").is_none(),
            "should return None when no connection has that name"
        );
    }

    #[test]
    fn test_get_all_connections_returns_all() {
        let manager = ConnectionManager::new(10);

        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();
        let (tx3, _) = mpsc::unbounded_channel();

        manager
            .add_connection(Connection::new(tx1))
            .expect("add connection");
        manager
            .add_connection(Connection::new(tx2))
            .expect("add connection");
        manager
            .add_connection(Connection::new(tx3))
            .expect("add connection");

        let all = manager.get_all_connections();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_remove_connection_is_idempotent() {
        let manager = ConnectionManager::new(10);
        // Removing a nonexistent connection should not panic
        manager.remove_connection("does-not-exist");
        assert_eq!(manager.get_active_count(), 0);
    }

    // === increment_messages / get_total_messages tests ===

    #[test]
    fn test_increment_and_get_total_messages() {
        let manager = ConnectionManager::new(10);
        assert_eq!(manager.get_total_messages(), 0);

        manager.increment_messages();
        manager.increment_messages();
        manager.increment_messages();

        assert_eq!(manager.get_total_messages(), 3);
    }

    // === broadcast_to_subscribers() tests ===

    #[test]
    fn test_broadcast_to_subscribers_only_reaches_subscribed() {
        let manager = ConnectionManager::new(10);

        // Connection 1: subscribed to "file.changed"
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let conn1 = Connection::new(tx1);
        let arc1 = manager.add_connection(conn1).expect("add connection");
        arc1.subscribe(vec![EventSubscription {
            event_type: "file.changed".to_string(),
            filter: None,
        }]);

        // Connection 2: subscribed to "panel.state" (different event)
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let conn2 = Connection::new(tx2);
        let arc2 = manager.add_connection(conn2).expect("add connection");
        arc2.subscribe(vec![EventSubscription {
            event_type: "panel.state".to_string(),
            filter: None,
        }]);

        // Connection 3: no subscriptions
        let (tx3, mut rx3) = mpsc::unbounded_channel();
        manager
            .add_connection(Connection::new(tx3))
            .expect("add connection");

        let notification = JsonRpcMessage::Request(JsonRpcRequest::new(
            "event/file.changed",
            Some(serde_json::json!({"path": "/tmp/test"})),
            None,
        ));

        let sent_to = manager.broadcast_to_subscribers("file.changed", notification);

        assert_eq!(
            sent_to.len(),
            1,
            "only one connection subscribed to file.changed"
        );
        assert_eq!(sent_to[0], arc1.id);

        // conn1 should have received the message
        assert!(
            rx1.try_recv().is_ok(),
            "subscribed connection should receive broadcast"
        );
        // conn2 and conn3 should not
        assert!(
            rx2.try_recv().is_err(),
            "differently-subscribed connection should not receive broadcast"
        );
        assert!(
            rx3.try_recv().is_err(),
            "unsubscribed connection should not receive broadcast"
        );
    }

    #[test]
    fn test_broadcast_to_multiple_subscribers() {
        let manager = ConnectionManager::new(10);

        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let conn1 = Connection::new(tx1);
        let arc1 = manager.add_connection(conn1).expect("add");
        arc1.subscribe(vec![EventSubscription {
            event_type: "shared-event".to_string(),
            filter: None,
        }]);

        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let conn2 = Connection::new(tx2);
        let arc2 = manager.add_connection(conn2).expect("add");
        arc2.subscribe(vec![EventSubscription {
            event_type: "shared-event".to_string(),
            filter: None,
        }]);

        let notification = JsonRpcMessage::Request(JsonRpcRequest::new(
            "event/shared-event",
            Some(serde_json::json!({})),
            None,
        ));

        let sent_to = manager.broadcast_to_subscribers("shared-event", notification);
        assert_eq!(sent_to.len(), 2);
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_broadcast_to_no_subscribers_returns_empty() {
        let manager = ConnectionManager::new(10);

        let (tx, _) = mpsc::unbounded_channel();
        manager.add_connection(Connection::new(tx)).expect("add");

        let notification = JsonRpcMessage::Request(JsonRpcRequest::new(
            "event/orphan",
            Some(serde_json::json!({})),
            None,
        ));

        let sent_to = manager.broadcast_to_subscribers("orphan", notification);
        assert!(sent_to.is_empty());
    }

    // === cleanup_stale_connections() tests ===

    #[tokio::test]
    async fn test_cleanup_stale_connections_removes_timed_out() {
        let manager = ConnectionManager::new(10);

        let (tx, _) = mpsc::unbounded_channel();
        let conn = Connection::new(tx);

        // Manually backdate the last_activity so it looks stale
        {
            let mut activity = conn.last_activity.write();
            *activity = Instant::now() - Duration::from_secs(200);
        }

        manager
            .add_connection(conn)
            .expect("add connection in test");
        assert_eq!(manager.get_active_count(), 1);

        let removed = manager.cleanup_stale_connections();
        assert_eq!(removed, 1);
        assert_eq!(manager.get_active_count(), 0);
    }

    #[test]
    fn test_cleanup_preserves_active_connections() {
        let manager = ConnectionManager::new(10);

        // One stale, one fresh
        let (tx_stale, _) = mpsc::unbounded_channel();
        let stale = Connection::new(tx_stale);
        *stale.last_activity.write() = Instant::now() - Duration::from_secs(200);
        manager.add_connection(stale).expect("add stale");

        let (tx_fresh, _) = mpsc::unbounded_channel();
        let fresh = Connection::new(tx_fresh);
        let fresh_id = fresh.id.clone();
        manager.add_connection(fresh).expect("add fresh");

        assert_eq!(manager.get_active_count(), 2);

        let removed = manager.cleanup_stale_connections();
        assert_eq!(removed, 1);
        assert_eq!(manager.get_active_count(), 1);

        // The fresh one should remain
        assert!(
            manager.get_connection(&fresh_id).is_some(),
            "active connection should survive cleanup"
        );
    }

    #[test]
    fn test_cleanup_with_no_stale_connections_returns_zero() {
        let manager = ConnectionManager::new(10);

        let (tx, _) = mpsc::unbounded_channel();
        manager
            .add_connection(Connection::new(tx))
            .expect("add connection");

        let removed = manager.cleanup_stale_connections();
        assert_eq!(removed, 0);
        assert_eq!(manager.get_active_count(), 1);
    }

    // === Connection age tests ===

    #[test]
    fn test_keep_alive_interval_is_30s() {
        assert_eq!(KEEP_ALIVE_INTERVAL, Duration::from_secs(30));
    }

    #[test]
    fn test_connection_ages_empty() {
        let manager = ConnectionManager::new(10);
        assert!(manager.connection_ages().is_empty());
        assert_eq!(manager.oldest_connection_age_secs(), None);
    }

    #[tokio::test]
    async fn test_connection_ages_returns_entries() {
        let manager = ConnectionManager::new(10);

        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();

        let conn1 = Connection::new(tx1);
        let id1 = conn1.id.clone();
        manager
            .add_connection(conn1)
            .expect("add connection 1 in test");

        let conn2 = Connection::new(tx2);
        let id2 = conn2.id.clone();
        manager
            .add_connection(conn2)
            .expect("add connection 2 in test");

        let ages = manager.connection_ages();
        assert_eq!(ages.len(), 2);

        let ids: Vec<&String> = ages.iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2));

        for (_id, secs) in &ages {
            assert!(*secs < 5, "freshly created connection should be < 5s old");
        }
    }

    #[tokio::test]
    async fn test_oldest_connection_age_secs() {
        let manager = ConnectionManager::new(10);

        assert_eq!(manager.oldest_connection_age_secs(), None);

        let (tx, _) = mpsc::unbounded_channel();
        manager
            .add_connection(Connection::new(tx))
            .expect("add connection in test");

        let age = manager
            .oldest_connection_age_secs()
            .expect("should return Some when connections exist");
        assert!(age < 5, "freshly created connection age should be < 5s");
    }

    // === Connection::send() channel wiring tests ===

    #[tokio::test]
    async fn test_connection_tx_reaches_writer_rx() {
        let (tx, mut rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let conn = Connection::new(tx);

        let msg = JsonRpcMessage::Response(JsonRpcResponse::success(
            serde_json::json!({"answer": 42}),
            RequestId::Number(1),
        ));

        conn.send(msg.clone())
            .await
            .expect("send through Connection.tx should succeed");

        let received = rx
            .try_recv()
            .expect("rx should have the message sent via Connection.tx");

        let sent_json = msg.to_string().expect("serialize sent message");
        let recv_json = received.to_string().expect("serialize received message");
        assert_eq!(sent_json, recv_json);
    }

    #[tokio::test]
    async fn test_connection_send_fails_when_receiver_dropped() {
        let (tx, rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let conn = Connection::new(tx);

        // Drop the receiver
        drop(rx);

        let msg = JsonRpcMessage::Response(JsonRpcResponse::success(
            serde_json::json!({}),
            RequestId::Number(1),
        ));

        let result = conn.send(msg).await;
        assert!(result.is_err(), "send should fail when receiver is dropped");
    }

    #[tokio::test]
    async fn test_handle_connection_writer_delivers_to_stream() {
        use tokio::io::AsyncBufReadExt as _;

        let (server_side, client_side) = tokio::io::duplex(8192);

        let (tx, rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let conn = Arc::new(Connection::new(tx));

        let (router_tx, _router_rx) = mpsc::unbounded_channel();

        let conn_clone = conn.clone();
        tokio::spawn(async move {
            let _ = handle_connection(server_side, conn_clone, router_tx, rx).await;
        });

        let msg = JsonRpcMessage::Response(JsonRpcResponse::success(
            serde_json::json!({"delivered": true}),
            RequestId::Number(99),
        ));
        conn.send(msg.clone())
            .await
            .expect("send through Connection.tx");

        let reader = tokio::io::BufReader::new(client_side);
        let mut lines = reader.lines();
        let line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("should not time out waiting for writer output")
            .expect("IO should succeed")
            .expect("should get at least one line");

        let received: JsonRpcMessage =
            serde_json::from_str(&line).expect("writer output should be valid JSON-RPC");
        let expected_json = msg.to_string().expect("serialize expected");
        let received_json = received.to_string().expect("serialize received");
        assert_eq!(expected_json, received_json);
    }
}
