//! Russh session wrapper for connection management.
//!
//! This module provides the RusshSession type that wraps russh's async
//! connection handling and bridges it to wezterm-ssh's synchronous API.

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::ssh_key::PrivateKey;
use smol::channel::Sender;
use tokio::net::TcpStream;

use super::channel::RusshChannel;
use super::handler::WezTermHandler;
use crate::session::SessionEvent;

/// Configuration for russh client connections.
fn create_client_config() -> Arc<client::Config> {
    Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(300)),
        keepalive_interval: Some(Duration::from_secs(60)),
        keepalive_max: 3,
        ..Default::default()
    })
}

/// Wrapper around a russh client session.
///
/// This type manages the underlying russh connection and provides
/// methods for authentication and channel operations.
pub struct RusshSession {
    /// The russh client handle
    handle: Handle<WezTermHandler>,
    /// Client configuration
    #[allow(dead_code)]
    config: Arc<client::Config>,
}

impl RusshSession {
    /// Connect to an SSH server.
    ///
    /// This establishes a TCP connection and performs the SSH handshake.
    /// Host key verification is handled via the event channel.
    pub async fn connect(
        host: &str,
        port: u16,
        event_tx: Sender<SessionEvent>,
    ) -> anyhow::Result<Self> {
        let config = create_client_config();
        let handler = WezTermHandler::new(event_tx);

        // Resolve hostname to socket address
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve hostname: {}", host))?;

        // Connect using tokio's TcpStream
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;

        // Perform SSH handshake
        let handle = client::connect_stream(config.clone(), stream, handler).await?;

        Ok(Self { handle, config })
    }

    /// Authenticate using password.
    pub async fn authenticate_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<bool> {
        let result = self
            .handle
            .authenticate_password(username, password)
            .await?;
        Ok(result.success())
    }

    /// Authenticate using a public key.
    pub async fn authenticate_publickey(
        &mut self,
        username: &str,
        key: Arc<PrivateKey>,
    ) -> anyhow::Result<bool> {
        // Get the best supported RSA hash algorithm (for RSA keys)
        let hash = self.handle.best_supported_rsa_hash().await?.flatten();

        let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(key, hash);
        let result = self
            .handle
            .authenticate_publickey(username, key_with_hash)
            .await?;
        Ok(result.success())
    }

    /// Open a new session channel.
    ///
    /// This creates a new channel that can be used for PTY operations,
    /// shell execution, or command execution.
    pub async fn open_channel(&self) -> anyhow::Result<RusshChannel> {
        let channel = self.handle.channel_open_session().await?;
        Ok(RusshChannel::new(channel))
    }

    /// Disconnect the session.
    pub async fn disconnect(
        &self,
        reason: russh::Disconnect,
        description: &str,
        language_tag: &str,
    ) -> anyhow::Result<()> {
        self.handle
            .disconnect(reason, description, language_tag)
            .await?;
        Ok(())
    }
}

impl std::fmt::Debug for RusshSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RusshSession").finish()
    }
}
