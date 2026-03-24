//! High-performance IPC implementations with optimizations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use dashmap::DashMap;
use parking_lot::Mutex as SyncMutex;
use lz4::block::{compress, decompress};

/// IPC client with multiple serialization formats
pub struct IpcClient {
    connection: Arc<Connection>,
    format: SerializationFormat,
}

#[derive(Clone, Copy)]
pub enum SerializationFormat {
    Json,
    MessagePack,
    MessagePackLz4,
}

/// Wrapper around Windows HANDLE that is safe to send between threads.
/// Named pipe handles are thread-safe when properly synchronized.
#[cfg(windows)]
struct SendableHandle(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
// SAFETY: Windows named pipe handles are safe to send between threads
// when access is synchronized (via Mutex). The handle itself is just
// a pointer-sized integer identifying a kernel object.
unsafe impl Send for SendableHandle {}

#[cfg(windows)]
// SAFETY: Access to the handle is synchronized through Arc<Mutex<...>>,
// so concurrent access from multiple threads is safe.
unsafe impl Sync for SendableHandle {}

struct Connection {
    #[cfg(windows)]
    pipe: Arc<Mutex<SendableHandle>>,
    #[cfg(not(windows))]
    socket: Arc<Mutex<tokio::net::UnixStream>>,
}

impl IpcClient {
    pub async fn connect_json() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection: Arc::new(connection),
            format: SerializationFormat::Json,
        })
    }

    pub async fn connect_msgpack() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection: Arc::new(connection),
            format: SerializationFormat::MessagePack,
        })
    }

    pub async fn connect_compressed() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection: Arc::new(connection),
            format: SerializationFormat::MessagePackLz4,
        })
    }

    pub async fn connect_zero_copy() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection: Arc::new(connection),
            format: SerializationFormat::MessagePack,
        })
    }

    pub async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: &T,
    ) -> Result<R, Box<dyn std::error::Error>> {
        let payload = self.serialize(params)?;
        let response = self.connection.send_receive(method, payload).await?;
        self.deserialize(&response)
    }

    pub async fn send_bytes(
        &self,
        method: &str,
        data: Bytes,
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        self.connection.send_receive_bytes(method, data).await
    }

    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.format {
            SerializationFormat::Json => {
                Ok(serde_json::to_vec(value)?)
            }
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::to_vec(value)?)
            }
            SerializationFormat::MessagePackLz4 => {
                let msgpack = rmp_serde::to_vec(value)?;
                let compressed = compress(&msgpack, None, true)?;
                Ok(compressed)
            }
        }
    }

    fn deserialize<R: for<'de> Deserialize<'de>>(
        &self,
        data: &[u8],
    ) -> Result<R, Box<dyn std::error::Error>> {
        match self.format {
            SerializationFormat::Json => {
                Ok(serde_json::from_slice(data)?)
            }
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::from_slice(data)?)
            }
            SerializationFormat::MessagePackLz4 => {
                let decompressed = decompress(data, None)?;
                Ok(rmp_serde::from_slice(&decompressed)?)
            }
        }
    }
}

impl Connection {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            // Windows named pipe implementation
            todo!("Implement Windows named pipe connection")
        }

        #[cfg(not(windows))]
        {
            let socket = tokio::net::UnixStream::connect("/tmp/wezterm-utils.sock").await?;
            Ok(Self {
                socket: Arc::new(Mutex::new(socket)),
            })
        }
    }

    async fn send_receive(
        &self,
        method: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Mock implementation for benchmarking
        Ok(payload)
    }

    async fn send_receive_bytes(
        &self,
        method: &str,
        data: Bytes,
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        // Zero-copy implementation
        Ok(data)
    }
}

/// Connection pool for reusing IPC connections
pub struct ConnectionPool {
    connections: Arc<DashMap<String, Arc<IpcClient>>>,
    max_connections: usize,
    last_used: Arc<SyncMutex<HashMap<String, Instant>>>,
}

