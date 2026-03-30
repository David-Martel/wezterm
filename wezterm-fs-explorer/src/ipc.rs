//! Cross-platform Unix Domain Socket IPC implementation.
//!
//! This module provides a unified interface for Unix Domain Sockets on both
//! Windows and Unix platforms.
//!
//! # Platform Support
//!
//! - **Windows**: Uses `uds_windows` crate (Windows 10 build 17063+)
//! - **Unix/Linux/macOS**: Uses `tokio::net::UnixStream`
//!
//! # Example
//!
//! ```no_run
//! use wezterm_fs_explorer::ipc::{IpcServer, IpcClient};
//!
//! # async fn example() -> std::io::Result<()> {
//! // Server
//! let server = IpcServer::bind("/tmp/test.sock")?;
//! let stream = server.accept().await?;
//!
//! // Client
//! let client = IpcClient::connect("/tmp/test.sock").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Note: This module is designed as a library API. Some items may appear unused
//! when compiling the binary crate but are exported for external consumers.

// Allow unused items since this is a library module - items are exported for external use
#![allow(dead_code)]

use std::io::{Error as IoError, ErrorKind, Result};
use std::path::Path;
use tokio::io::{AsyncRead, AsyncWrite};

// ============================================================================
// Unix Implementation
// ============================================================================

#[cfg(unix)]
mod platform {
    use super::*;
    use tokio::net::{UnixListener, UnixStream};

    #[derive(Debug)]
    pub struct Listener {
        inner: UnixListener,
    }

    impl Listener {
        pub fn bind(path: &Path) -> Result<Self> {
            let inner = UnixListener::bind(path)?;
            Ok(Self { inner })
        }

        pub async fn accept(&self) -> Result<Stream> {
            let (stream, _) = self.inner.accept().await?;
            Ok(Stream { inner: stream })
        }
    }

    #[derive(Debug)]
    pub struct Stream {
        inner: UnixStream,
    }

    impl Stream {
        pub async fn connect(path: &Path) -> Result<Self> {
            let inner = UnixStream::connect(path).await?;
            Ok(Self { inner })
        }

        pub fn into_split(
            self,
        ) -> (
            tokio::io::ReadHalf<UnixStream>,
            tokio::io::WriteHalf<UnixStream>,
        ) {
            tokio::io::split(self.inner)
        }
    }

    impl AsyncRead for Stream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for Stream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize>> {
            std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
        }

        fn poll_flush(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(windows)]
mod platform {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uds_windows::{UnixListener as StdUnixListener, UnixStream as StdUnixStream};

    #[derive(Debug)]
    pub struct Listener {
        inner: Arc<StdUnixListener>,
    }

    impl Listener {
        pub fn bind(path: &Path) -> Result<Self> {
            let inner = StdUnixListener::bind(path)?;
            Ok(Self {
                inner: Arc::new(inner),
            })
        }

        pub async fn accept(&self) -> Result<Stream> {
            let listener = Arc::clone(&self.inner);
            // Use spawn_blocking for the synchronous accept operation
            let (stream, _) = tokio::task::spawn_blocking(move || listener.accept())
                .await
                .map_err(IoError::other)??;

            // Set non-blocking mode for async I/O
            stream.set_nonblocking(true)?;

            Ok(Stream {
                inner: Arc::new(Mutex::new(stream)),
            })
        }
    }

    #[derive(Debug)]
    pub struct Stream {
        inner: Arc<Mutex<StdUnixStream>>,
    }

    impl Stream {
        pub async fn connect(path: &Path) -> Result<Self> {
            let path = path.to_path_buf();
            // Use spawn_blocking for the synchronous connect operation
            let stream = tokio::task::spawn_blocking(move || StdUnixStream::connect(&path))
                .await
                .map_err(IoError::other)??;

            // Set non-blocking mode for async I/O
            stream.set_nonblocking(true)?;

            Ok(Self {
                inner: Arc::new(Mutex::new(stream)),
            })
        }

        pub fn into_split(self) -> (ReadHalf, WriteHalf) {
            (
                ReadHalf {
                    inner: Arc::clone(&self.inner),
                },
                WriteHalf {
                    inner: Arc::clone(&self.inner),
                },
            )
        }
    }

    // Simple read/write halves for Windows
    // PERFORMANCE: Uses try_lock with immediate wake on contention.
    // This is acceptable for IPC scenarios with low contention but may
    // cause busy-waiting under high concurrent load.
    #[derive(Debug)]
    pub struct ReadHalf {
        inner: Arc<Mutex<StdUnixStream>>,
    }

    #[derive(Debug)]
    pub struct WriteHalf {
        inner: Arc<Mutex<StdUnixStream>>,
    }

    impl AsyncRead for Stream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<Result<()>> {
            let inner = Arc::clone(&self.inner);
            let mut lock = match inner.try_lock() {
                Ok(lock) => lock,
                Err(_) => {
                    cx.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                }
            };

            match lock.read(buf.initialize_unfilled()) {
                Ok(n) => {
                    buf.advance(n);
                    std::task::Poll::Ready(Ok(()))
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
                Err(e) => std::task::Poll::Ready(Err(e)),
            }
        }
    }

    impl AsyncWrite for Stream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize>> {
            let inner = Arc::clone(&self.inner);
            let mut lock = match inner.try_lock() {
                Ok(lock) => lock,
                Err(_) => {
                    cx.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                }
            };

