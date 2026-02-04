//! Server-side TLS stream implementation.
//!
//! This module provides `TlsServerStream` and configuration builders for
//! accepting TLS connections.

use crate::stream::AsyncRustlsStream;
use crate::verifier::WezTermClientCertVerifier;
use anyhow::{anyhow, Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{RootCertStore, ServerConfig, ServerConnection};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

/// Server-side TLS stream wrapper.
///
/// This wraps a rustls `StreamOwned<ServerConnection, TcpStream>` and provides
/// a simpler interface for accepting TLS connections.
pub struct TlsServerStream {
    inner: AsyncRustlsStream,
}

impl TlsServerStream {
    /// Accept a TLS connection from an incoming TCP stream.
    ///
    /// This performs the TLS handshake synchronously and returns the wrapped stream.
    pub fn accept(tcp_stream: TcpStream, config: Arc<ServerConfig>) -> Result<Self> {
        let conn = ServerConnection::new(config)
            .map_err(|e| anyhow!("Failed to create ServerConnection: {}", e))?;

        let mut stream = rustls::StreamOwned::new(conn, tcp_stream);

        // Drive the handshake to completion
        // The handshake happens automatically on first read/write, but we can
        // force it to complete here for better error handling
        loop {
            match stream.conn.complete_io(&mut stream.sock) {
                Ok(_) => {
                    if !stream.conn.is_handshaking() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Non-blocking mode - just continue
                    continue;
                }
                Err(e) => {
                    return Err(anyhow!("TLS handshake failed: {}", e));
                }
            }
        }

        Ok(Self {
            inner: AsyncRustlsStream::new_server(stream),
        })
    }

    /// Get the peer's client certificates, if any were provided.
    pub fn peer_certificates(&self) -> Option<Vec<CertificateDer<'static>>> {
        self.inner.peer_certificates()
    }

    /// Get a reference to the underlying stream.
    pub fn get_ref(&self) -> &AsyncRustlsStream {
        &self.inner
    }

    /// Consume self and return the inner AsyncRustlsStream.
    pub fn into_inner(self) -> AsyncRustlsStream {
        self.inner
    }
}

impl Read for TlsServerStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for TlsServerStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl std::fmt::Debug for TlsServerStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsServerStream")
            .field("inner", &self.inner)
            .finish()
    }
}

/// Build a rustls ServerConfig for the mux server.
///
/// # Arguments
///
/// * `cert_pem` - Server certificate in PEM format
/// * `key_pem` - Server private key in PEM format
/// * `ca_pem` - Optional CA certificate for client verification
/// * `require_client_cert` - Whether to require mutual TLS
///
/// # Returns
///
/// An `Arc<ServerConfig>` ready to use for accepting connections.
pub fn build_server_config(
    cert_pem: &[u8],
    key_pem: &[u8],
    ca_pem: Option<&[u8]>,
    require_client_cert: bool,
) -> Result<Arc<ServerConfig>> {
    // Parse server certificate chain
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut std::io::Cursor::new(cert_pem))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse server certificate PEM")?;

    if certs.is_empty() {
        return Err(anyhow!("No certificates found in server cert PEM"));
    }

    // Parse private key
    let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(key_pem))
        .context("Failed to parse private key PEM")?
        .ok_or_else(|| anyhow!("No private key found in PEM file"))?;

    // Build server config
    let config = if require_client_cert {
        // Set up client certificate verification
        let mut root_store = RootCertStore::empty();

        if let Some(ca_bytes) = ca_pem {
            let ca_certs: Vec<CertificateDer<'static>> =
                rustls_pemfile::certs(&mut std::io::Cursor::new(ca_bytes))
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to parse CA certificate PEM")?;

            for cert in ca_certs {
                root_store
                    .add(cert)
                    .context("Failed to add CA cert to root store")?;
            }
        }

        // Create custom client verifier that also checks CN
        let verifier = WezTermClientCertVerifier::new(root_store)?;

        ServerConfig::builder()
            .with_client_cert_verifier(Arc::new(verifier))
            .with_single_cert(certs, key)
            .context("Failed to build server config with client auth")?
    } else {
        // No client certificate required
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to build server config")?
    };

    // Set ALPN protocols (optional, for protocol negotiation)
    let mut config = config;
    config.alpn_protocols = vec![b"wezterm-mux".to_vec()];

    Ok(Arc::new(config))
}

/// Load a certificate from a PEM file.
pub fn load_cert_from_file(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>> {
    let pem_bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read certificate file: {}", path.display()))?;

    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut std::io::Cursor::new(&pem_bytes))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("Failed to parse PEM from: {}", path.display()))?;

    Ok(certs)
}

/// Load a private key from a PEM file.
pub fn load_key_from_file(path: &std::path::Path) -> Result<PrivateKeyDer<'static>> {
    let pem_bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read key file: {}", path.display()))?;

    let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(&pem_bytes))
        .with_context(|| format!("Failed to parse key PEM from: {}", path.display()))?
        .ok_or_else(|| anyhow!("No private key found in: {}", path.display()))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_cert_from_bytes() {
        // Generate a self-signed cert for testing using rcgen 0.12 API
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_pem = cert.serialize_pem().unwrap();
        let key_pem = cert.serialize_private_key_pem();

        let config = build_server_config(cert_pem.as_bytes(), key_pem.as_bytes(), None, false);
        assert!(config.is_ok(), "Failed to build config: {:?}", config.err());
    }

    #[test]
    fn test_missing_cert_error() {
        let empty_pem = b"";
        let result = build_server_config(empty_pem, empty_pem, None, false);
        assert!(result.is_err());
    }
}
