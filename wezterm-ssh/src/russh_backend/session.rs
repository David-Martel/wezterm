//! SSH session management using russh.
//!
//! This module provides [`RusshSession`], which manages the SSH connection
//! lifecycle including authentication and channel creation.
//!
//! ## Connection Flow
//!
//! ```text
//! 1. TCP connect     ──► TcpStream::connect(host:port)
//! 2. SSH handshake   ──► client::connect_stream() with nodelay
//! 3. Host verify     ──► Handler.check_server_key() → UI prompt
//! 4. Authenticate    ──► password or publickey
//! 5. Open channels   ──► session/SFTP channels
//! ```
//!
//! ## Authentication Methods
//!
//! | Method | Function | Notes |
//! |--------|----------|-------|
//! | Password | [`authenticate_password`](RusshSession::authenticate_password) | Plain password auth |
//! | Public Key | [`authenticate_publickey`](RusshSession::authenticate_publickey) | RSA, Ed25519, ECDSA |
//!
//! ## Configuration
//!
//! Default client configuration:
//! - **Inactivity timeout**: 300 seconds
//! - **Keepalive interval**: 60 seconds
//! - **Keepalive max**: 3 missed before disconnect
//! - **TCP nodelay**: Enabled for low latency

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::ssh_key::PrivateKey;
use smol::channel::Sender;
use tokio::net::TcpStream;

use super::channel::RusshChannel;
use super::handler::WezTermHandler;
use super::sftp::RusshSftp;
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

/// SSH session wrapper for russh.
///
/// Manages the SSH connection lifecycle and provides methods for:
/// - Authentication (password, public key)
/// - Channel creation (PTY, exec, SFTP)
/// - Session disconnection
///
/// ## Example
///
/// ```ignore
/// // Connect to server
/// let session = RusshSession::connect("example.com", 22, event_tx).await?;
///
/// // Authenticate
/// session.authenticate_password("user", "password").await?;
///
/// // Open channel for shell
/// let mut channel = session.open_channel().await?;
/// channel.request_pty("xterm-256color", size).await?;
/// channel.request_shell().await?;
/// ```
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

    /// Open an SFTP channel.
    ///
    /// This creates a new channel with the SFTP subsystem for file operations.
    pub async fn open_sftp(&self) -> anyhow::Result<RusshSftp> {
        let channel = self.handle.channel_open_session().await?;
        RusshSftp::new(channel).await
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
