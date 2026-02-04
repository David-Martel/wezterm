//! # Async Rustls
//!
//! Pure-Rust async TLS using rustls with smol/async-io compatibility.
//!
//! This crate provides TLS stream wrappers that integrate with the smol async
//! runtime, replacing the OpenSSL-based `async_ossl` crate with a pure-Rust
//! implementation.
//!
//! ## Features
//!
//! - **Pure Rust**: No C library dependencies (no OpenSSL, no vcpkg)
//! - **Async-compatible**: Works with smol's `Async<T>` wrapper
//! - **Platform-agnostic**: Works on Windows, macOS, and Linux
//!
//! ## Server Example
//!
//! ```rust,no_run
//! use async_rustls::{TlsServerStream, build_server_config};
//! use std::net::TcpStream;
//!
//! # fn main() -> anyhow::Result<()> {
//! let cert_pem = std::fs::read("server.pem")?;
//! let key_pem = std::fs::read("server.key")?;
//! let config = build_server_config(&cert_pem, &key_pem, None, true)?;
//!
//! // Accept TCP connection, then upgrade to TLS
//! // let tcp_stream: TcpStream = listener.accept()?.0;
//! // let tls_stream = TlsServerStream::accept(tcp_stream, config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Client Example
//!
//! ```rust,no_run
//! use async_rustls::{TlsClientStream, build_client_config};
//! use std::net::TcpStream;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = build_client_config(None, None, None, &[])?;
//!
//! // Connect TCP, then upgrade to TLS
//! // let tcp_stream = TcpStream::connect("example.com:443")?;
//! // let tls_stream = TlsClientStream::connect(tcp_stream, config, "example.com")?;
//! # Ok(())
//! # }
//! ```

mod client;
mod server;
mod stream;
mod verifier;

pub use client::{build_client_config, build_client_config_with_custom_verifier, TlsClientStream};
pub use server::{build_server_config, load_cert_from_file, load_key_from_file, TlsServerStream};
pub use stream::{AsRawDesc, AsyncRustlsStream};
pub use verifier::{extract_cn, WezTermCertVerifier, WezTermClientCertVerifier};

// Re-export commonly used types from rustls
pub use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
pub use rustls::{ClientConfig, ServerConfig};
