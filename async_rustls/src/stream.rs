//! Core TLS stream wrapper with async-io compatibility.
//!
//! This module provides `AsyncRustlsStream`, a wrapper around rustls streams
//! that implements the necessary traits for use with smol's `Async<T>`.

use rustls::{ClientConnection, ServerConnection};
use std::io::{self, Read, Write};
use std::net::TcpStream;

/// Trait for raw descriptor access, required by smol's Async wrapper.
///
/// On Unix systems, this requires `AsRawFd`. On Windows, `AsRawSocket`.
#[cfg(unix)]
pub trait AsRawDesc: std::os::unix::io::AsRawFd {}

#[cfg(windows)]
pub trait AsRawDesc: std::os::windows::io::AsRawSocket {}

/// Server-side TLS connection state.
pub type ServerState = rustls::StreamOwned<ServerConnection, TcpStream>;

/// Client-side TLS connection state.
pub type ClientState = rustls::StreamOwned<ClientConnection, TcpStream>;

/// Unified async TLS stream that wraps either a server or client connection.
///
/// This stream implements `Read`, `Write`, and the platform-specific raw
/// descriptor traits needed for use with `smol::Async<T>`.
#[derive(Debug)]
pub enum AsyncRustlsStream {
    /// Server-side TLS stream (accepts connections).
    Server(ServerState),
    /// Client-side TLS stream (initiates connections).
    Client(ClientState),
}

// SAFETY: The underlying TcpStream is safe for async-io operations.
// We are wrapping rustls streams which delegate I/O to TcpStream.
unsafe impl async_io::IoSafe for AsyncRustlsStream {}

impl AsyncRustlsStream {
    /// Create a new server-side TLS stream.
    pub fn new_server(stream: ServerState) -> Self {
        Self::Server(stream)
    }

    /// Create a new client-side TLS stream.
    pub fn new_client(stream: ClientState) -> Self {
        Self::Client(stream)
    }

    /// Get a reference to the underlying TCP stream.
    pub fn get_ref(&self) -> &TcpStream {
        match self {
            Self::Server(s) => s.get_ref(),
            Self::Client(s) => s.get_ref(),
        }
    }

    /// Get a mutable reference to the underlying TCP stream.
    pub fn get_mut(&mut self) -> &mut TcpStream {
        match self {
            Self::Server(s) => s.get_mut(),
            Self::Client(s) => s.get_mut(),
        }
    }

    /// Get the peer certificate if available (server-side only).
    pub fn peer_certificates(&self) -> Option<Vec<CertificateDer<'static>>> {
        match self {
            Self::Server(s) => s.conn.peer_certificates().map(|certs| {
                certs
                    .iter()
                    .map(|c| CertificateDer::from(c.as_ref().to_vec()))
                    .collect()
            }),
            Self::Client(s) => s.conn.peer_certificates().map(|certs| {
                certs
                    .iter()
                    .map(|c| CertificateDer::from(c.as_ref().to_vec()))
                    .collect()
            }),
        }
    }
}

use rustls::pki_types::CertificateDer;

impl Read for AsyncRustlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Server(s) => s.read(buf),
            Self::Client(s) => s.read(buf),
        }
    }
}

impl Write for AsyncRustlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Server(s) => s.write(buf),
            Self::Client(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Server(s) => s.flush(),
            Self::Client(s) => s.flush(),
        }
    }
}

// Platform-specific raw descriptor implementations.

#[cfg(unix)]
impl std::os::fd::AsFd for AsyncRustlsStream {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        std::os::fd::AsFd::as_fd(self.get_ref())
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for AsyncRustlsStream {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        std::os::unix::io::AsRawFd::as_raw_fd(self.get_ref())
    }
}

#[cfg(unix)]
impl AsRawDesc for AsyncRustlsStream {}

#[cfg(windows)]
impl std::os::windows::io::AsRawSocket for AsyncRustlsStream {
    fn as_raw_socket(&self) -> std::os::windows::io::RawSocket {
        std::os::windows::io::AsRawSocket::as_raw_socket(self.get_ref())
    }
}

#[cfg(windows)]
impl std::os::windows::io::AsSocket for AsyncRustlsStream {
    fn as_socket(&self) -> std::os::windows::io::BorrowedSocket<'_> {
        std::os::windows::io::AsSocket::as_socket(self.get_ref())
    }
}

#[cfg(windows)]
impl AsRawDesc for AsyncRustlsStream {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_rustls_stream_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AsyncRustlsStream>();
    }

    #[test]
    fn test_async_rustls_stream_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AsyncRustlsStream>();
    }
}
