//! SSH client handler for russh.
//!
//! This module implements the [`russh::client::Handler`] trait to handle
//! SSH protocol events like host key verification and authentication banners.
//!
//! ## Event Flow
//!
//! ```text
//! russh server message ──► check_server_key() ──► SessionEvent::HostVerify
//!                                                        │
//!                                                        ▼
//!                                                 WezTerm UI prompt
//!                                                        │
//!                                                        ▼
//!                                                  User response
//!                                                        │
//!                                                        ▼
//!                                              ◄─────────┘
//!                                              accept/reject
//! ```
//!
//! ## Host Key Verification
//!
//! When connecting to a server, the handler:
//! 1. Receives the server's public key
//! 2. Computes SHA256 fingerprint (base64 encoded)
//! 3. Sends verification request to WezTerm UI via event channel
//! 4. Waits for user approval/rejection
//! 5. Returns result to russh to continue or abort connection

use russh::keys::ssh_key::PublicKey;
use smol::channel::Sender;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::host::HostVerificationEvent;
use crate::session::SessionEvent;

/// Handler for russh client events.
///
/// This handler bridges russh's async event callbacks to wezterm-ssh's
/// event channel system.
pub struct WezTermHandler {
    /// Channel to send events back to the session manager
    event_tx: Sender<SessionEvent>,
    /// Whether the host key has been verified
    host_key_verified: Arc<AtomicBool>,
}

impl WezTermHandler {
    /// Create a new handler with the given event channel.
    pub fn new(event_tx: Sender<SessionEvent>) -> Self {
        Self {
            event_tx,
            host_key_verified: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the host key has been verified.
    #[allow(dead_code)]
    pub fn is_host_verified(&self) -> bool {
        self.host_key_verified.load(Ordering::SeqCst)
    }

    /// Format a public key fingerprint for display.
    fn format_key_fingerprint(key: &PublicKey) -> String {
        use sha2::{Digest, Sha256};

        // Get the key bytes, handling potential errors
        let key_bytes = match key.to_bytes() {
            Ok(bytes) => bytes,
            Err(_) => {
                return format!(
                    "{} <unable to compute fingerprint>",
                    key.algorithm().as_str()
                )
            }
        };

        let hash = Sha256::digest(&key_bytes);
        let fingerprint =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &hash[..]);

        format!(
            "{} SHA256:{}",
            key.algorithm().as_str(),
            fingerprint.trim_end_matches('=')
        )
    }
}

impl russh::client::Handler for WezTermHandler {
    type Error = anyhow::Error;

    /// Called when the server's host key needs to be verified.
    fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let fingerprint = Self::format_key_fingerprint(server_public_key);
        let event_tx = self.event_tx.clone();
        let host_key_verified = self.host_key_verified.clone();

        async move {
            // Send host verification event to the UI
            let (reply_tx, reply_rx) = smol::channel::bounded(1);
            event_tx
                .send(SessionEvent::HostVerify(HostVerificationEvent {
                    message: format!(
                        "The server's host key fingerprint is:\n{}\n\nDo you want to continue connecting?",
                        fingerprint
                    ),
                    reply: reply_tx,
                }))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send host verification event: {}", e))?;

            // Wait for user response
            let accepted = reply_rx.recv().await.map_err(|e| {
                anyhow::anyhow!("Failed to receive host verification response: {}", e)
            })?;

            host_key_verified.store(accepted, Ordering::SeqCst);
            Ok(accepted)
        }
    }

    /// Called when the server sends an authentication banner.
    fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut russh::client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let banner = banner.to_string();
        let event_tx = self.event_tx.clone();

        async move {
            let _ = event_tx.try_send(SessionEvent::Banner(Some(banner)));
            Ok(())
        }
    }
}
