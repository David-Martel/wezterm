//! Pure-Rust TLS server listener using rustls.
//!
//! This module replaces ossl.rs and eliminates the OpenSSL dependency.

use anyhow::{anyhow, Context, Error};
use async_rustls::{extract_cn, TlsServerStream, WezTermClientCertVerifier};
use config::TlsDomainServer;
use promise::spawn::spawn_into_main_thread;
use rustls::pki_types::CertificateDer;
use rustls::{RootCertStore, ServerConfig};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use wezterm_mux_server_impl::PKI;

struct RustlsNetListener {
    config: Arc<ServerConfig>,
    listener: TcpListener,
}

impl RustlsNetListener {
    pub fn new(listener: TcpListener, config: Arc<ServerConfig>) -> Self {
        Self { listener, config }
    }

    /// Authenticates the peer by verifying the certificate CN.
    ///
    /// The requirements are:
    /// * The peer must have a certificate
    /// * The peer certificate must be trusted
    /// * The peer certificate must include a CN string that is
    ///   either an exact match for the unix username of the
    ///   user running this mux server instance, or must match
    ///   a special encoded prefix set up by a proprietary PKI
    ///   infrastructure in an environment used by the author.
    fn verify_peer_cert(stream: &TlsServerStream) -> anyhow::Result<()> {
        let certs = stream
            .peer_certificates()
            .ok_or_else(|| anyhow!("no peer cert"))?;

        if certs.is_empty() {
            anyhow::bail!("no peer certificates provided");
        }

        let cert = &certs[0];
        let cn_str = extract_cn(cert)?;

        #[cfg(unix)]
        let wanted_unix_name = std::env::var("USER")?;
        #[cfg(windows)]
        let wanted_unix_name = std::env::var("USERNAME")?;

        if wanted_unix_name == cn_str {
            log::trace!(
                "Peer certificate CN `{}` == $USER `{}`",
                cn_str,
                wanted_unix_name
            );
            Ok(())
        } else {
            // Some environments that are used by the author of this
            // program encode the CN in the form `user:unixname/DATA`
            let maybe_encoded = format!("user:{}/", wanted_unix_name);
            if cn_str.starts_with(&maybe_encoded) {
                log::trace!(
                    "Peer certificate CN `{}` matches $USER `{}`",
                    cn_str,
                    wanted_unix_name
                );
                Ok(())
            } else {
                anyhow::bail!("CN `{}` did not match $USER `{}`", cn_str, wanted_unix_name);
            }
        }
    }

    fn run(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    stream.set_nodelay(true).ok();
                    let config = self.config.clone();

                    match TlsServerStream::accept(stream, config) {
                        Ok(tls_stream) => {
                            if let Err(err) = Self::verify_peer_cert(&tls_stream) {
                                log::error!("problem with peer cert: {}", err);
                                continue;
                            }
                            spawn_into_main_thread(async move {
                                log::info!("Accepted new TLS connection");
                                wezterm_mux_server_impl::dispatch::process(tls_stream.into_inner())
                                    .await
                                    .map_err(|e| {
                                        log::error!("process: {:?}", e);
                                        e
                                    })
                            })
                            .detach();
                        }
                        Err(e) => {
                            log::error!("TLS accept failed: {}", e);
                        }
                    }
                }
                Err(err) => {
                    log::error!("accept failed: {}", err);
                    return;
                }
            }
        }
    }
}

/// Load a certificate from a PEM file.
fn load_cert(name: &Path) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let cert_bytes = std::fs::read(name)?;
    log::trace!("loaded {}", name.display());

    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut std::io::Cursor::new(&cert_bytes))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("Failed to parse certificate from {}", name.display()))?;

    Ok(certs)
}

/// Spawn the TLS listener for the mux server.
///
/// This function sets up a rustls-based TLS server that:
/// 1. Requires client certificates (mutual TLS)
/// 2. Verifies client certificate CN matches the Unix username
/// 3. Dispatches authenticated connections to the mux protocol handler
pub fn spawn_tls_listener(tls_server: &TlsDomainServer) -> Result<(), Error> {
    // Build root certificate store for client verification
    let mut root_store = RootCertStore::empty();

    // Add configured root certs
    for name in &tls_server.pem_root_certs {
        if name.is_dir() {
            for entry in std::fs::read_dir(name)? {
                if let Ok(certs) = load_cert(&entry?.path()) {
                    for cert in certs {
                        root_store.add(cert).ok();
                    }
                }
            }
        } else {
            for cert in load_cert(name)? {
                root_store.add(cert)?;
            }
        }
    }

    // Add PKI CA certificate
    for cert in load_cert(&PKI.ca_pem())? {
        root_store.add(cert)?;
    }

    // Load server certificate
    let cert_file = tls_server
        .pem_cert
        .clone()
        .unwrap_or_else(|| PKI.server_pem());
    let cert_pem = std::fs::read(&cert_file).context(format!(
        "Failed to read server certificate from {}",
        cert_file.display()
    ))?;

    // Load certificate chain if provided
    let mut full_cert_pem = cert_pem;
    if let Some(chain_file) = tls_server.pem_ca.as_ref() {
        let chain_pem = std::fs::read(chain_file).context(format!(
            "Failed to read certificate chain from {}",
            chain_file.display()
        ))?;
        full_cert_pem.extend(b"\n");
        full_cert_pem.extend(chain_pem);
    }

    // Load private key
    let key_file = tls_server
        .pem_private_key
        .clone()
        .unwrap_or_else(|| PKI.server_pem());
    let key_pem = std::fs::read(&key_file).context(format!(
        "Failed to read private key from {}",
        key_file.display()
    ))?;

    // Build server config with client certificate verification
    let config = build_server_config_with_client_verifier(&full_cert_pem, &key_pem, root_store)?;

    log::info!("listening with TLS on {:?}", tls_server.bind_address);

    let mut net_listener = RustlsNetListener::new(
        TcpListener::bind(&tls_server.bind_address).with_context(|| {
            format!(
                "error binding to mux_server_bind_address {}",
                tls_server.bind_address,
            )
        })?,
        config,
    );

    std::thread::spawn(move || {
        net_listener.run();
    });

    Ok(())
}

/// Build a ServerConfig with WezTerm's custom client certificate verifier.
fn build_server_config_with_client_verifier(
    cert_pem: &[u8],
    key_pem: &[u8],
    root_store: RootCertStore,
) -> anyhow::Result<Arc<ServerConfig>> {
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

    // Create custom client verifier
    let verifier = WezTermClientCertVerifier::new(root_store)?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(verifier))
        .with_single_cert(certs, key)
        .context("Failed to build server config with client auth")?;

    // Set ALPN protocols
    let mut config = config;
    config.alpn_protocols = vec![b"wezterm-mux".to_vec()];

    Ok(Arc::new(config))
}