            match lock.write(buf) {
                Ok(n) => std::task::Poll::Ready(Ok(n)),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
                Err(e) => std::task::Poll::Ready(Err(e)),
            }
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<()>> {
            let inner = Arc::clone(&self.inner);
            let mut lock = match inner.try_lock() {
                Ok(lock) => lock,
                Err(_) => {
                    cx.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                }
            };

            match lock.flush() {
                Ok(()) => std::task::Poll::Ready(Ok(())),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
                Err(e) => std::task::Poll::Ready(Err(e)),
            }
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<()>> {
            // Windows UDS doesn't have a shutdown method in uds_windows 1.0
            std::task::Poll::Ready(Ok(()))
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// IPC server that listens for incoming connections on a Unix Domain Socket.
///
/// # Platform Support
///
/// - **Windows**: Requires Windows 10 build 17063+ (April 2018 Update)
/// - **Unix/Linux/macOS**: Standard Unix Domain Socket support
///
/// # Examples
///
/// ```no_run
/// use wezterm_fs_explorer::ipc::IpcServer;
///
/// # async fn example() -> std::io::Result<()> {
/// let server = IpcServer::bind("/tmp/wezterm-explorer.sock")?;
/// loop {
///     let stream = server.accept().await?;
///     // Handle connection...
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IpcServer {
    listener: platform::Listener,
}

impl IpcServer {
    /// Binds to the specified socket path and starts listening for connections.
    ///
    /// If a socket file already exists at the path, it will be removed before
    /// binding. This is safe because Unix Domain Sockets are automatically
    /// cleaned up when the server exits.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The socket path is invalid
    /// - The socket file cannot be removed
    /// - The socket cannot be bound
    /// - Permissions are insufficient
    pub fn bind(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Remove existing socket file if it exists
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = platform::Listener::bind(path)?;
        Ok(Self { listener })
    }

    /// Accepts a new incoming connection.
    ///
    /// This method will block until a connection is received.
    ///
    /// # Errors
    ///
    /// Returns an error if the accept operation fails.
    pub async fn accept(&self) -> Result<IpcStream> {
        let stream = self.listener.accept().await?;
        Ok(IpcStream { stream })
    }
}

/// IPC client for connecting to a Unix Domain Socket server.
///
/// # Examples
///
/// ```no_run
/// use wezterm_fs_explorer::ipc::IpcClient;
/// use tokio::io::AsyncWriteExt;
///
/// # async fn example() -> std::io::Result<()> {
/// let mut stream = IpcClient::connect("/tmp/wezterm-explorer.sock").await?;
/// stream.write_all(b"Hello, server!").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IpcClient;

impl IpcClient {
    /// Connects to the IPC server at the specified socket path.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The socket path does not exist
    /// - The server is not listening
    /// - Connection is refused
    /// - Permissions are insufficient
    pub async fn connect(path: impl AsRef<Path>) -> Result<IpcStream> {
        let stream = platform::Stream::connect(path.as_ref()).await?;
        Ok(IpcStream { stream })
    }

    /// Attempts to connect with a timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails or times out.
    pub async fn connect_timeout(
        path: impl AsRef<Path>,
        timeout: std::time::Duration,
    ) -> Result<IpcStream> {
        tokio::time::timeout(timeout, Self::connect(path))
            .await
            .map_err(|_| IoError::new(ErrorKind::TimedOut, "Connection timed out"))?
    }
}

/// A bidirectional stream for IPC communication.
///
/// This wraps the platform-specific Unix Domain Socket stream and provides
/// a unified interface. It implements `AsyncRead` and `AsyncWrite`.
///
/// # Examples
///
/// ```no_run
/// use wezterm_fs_explorer::ipc::IpcClient;
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// # async fn example() -> std::io::Result<()> {
/// let mut stream = IpcClient::connect("/tmp/test.sock").await?;
/// stream.write_all(b"Hello").await?;
///
/// let mut buffer = [0u8; 1024];
/// let n = stream.read(&mut buffer).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IpcStream {
    stream: platform::Stream,
}

impl AsyncRead for IpcStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for IpcStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize>> {
        std::pin::Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_ipc_server_client() {
        let socket_path = if cfg!(windows) {
            r"C:\temp\wezterm-test-socket"
        } else {
            "/tmp/wezterm-test-socket"
        };

        // Clean up any existing socket
        let _ = std::fs::remove_file(socket_path);

        // Create server
        let server = IpcServer::bind(socket_path).expect("Failed to bind server");

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let mut stream = server.accept().await.expect("Failed to accept connection");
            let mut buffer = [0u8; 5];
            stream
                .read_exact(&mut buffer)
                .await
                .expect("Failed to read");
            assert_eq!(&buffer, b"hello");
            stream.write_all(b"world").await.expect("Failed to write");
        });

        // Give server time to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create client
        let mut client = IpcClient::connect(socket_path)
            .await
            .expect("Failed to connect client");

        // Send message
        client.write_all(b"hello").await.expect("Failed to write");

        // Receive response
        let mut buffer = [0u8; 5];
        client
            .read_exact(&mut buffer)
            .await
            .expect("Failed to read");
        assert_eq!(&buffer, b"world");

        // Wait for server to finish
        server_task.await.expect("Server task failed");

        // Clean up
        let _ = std::fs::remove_file(socket_path);
    }
}