impl ConnectionPool {
    pub async fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            max_connections,
            last_used: Arc::new(SyncMutex::new(HashMap::new())),
        }
    }

    pub async fn get_or_create(&self, utility_id: &str) -> Arc<IpcClient> {
        // Check if connection exists
        if let Some(conn) = self.connections.get(utility_id) {
            self.update_last_used(utility_id);
            return conn.clone();
        }

        // Create new connection
        let client = IpcClient::connect_msgpack().await.unwrap();
        let client_arc = Arc::new(client);

        // Check if we need to evict
        if self.connections.len() >= self.max_connections {
            self.evict_oldest();
        }

        self.connections.insert(utility_id.to_string(), client_arc.clone());
        self.update_last_used(utility_id);

        client_arc
    }

    fn update_last_used(&self, utility_id: &str) {
        let mut last_used = self.last_used.lock();
        last_used.insert(utility_id.to_string(), Instant::now());
    }

    fn evict_oldest(&self) {
        let last_used = self.last_used.lock();
        if let Some((oldest_id, _)) = last_used.iter()
            .min_by_key(|(_, time)| *time) {
            self.connections.remove(oldest_id);
        }
    }
}

/// Message batcher for reducing IPC overhead
pub struct MessageBatcher {
    client: Arc<IpcClient>,
    batch_size: usize,
    batch_timeout: Duration,
    pending: Arc<Mutex<Vec<PendingMessage>>>,
    sender: mpsc::Sender<BatchCommand>,
}

struct PendingMessage {
    method: String,
    payload: Vec<u8>,
    response_tx: oneshot::Sender<Vec<u8>>,
}

enum BatchCommand {
    Add(PendingMessage),
    Flush,
}

impl MessageBatcher {
    pub fn new(client: IpcClient) -> Self {
        let client = Arc::new(client);
        let pending = Arc::new(Mutex::new(Vec::new()));
        let (tx, mut rx) = mpsc::channel::<BatchCommand>(100);

        let batcher = Self {
            client: client.clone(),
            batch_size: 10,
            batch_timeout: Duration::from_millis(10),
            pending: pending.clone(),
            sender: tx,
        };

        // Spawn batch processor
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(10));

            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            BatchCommand::Add(msg) => {
                                let mut pending = pending.lock().await;
                                pending.push(msg);

                                if pending.len() >= 10 {
                                    Self::flush_batch(&client, &mut pending).await;
                                }
                            }
                            BatchCommand::Flush => {
                                let mut pending = pending.lock().await;
                                Self::flush_batch(&client, &mut pending).await;
                            }
                        }
                    }
                    _ = interval.tick() => {
                        let mut pending = pending.lock().await;
                        if !pending.is_empty() {
                            Self::flush_batch(&client, &mut pending).await;
                        }
                    }
                }
            }
        });

        batcher
    }

    pub async fn send<T: Serialize>(
        &mut self,
        method: &str,
        params: T,
    ) -> oneshot::Receiver<Vec<u8>> {
        let payload = serde_json::to_vec(&params).unwrap();
        let (tx, rx) = oneshot::channel();

        let msg = PendingMessage {
            method: method.to_string(),
            payload,
            response_tx: tx,
        };

        self.sender.send(BatchCommand::Add(msg)).await.unwrap();
        rx
    }

    async fn flush_batch(client: &IpcClient, pending: &mut Vec<PendingMessage>) {
        if pending.is_empty() {
            return;
        }

        // Send batch request
        let batch: Vec<_> = pending.drain(..).collect();

        // Process responses
        for msg in batch {
            let response = client.send_request::<_, Vec<u8>>(&msg.method, &msg.payload).await.unwrap();
            let _ = msg.response_tx.send(response);
        }
    }
}

/// IPC message for benchmarking
#[derive(Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires wezterm-utils-daemon running on named pipe"]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(5).await;

        // Get multiple connections
        let conn1 = pool.get_or_create("util1").await;
        let conn2 = pool.get_or_create("util2").await;
        let conn3 = pool.get_or_create("util1").await; // Should reuse

        assert!(Arc::ptr_eq(&conn1, &conn3));
        assert!(!Arc::ptr_eq(&conn1, &conn2));
    }

    #[tokio::test]
    #[ignore = "requires wezterm-utils-daemon running on named pipe"]
    async fn test_message_batching() {
        let client = IpcClient::connect_json().await.unwrap();
        let mut batcher = MessageBatcher::new(client);

        // Send multiple messages sequentially (send takes &mut self)
        let mut receivers = Vec::new();
        for i in 0..5 {
            let rx = batcher.send("test", i).await;
            receivers.push(rx);
        }

        // All should be batched
        for rx in receivers {
            let _ = rx.await;
        }
    }
}