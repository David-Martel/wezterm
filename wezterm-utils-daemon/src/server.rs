//! Named pipe server implementation for Windows
//!
//! Creates and manages the named pipe server for IPC communication.

use crate::connections::{handle_connection, Connection, ConnectionManager};
use crate::error::{DaemonError, Result};
use crate::protocol::JsonRpcMessage;
use crate::router::MessageRouter;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// IPC Server (Named Pipes on Windows, Unix Domain Sockets on Unix)
pub struct IpcServer {
    path: String,
    connection_manager: Arc<ConnectionManager>,
    router: Arc<MessageRouter>,
}

impl IpcServer {
    pub fn new(path: impl Into<String>, max_connections: usize, version: String) -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(max_connections));
        let router = Arc::new(MessageRouter::new(connection_manager.clone(), version));

        Self {
            path: path.into(),
            connection_manager,
            router,
        }
    }

    /// Start the server and accept connections
    pub async fn run(&self) -> Result<()> {
        info!(path = %self.path, "Starting IPC server");

        // Start router task
        let (router_tx, router_rx) = mpsc::unbounded_channel();
        let router = self.router.clone();
        tokio::spawn(async move {
            router.run(router_rx).await;
        });

        // Start connection cleanup task
        let cm = self.connection_manager.clone();
        cm.start_cleanup_task();

        #[cfg(windows)]
        {
            // Accept connections in a loop
            loop {
                match self.create_named_pipe_instance().await {
                    Ok(server) => {
                        let router_tx = router_tx.clone();
                        let cm = self.connection_manager.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                Self::accept_named_pipe_connection(server, cm, router_tx).await
                            {
                                error!(error = %e, "Connection handling error");
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to create server instance");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            // Remove existing socket if it exists
            let _ = std::fs::remove_file(&self.path);

            let listener = UnixListener::bind(&self.path).map_err(|e| {
                DaemonError::Connection(format!("Failed to bind to {}: {}", self.path, e))
            })?;

            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let router_tx = router_tx.clone();
                        let cm = self.connection_manager.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::accept_connection(stream, cm, router_tx).await {
                                error!(error = %e, "Connection handling error");
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "Accept error");
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    async fn create_named_pipe_instance(
        &self,
    ) -> Result<tokio::net::windows::named_pipe::NamedPipeServer> {
        let mut server_opts = ServerOptions::new();

        server_opts
            .access_inbound(true)
            .access_outbound(true)
            .first_pipe_instance(false)
            .reject_remote_clients(true);

        server_opts
            .create(&self.path)
            .map_err(|e| DaemonError::NamedPipe(format!("Failed to create pipe: {}", e)))
    }

    #[cfg(windows)]
    async fn accept_named_pipe_connection(
        server: tokio::net::windows::named_pipe::NamedPipeServer,
        connection_manager: Arc<ConnectionManager>,
        router_tx: mpsc::UnboundedSender<(String, JsonRpcMessage)>,
    ) -> Result<()> {
        // Wait for client to connect
        server
            .connect()
            .await
            .map_err(|e| DaemonError::Connection(format!("Failed to accept connection: {}", e)))?;

        info!("Client connected to pipe");
        Self::accept_connection(server, connection_manager, router_tx).await
    }

    async fn accept_connection<S>(
        stream: S,
        connection_manager: Arc<ConnectionManager>,
        router_tx: mpsc::UnboundedSender<(String, JsonRpcMessage)>,
    ) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        // Create message channel for this connection.
        // tx goes into Connection (for sending responses/events to the client).
        // rx goes to handle_connection (wired to the writer task).
        let (tx, rx) = mpsc::unbounded_channel();
        let connection = Connection::new(tx);
        let connection_id = connection.id.clone();

        // Add to connection manager
        let connection = match connection_manager.add_connection(connection) {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "Failed to add connection");
                return Err(e);
            }
        };

        // Handle the connection — pass rx so the writer task can send responses
        let result = handle_connection(stream, connection.clone(), router_tx, rx).await;

        // Remove connection when done
        connection_manager.remove_connection(&connection_id);

        result
    }
}

/// Create a client connection to the IPC server (for testing)
pub async fn connect_client(path: &str) -> Result<Box<dyn crate::connections::Stream>> {
    #[cfg(windows)]
    {
        // Retry connection a few times
        for attempt in 1..=5 {
            match ClientOptions::new().open(path) {
                Ok(client) => {
                    info!("Connected to pipe server");
                    return Ok(Box::new(client));
                }
                Err(e) => {
                    warn!(
                        attempt = attempt,
                        error = %e,
                        "Failed to connect to pipe"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let stream = tokio::net::UnixStream::connect(path).await.map_err(|e| {
            DaemonError::Connection(format!("Failed to connect to {}: {}", path, e))
        })?;
        return Ok(Box::new(stream));
    }

    #[cfg(windows)]
    {
        Err(DaemonError::Connection(
            "Failed to connect to pipe after retries".to_string(),
        ))
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err(DaemonError::Connection("Unsupported platform".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = IpcServer::new("wezterm-utils-test", 10, "0.1.0".to_string());

        assert!(server.path.contains("wezterm-utils-test"));
    }

    // Additional integration tests would require spawning the server
    // and connecting clients, which is better suited for integration tests
}
