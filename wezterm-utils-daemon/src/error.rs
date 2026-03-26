//! Error types for wezterm-utils-daemon

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Routing error: {0}")]
    Routing(String),

    #[error("Connection limit reached: max {0} connections")]
    ConnectionLimitReached(usize),

    #[error("Utility not found: {0}")]
    UtilityNotFound(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Named pipe error: {0}")]
    NamedPipe(String),

    #[cfg(windows)]
    #[error("Windows API error: {0}")]
    WindowsApi(#[from] windows::core::Error),

    #[error("Shutdown in progress")]
    ShuttingDown,
}

pub type Result<T> = std::result::Result<T, DaemonError>;
