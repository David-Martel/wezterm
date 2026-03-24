//! Named pipe server implementation for Windows
//!
//! Creates and manages the named pipe server for IPC communication.

use crate::connections::{handle_connection, Connection, ConnectionManager};
use crate::error::{DaemonError, Result};
use crate::protocol::JsonRpcMessage;
use crate::router::MessageRouter;
use std::sync::Arc;
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::sync::mpsc;
use tracing::{error, info, warn};


/// Named pipe server
pub struct NamedPipeServer {
    pipe_name: String,
    connection_manager: Arc<ConnectionManager>,
    router: Arc<MessageRouter>,
}

impl NamedPipeServer {
    pub fn new(
        pipe_name: impl Into<String>,
        max_connections: usize,
        version: String,
    ) -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(max_connections));
        let router = Arc::new(MessageRouter::new(
            connection_manager.clone(),
            version,
        ));

        Self {
            pipe_name: pipe_name.into(),
            connection_manager,
            router,
        }
    }

    /// Start the server and accept connections
    pub async fn run(&self) -> Result<()> {
        info!(pipe_name = %self.pipe_name, "Starting named pipe server");

        // Start router task
        let (router_tx, router_rx) = mpsc::unbounded_channel();
        let router = self.router.clone();
        tokio::spawn(async move {
            router.run(router_rx).await;
        });

        // Start connection cleanup task
        let cm = self.connection_manager.clone();
        cm.start_cleanup_task();

        // Accept connections in a loop
        loop {
            match self.create_server_instance().await {
                Ok(server) => {
                    let router_tx = router_tx.clone();
                    let cm = self.connection_manager.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::accept_connection(server, cm, router_tx).await {
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

    async fn create_server_instance(&self) -> Result<tokio::net::windows::named_pipe::NamedPipeServer> {
        let mut server_opts = ServerOptions::new();

        #[cfg(windows)]
        {
            server_opts.access_inbound(true)
                .access_outbound(true)
                .first_pipe_instance(false)
                .reject_remote_clients(true);
        }

        server_opts
            .create(&self.pipe_name)
            .map_err(|e| DaemonError::NamedPipe(format!("Failed to create pipe: {}", e)))
    }

    async fn accept_connection(
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

        // Create message channel for this connection
        let (tx, _rx) = mpsc::unbounded_channel();
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

        // Handle the connection
        let result = handle_connection(server, connection.clone(), router_tx).await;

        // Remove connection when done
        connection_manager.remove_connection(&connection_id);

        result
    }
}

/// Create a client connection to the named pipe (for testing)
#[cfg(windows)]
pub async fn connect_client(pipe_name: &str) -> Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    // Retry connection a few times
    for attempt in 1..=5 {
        match ClientOptions::new().open(pipe_name) {
            Ok(client) => {
                info!("Connected to pipe server");
                return Ok(client);
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

    Err(DaemonError::Connection("Failed to connect to pipe after retries".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = NamedPipeServer::new(
            r"\\.\pipe\wezterm-utils-test",
            10,
            "0.1.0".to_string(),
        );

        assert!(server.pipe_name.contains("wezterm-utils-test"));
    }

    // Additional integration tests would require spawning the server
    // and connecting clients, which is better suited for integration tests
}