//! WezTerm Utilities Daemon -- library API.
//!
//! Provides the IPC router daemon client, protocol types, and server
//! for embedding in other binaries (e.g., `wezterm daemon` subcommand).

pub mod client;
pub mod config;
pub mod connections;
pub mod error;
pub mod protocol;
pub mod router;
pub mod server;
