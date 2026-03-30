//! `wezterm watch` subcommand -- file watcher with git integration.

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct WatchCommand {
    /// Directory to watch
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Output format: json, pretty, events, summary
    #[arg(short, long, default_value = "pretty")]
    format: String,

    /// Debounce interval in milliseconds
    #[arg(short = 'd', long, default_value = "100")]
    interval: u64,

    /// Enable git integration (default: auto-detect)
    #[arg(short, long)]
    git: bool,

    /// Disable git integration
    #[arg(long)]
    no_git: bool,

    /// Additional ignore patterns (can be specified multiple times)
    #[arg(short = 'i', long = "ignore")]
    ignore_patterns: Vec<String>,

    /// Disable .gitignore file handling
    #[arg(long)]
    no_gitignore: bool,

    /// Maximum recursion depth (0 for unlimited)
    #[arg(short, long, default_value = "0")]
    recursive: usize,

    /// Show initial git status and exit
    #[arg(long)]
    status: bool,

    /// Verbose output (show ignored files)
    #[arg(short, long)]
    verbose: bool,
}

impl WatchCommand {
    pub fn run(&self) -> anyhow::Result<()> {
        let config = wezterm_watch::WatchConfig {
            format: self.format.clone(),
            interval: self.interval,
            git: self.git,
            no_git: self.no_git,
            ignore_patterns: self.ignore_patterns.clone(),
            no_gitignore: self.no_gitignore,
            recursive: self.recursive,
            status: self.status,
            verbose: self.verbose,
        };

        wezterm_watch::run(&self.path, config)
    }
}
