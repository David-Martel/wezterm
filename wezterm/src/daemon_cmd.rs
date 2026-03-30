//! `wezterm daemon` subcommand -- starts the IPC utility daemon.

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct DaemonCommand {
    /// Configuration file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Named pipe path (overrides config)
    #[arg(short, long)]
    pipe: Option<String>,

    /// Maximum concurrent connections (overrides config)
    #[arg(short, long)]
    max_connections: Option<usize>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

impl DaemonCommand {
    pub fn run(&self) -> anyhow::Result<()> {
        use tracing::info;
        use wezterm_utils_daemon::config::Config;
        use wezterm_utils_daemon::server::IpcServer;

        // Try to initialize tracing — may fail if wezterm already set up a subscriber.
        // When run as `wezterm daemon`, the parent binary installs env_bootstrap logging
        // first, so try_init() gracefully no-ops in that case.
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&self.log_level));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let config_path = self.config.clone();
        let pipe_override = self.pipe.clone();
        let max_connections_override = self.max_connections;

        rt.block_on(async move {
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

            let version = env!("CARGO_PKG_VERSION").to_string();
            let server = IpcServer::new(config.pipe_name.clone(), config.max_connections, version);

            info!("Starting daemon...");

            // Setup graceful shutdown
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                if let Err(e) = tokio::signal::ctrl_c().await {
                    tracing::error!("Failed to listen for Ctrl+C: {}", e);
                    return;
                }
                info!("Received shutdown signal");
                let _ = shutdown_tx.send(());
            });

            // Run server with shutdown handling
            tokio::select! {
                result = server.run() => {
                    if let Err(e) = result {
                        tracing::error!("Server error: {}", e);
                        return Err(anyhow::anyhow!("Daemon server error: {}", e));
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Shutting down gracefully...");
                }
            }

            Ok(())
        })
    }
}
