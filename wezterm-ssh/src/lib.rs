//! # WezTerm SSH
//!
//! A higher-level SSH client library providing session management, PTY handling,
//! and SFTP operations for WezTerm.
//!
//! ## Backends
//!
//! This crate supports multiple SSH backends:
//!
//! - **`russh`** (default, recommended): Pure-Rust SSH implementation using the
//!   [`russh`](https://crates.io/crates/russh) crate. Zero C dependencies,
//!   cross-platform support, and native Windows Pageant integration.
//!
//! - **`legacy`** (deprecated): OpenSSL-based backends (`libssh-rs` and `ssh2`).
//!   Requires OpenSSL/vcpkg setup. Will be removed in a future release.
//!
//! ## Features
//!
//! - SSH session management with host key verification
//! - PTY allocation and terminal emulation
//! - SFTP file operations (read, write, list directories)
//! - SSH config file parsing (`~/.ssh/config`)
//! - Agent forwarding support
//!
//! ## Example
//!
//! ```rust,no_run
//! use wezterm_ssh::{Session, Config, SessionEvent};
//!
//! // Load SSH config
//! let config = Config::new();
//! let host = config.for_host("example.com");
//!
//! // Create session (uses russh by default)
//! let (session, events) = Session::connect(host)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

#[cfg(not(any(feature = "libssh-rs", feature = "ssh2", feature = "russh")))]
compile_error!("At least one SSH backend must be enabled: libssh-rs, ssh2, or russh");

mod auth;
mod channelwrap;
mod config;
mod dirwrap;
mod filewrap;
mod host;
mod pty;
mod session;
mod sessioninner;
mod sessionwrap;
mod sftp;
mod sftpwrap;

#[cfg(feature = "russh")]
mod russh_backend;

pub use auth::*;
pub use config::*;
pub use host::*;
pub use pty::*;
pub use session::*;
pub use sftp::error::*;
pub use sftp::types::*;
pub use sftp::*;

// NOTE: Re-exported as is exposed in a public API of this crate
pub use camino::{Utf8Path, Utf8PathBuf};
pub use filedescriptor::FileDescriptor;
pub use portable_pty::{Child, ChildKiller, MasterPty, PtySize};
