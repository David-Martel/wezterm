//! # Pure-Rust SSH Backend
//!
//! This module provides a pure-Rust SSH implementation using the [`russh`] crate,
//! eliminating all C library dependencies (libssh, libssh2, OpenSSL).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    wezterm-ssh (sync API)                    │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │ SessionWrap │  │ ChannelWrap │  │   SftpWrap  │          │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
//! │         │                │                │                  │
//! │         └────────────────┼────────────────┘                  │
//! │                          │ block_on()                        │
//! │                          ▼                                   │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │              russh_backend (async bridge)              │  │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │  │
//! │  │  │RusshSession │  │RusshChannel │  │  RusshSftp  │    │  │
//! │  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │  │
//! │  │         │                │                │            │  │
//! │  │         └────────────────┴────────────────┘            │  │
//! │  │                          │                              │  │
//! │  │         ┌────────────────┴────────────────┐            │  │
//! │  │         │     Tokio Runtime (2 threads)   │            │  │
//! │  │         └────────────────────────────────-┘            │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! Russh is async-first, using tokio for I/O. We bridge to wezterm-ssh's
//! synchronous API using a dedicated tokio runtime and `block_on` calls.
//!
//! ## Features
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **Pure Rust** | No C dependencies, simplified builds |
//! | **Windows Pageant** | Native SSH agent support on Windows |
//! | **SFTP** | Full file transfer via [`russh-sftp`] |
//! | **Modern Crypto** | Ring backend for cryptography |
//!
//! ## Module Structure
//!
//! - [`handler`]: SSH client event handler (host verification, banners)
//! - [`session`]: Connection and authentication management
//! - [`channel`]: PTY and command execution channels
//! - [`sftp`]: SFTP file operations
//!
//! ## Usage
//!
//! This module is used internally by wezterm-ssh when the `russh` feature
//! is enabled (default). Users interact with the higher-level wezterm-ssh API.
//!
//! ```toml
//! # Cargo.toml - russh is the default
//! [dependencies]
//! wezterm-ssh = "0.4"  # Uses russh by default
//!
//! # Or explicitly:
//! wezterm-ssh = { version = "0.4", features = ["russh"] }
//! ```
//!
//! ## Performance Considerations
//!
//! - **Runtime**: Single shared tokio runtime with 2 worker threads
//! - **Blocking**: `block_on()` bridges async to sync, may block caller
//! - **SFTP**: File operations use async I/O internally
//!
//! [`russh`]: https://docs.rs/russh
//! [`russh-sftp`]: https://docs.rs/russh-sftp

mod channel;
mod handler;
mod session;
mod sftp;

#[cfg(test)]
mod tests;

pub use channel::RusshChannel;
pub use session::RusshSession;
pub use sftp::{RusshDir, RusshFile, RusshSftp};

use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// Get the shared tokio runtime for russh operations.
///
/// This runtime is used to bridge russh's async API with wezterm-ssh's
/// synchronous interface.
pub(crate) fn get_runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("russh-worker")
            .build()
            .expect("Failed to create tokio runtime for russh")
    })
}

/// Block on an async operation using the shared runtime.
pub(crate) fn block_on<F: std::future::Future>(future: F) -> F::Output {
    get_runtime().block_on(future)
}
