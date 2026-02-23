//! Configuration management for wezterm-utils-daemon

use crate::error::{DaemonError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Named pipe path
    #[serde(default = "default_pipe_name")]
    pub pipe_name: String,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Enable JSON logging
    #[serde(default)]
    pub json_logging: bool,

    /// Keep-alive interval in seconds
    #[serde(default = "default_keep_alive_seconds")]
    pub keep_alive_seconds: u64,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Process ID validation (Windows only)
    #[serde(default = "default_true")]
    pub validate_process_ids: bool,
}

fn default_pipe_name() -> String {
    r"\\.\pipe\wezterm-utils-ipc".to_string()
}

fn default_max_connections() -> usize {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_keep_alive_seconds() -> u64 {
    30
}

fn default_timeout_seconds() -> u64 {
    120
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pipe_name: default_pipe_name(),
            max_connections: default_max_connections(),
            log_level: default_log_level(),
            json_logging: false,
            keep_alive_seconds: default_keep_alive_seconds(),
            timeout_seconds: default_timeout_seconds(),
            validate_process_ids: default_true(),
        }
    }
}

impl Config {
    /// Load configuration from TOML file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!(path = ?path, "Loading configuration");

        let content = fs::read_to_string(path).await
            .map_err(|e| DaemonError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)?;

        Ok(config)
    }

    /// Save configuration to TOML file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!(path = ?path, "Saving configuration");

        let content = toml::to_string_pretty(self)
            .map_err(|e| DaemonError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content).await
            .map_err(|e| DaemonError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get default config file path
    pub fn default_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("wezterm-utils-daemon");
        path.push("config.toml");
        path
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.max_connections == 0 {
            return Err(DaemonError::Config(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        if self.max_connections > 1000 {
            return Err(DaemonError::Config(
                "max_connections must be 1000 or less".to_string(),
            ));
        }

        if self.keep_alive_seconds == 0 {
            return Err(DaemonError::Config(
                "keep_alive_seconds must be greater than 0".to_string(),
            ));
        }

        if self.timeout_seconds < self.keep_alive_seconds {
            return Err(DaemonError::Config(
                "timeout_seconds must be >= keep_alive_seconds".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.max_connections, 10);
        assert!(config.pipe_name.contains("wezterm-utils-ipc"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.max_connections = 0;
        assert!(config.validate().is_err());

        config.max_connections = 10;
        config.timeout_seconds = 10;
        config.keep_alive_seconds = 20;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.max_connections, parsed.max_connections);
        assert_eq!(config.pipe_name, parsed.pipe_name);
    }
}