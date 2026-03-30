//! `wezterm explore` subcommand -- interactive filesystem explorer.

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct ExploreCommand {
    /// Starting directory
    #[arg(value_name = "DIRECTORY", default_value = ".")]
    path: PathBuf,

    /// Output selected paths as JSON instead of interactive mode
    #[arg(long)]
    json: bool,

    /// IPC socket path for communication with wezterm-utils-daemon
    #[arg(long, value_name = "PATH")]
    ipc_socket: Option<String>,
}

impl ExploreCommand {
    pub fn run(&self) -> anyhow::Result<()> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let start_dir = self.path.clone();
        let config = wezterm_fs_explorer::ExploreConfig {
            json: self.json,
            ipc_socket: self.ipc_socket.clone(),
        };

        rt.block_on(async move {
            wezterm_fs_explorer::run_explorer(&start_dir, config).await
        })
    }
}
