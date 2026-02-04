//! Client-side TLS stream implementation.
//!
//! This module provides `TlsClientStream` and configuration builders for
//! initiating TLS connections.

use crate::stream::AsyncRustlsStream;
use crate::verifier::WezTermCertVerifier;
use anyhow::{anyhow, Context, Result};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

/// Client-side TLS stream wrapper.
///
/// This wraps a rustls `StreamOwned<ClientConnection, TcpStream>` and provides
/// a simpler interface for initiating TLS connections.
pub struct TlsClientStream {
    inner: AsyncRustlsStream,
}

impl TlsClientStream {
    /// Connect to a TLS server.
    ///
    /// This performs the TLS handshake synchronously and returns the wrapped stream.
    ///
    /// # Arguments
    ///
    /// * `tcp_stream` - The underlying TCP connection
    /// * `config` - TLS client configuration
    /// * `server_name` - The server name for SNI and certificate verification
    pub fn connect(
        tcp_stream: TcpStream,
        config: Arc<ClientConfig>,
        server_name: &str,
    ) -> Result<Self> {
        let server_name = ServerName::try_from(server_name.to_string())
            .map_err(|_| anyhow!("Invalid server name: {}", server_name))?;

        let conn = ClientConnection::new(config, server_name)
            .map_err(|e| anyhow!("Failed to create ClientConnection: {}", e))?;

        let mut stream = rustls::StreamOwned::new(conn, tcp_stream);

        // Drive the handshake to completion
        loop {
            match stream.conn.complete_io(&mut stream.sock) {
                Ok(_) => {
                    if !stream.conn.is_handshaking() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(anyhow!("TLS handshake failed: {}", e));
                }
            }
        }

        Ok(Self {
            inner: AsyncRustlsStream::new_client(stream),
        })
    }

    /// Get the server's certificate chain, if available.
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

impl Read for TlsClientStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for TlsClientStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl std::fmt::Debug for TlsClientStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsClientStream")
            .field("inner", &self.inner)
            .finish()
    }
}

/// Build a rustls ClientConfig for connecting to a mux server.
///
/// # Arguments
///
/// * `cert_pem` - Optional client certificate in PEM format (for mutual TLS)
/// * `key_pem` - Optional client private key in PEM format
/// * `ca_pem` - Optional CA certificate for server verification
/// * `additional_root_certs` - Additional root certificates to trust
///
/// # Returns
///
/// An `Arc<ClientConfig>` ready to use for connections.
pub fn build_client_config(
    cert_pem: Option<&[u8]>,
    key_pem: Option<&[u8]>,
    ca_pem: Option<&[u8]>,
    additional_root_certs: &[CertificateDer<'static>],
) -> Result<Arc<ClientConfig>> {
    let mut root_store = RootCertStore::empty();

    // Add system root certificates
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Add custom CA if provided
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

    // Add additional root certs
    for cert in additional_root_certs {
        root_store
            .add(cert.clone())
            .context("Failed to add additional root cert")?;
    }

    // Build client config
    let builder = ClientConfig::builder().with_root_certificates(root_store);

    let config = match (cert_pem, key_pem) {
        (Some(cert_bytes), Some(key_bytes)) => {
            // Mutual TLS - client provides certificate
            let certs: Vec<CertificateDer<'static>> =
                rustls_pemfile::certs(&mut std::io::Cursor::new(cert_bytes))
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to parse client certificate PEM")?;

            let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(key_bytes))
                .context("Failed to parse client key PEM")?
                .ok_or_else(|| anyhow!("No private key found in client key PEM"))?;

            builder
                .with_client_auth_cert(certs, key)
                .context("Failed to configure client auth")?
        }
        (None, None) => {
            // No client certificate
            builder.with_no_client_auth()
        }
        _ => {
            return Err(anyhow!(
                "Both client certificate and key must be provided together"
            ));
        }
    };

    // Set ALPN protocols
    let mut config = config;
    config.alpn_protocols = vec![b"wezterm-mux".to_vec()];

    Ok(Arc::new(config))
}

/// Build a client config with custom certificate verification.
///
/// This allows accepting self-signed certificates that match a specific CA.
pub fn build_client_config_with_custom_verifier(
    cert_pem: Option<&[u8]>,
    key_pem: Option<&[u8]>,
    ca_cert: CertificateDer<'static>,
    accept_invalid_hostnames: bool,
) -> Result<Arc<ClientConfig>> {
    let verifier = WezTermCertVerifier::new(ca_cert, accept_invalid_hostnames)?;

    let builder = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier));

    let config = match (cert_pem, key_pem) {
        (Some(cert_bytes), Some(key_bytes)) => {
            let certs: Vec<CertificateDer<'static>> =
                rustls_pemfile::certs(&mut std::io::Cursor::new(cert_bytes))
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to parse client certificate PEM")?;

            let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(key_bytes))
                .context("Failed to parse client key PEM")?
                .ok_or_else(|| anyhow!("No private key found in client key PEM"))?;

            builder
                .with_client_auth_cert(certs, key)
                .context("Failed to configure client auth")?
        }
        (None, None) => builder.with_no_client_auth(),
        _ => {
            return Err(anyhow!(
                "Both client certificate and key must be provided together"
            ));
        }
    };

    let mut config = config;
    config.alpn_protocols = vec![b"wezterm-mux".to_vec()];

    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_client_config_no_client_auth() {
        let config = build_client_config(None, None, None, &[]);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_client_config_with_ca() {
        // Generate a self-signed CA for testing using rcgen 0.12 API
        let ca = rcgen::generate_simple_self_signed(vec!["Test CA".into()]).unwrap();
        let ca_pem = ca.serialize_pem().unwrap();

        let config = build_client_config(None, None, Some(ca_pem.as_bytes()), &[]);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_client_config_mismatch_cert_key() {
        // Providing only cert without key should fail
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_pem = cert.serialize_pem().unwrap();

        let config = build_client_config(Some(cert_pem.as_bytes()), None, None, &[]);
        assert!(config.is_err());
    }
}
