//! wezterm-utils-daemon - IPC Router Daemon for WezTerm utilities
//!
//! A high-performance daemon that routes JSON-RPC 2.0 messages between WezTerm utilities
//! using Windows Named Pipes.

pub mod client;
mod config;
mod connections;
mod error;
pub mod protocol;
mod router;
mod server;

use clap::{Parser, Subcommand};
use config::Config;
use error::Result;
use server::IpcServer;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "wezterm-utils-daemon")]
#[command(version = VERSION)]
#[command(about = "IPC Router Daemon for WezTerm utilities", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long)]
    json_logs: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon server
    Start {
        /// Named pipe path
        #[arg(short, long)]
        pipe: Option<String>,

        /// Maximum concurrent connections
        #[arg(short, long)]
        max_connections: Option<usize>,
    },

    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for config file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate configuration file
    ValidateConfig {
        /// Config file to validate
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Show daemon status (requires running daemon)
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    init_tracing(&cli.log_level, cli.json_logs);

    info!("wezterm-utils-daemon v{}", VERSION);

    match cli.command {
        Some(Commands::Start {
            pipe,
            max_connections,
        }) => {
            start_daemon(cli.config, pipe, max_connections).await?;
        }
        Some(Commands::GenerateConfig { output }) => {
            generate_config(output).await?;
        }
        Some(Commands::ValidateConfig { file }) => {
            validate_config(file.or(cli.config)).await?;
        }
        Some(Commands::Status) => {
            show_status().await?;
        }
        None => {
            // Default: start daemon
            start_daemon(cli.config, None, None).await?;
        }
    }

    Ok(())
}

async fn start_daemon(
    config_path: Option<PathBuf>,
    pipe_override: Option<String>,
    max_connections_override: Option<usize>,
) -> Result<()> {
    // Load configuration
    let mut config = if let Some(path) = config_path {
        info!("Loading configuration from {:?}", path);
        Config::load_from_file(path).await?
    } else {
        info!("Using default configuration");
        Config::default()
    };

    // Apply CLI overrides
    if let Some(pipe) = pipe_override {
        config.pipe_name = pipe;
    }
    if let Some(max_conn) = max_connections_override {
        config.max_connections = max_conn;
    }

    // Validate configuration
    config.validate()?;

    info!("Configuration:");
    info!("  Pipe name: {}", config.pipe_name);
    info!("  Max connections: {}", config.max_connections);
    info!("  Keep-alive: {}s", config.keep_alive_seconds);
    info!("  Timeout: {}s", config.timeout_seconds);

    // Create and start server
    let server = IpcServer::new(
        config.pipe_name.clone(),
        config.max_connections,
        VERSION.to_string(),
    );

    info!("Starting daemon...");

    // Setup graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        info!("Received shutdown signal");
        let _ = shutdown_tx.send(());
    });

    // Run server with shutdown handling
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(e);
            }
        }
        _ = &mut shutdown_rx => {
            info!("Shutting down gracefully...");
        }
    }

    Ok(())
}

async fn generate_config(output: Option<PathBuf>) -> Result<()> {
    let config = Config::default();
    let path = output.unwrap_or_else(Config::default_path);

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    config.save_to_file(&path).await?;

    info!("Configuration file generated: {:?}", path);
    println!("Configuration file generated: {}", path.display());

    Ok(())
}

async fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    let path = config_path.unwrap_or_else(Config::default_path);

    info!("Validating configuration: {:?}", path);

    let config = Config::load_from_file(&path).await?;
    config.validate()?;

    println!("✓ Configuration is valid");
    println!("  Pipe name: {}", config.pipe_name);
    println!("  Max connections: {}", config.max_connections);
    println!("  Keep-alive: {}s", config.keep_alive_seconds);
    println!("  Timeout: {}s", config.timeout_seconds);

    Ok(())
}

async fn show_status() -> Result<()> {
    info!("Requesting daemon status...");

    use crate::protocol::{JsonRpcRequest, RequestId};
    use server::connect_client;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    // Connect to daemon
    let config = Config::default();
    let mut client = connect_client(&config.pipe_name).await?;

    // Send status request
    let request = JsonRpcRequest::new("daemon/status", None, Some(RequestId::Number(1)));

    let json = serde_json::to_string(&request)?;
    client.write_all(format!("{}\n", json).as_bytes()).await?;
    client.flush().await?;

    // Read response
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: serde_json::Value = serde_json::from_str(&line)?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

fn init_tracing(level: &str, json_logs: bool) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    if json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(&["wezterm-utils-daemon", "start"]);
        assert!(matches!(cli.command, Some(Commands::Start { .. })));
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
