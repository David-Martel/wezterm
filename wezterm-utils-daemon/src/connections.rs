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

/// Connection keep-alive interval
#[expect(dead_code, reason = "reserved for keep-alive heartbeat implementation")]
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(30);

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
    #[expect(dead_code, reason = "reserved for connection age metrics")]
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

    /// Start periodic cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.cleanup_stale_connections();
            }
        });
    }
}

/// Handle a single connection (reads messages from pipe/socket).
///
/// `outbound_rx` receives messages destined for this connection's client
/// (e.g., responses, broadcast events). It should be the receiver half
/// of the channel whose sender is stored in `Connection.tx`.
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
}
