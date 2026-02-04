//! WezTerm Filesystem Explorer Library
//!
//! This library provides utilities for filesystem exploration with WSL path translation,
//! git integration, shell detection, and IPC support for WezTerm.
//!
//! # IPC Support
//!
//! The `ipc` module provides cross-platform Unix Domain Socket support for inter-process
//! communication using `tokio::net::UnixStream`. This works on both Windows (10 build 17063+)
//! and Unix platforms through tokio's native async UDS implementation.
//!
//! ## Example
//!
//! ```no_run
//! use wezterm_fs_explorer::ipc::{IpcServer, IpcClient};
//!
//! # async fn example() -> std::io::Result<()> {
//! // Create an IPC server
//! let server = IpcServer::bind("/tmp/wezterm-explorer.sock")?;
//! let stream = server.accept().await?;
//!
//! // Connect as a client
//! let client = IpcClient::connect("/tmp/wezterm-explorer.sock").await?;
//! # Ok(())
//! # }
//! ```

pub mod ipc;
pub mod path_utils;
pub mod shell;

// Re-export commonly used types
pub use ipc::{IpcClient, IpcServer, IpcStream};
pub use path_utils::{detect_path_type, normalize_path, to_windows_path, to_wsl_path, PathType};
pub use shell::{detect_shell, execute_command, translate_command, translate_path_in_command, Shell, ShellError};
