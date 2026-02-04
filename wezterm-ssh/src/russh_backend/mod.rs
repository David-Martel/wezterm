//! Pure-Rust SSH backend using russh.
//!
//! This backend eliminates all C library dependencies (libssh, libssh2, OpenSSL)
//! and provides native Windows Pageant support via the russh ecosystem.
//!
//! ## Architecture
//!
//! Russh is async-first, using tokio for I/O. We bridge to wezterm-ssh's
//! synchronous API using a dedicated tokio runtime and `block_on` calls.
//!
//! ## Features
//!
//! - Pure Rust implementation (no C dependencies)
//! - Native Windows Pageant support
//! - SSH agent forwarding
//! - SFTP via russh-sftp
//! - Modern cryptography (aws-lc-rs backend - FIPS validated)

mod handler;
mod session;
mod channel;

#[cfg(test)]
mod tests;

pub use handler::WezTermHandler;
pub use session::RusshSession;
pub use channel::RusshChannel;

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
