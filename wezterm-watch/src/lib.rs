//! WezTerm Watch Library
//!
//! This library provides file watching and git integration functionality.

pub mod git;
pub mod output;
pub mod watcher;

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configuration for a watch session, matching the CLI arguments.
pub struct WatchConfig {
    /// Output format: json, pretty, events, summary
    pub format: String,
    /// Debounce interval in milliseconds
    pub interval: u64,
    /// Enable git integration
    pub git: bool,
    /// Disable git integration
    pub no_git: bool,
    /// Additional ignore patterns
    pub ignore_patterns: Vec<String>,
    /// Disable .gitignore file handling
    pub no_gitignore: bool,
    /// Maximum recursion depth (0 for unlimited)
    pub recursive: usize,
    /// Show initial git status and exit
    pub status: bool,
    /// Verbose output
    pub verbose: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            format: "pretty".to_string(),
            interval: 100,
            git: false,
            no_git: false,
            ignore_patterns: Vec::new(),
            no_gitignore: false,
            recursive: 0,
            status: false,
            verbose: false,
        }
    }
}

/// Run the file watcher with the given path and configuration.
///
/// This is the library entry point equivalent to `wezterm-watch`'s `main()`.
pub fn run(watch_path: &Path, config: WatchConfig) -> Result<()> {
    let watch_path = watch_path
        .canonicalize()
        .context("Failed to resolve watch path")?;

    let format: output::OutputFormat = config
        .format
        .parse()
        .context("Invalid output format. Use: json, pretty, events, or summary")?;

    // Initialize Git monitor
    let git_enabled = if config.no_git {
        false
    } else if config.git {
        true
    } else {
        git::GitMonitor::new(&watch_path).is_git_repo()
    };

    let git_monitor = if git_enabled {
        Some(git::GitMonitor::new(&watch_path))
    } else {
        None
    };

    // Show initial status if requested
    if config.status {
        if let Some(monitor) = &git_monitor {
            let info = monitor.get_status()?;
            let formatter = output::OutputFormatter::new(format);
            println!("{}", formatter.format_git_info(&info));
        } else {
            eprintln!("Not a git repository or git disabled");
        }
        return Ok(());
    }

    // Initialize file watcher
    let use_gitignore = !config.no_gitignore;
    let mut file_watcher = watcher::FileWatcher::new(
        watch_path.clone(),
        config.interval,
        use_gitignore,
        config.ignore_patterns,
    )?;

    file_watcher.watch(config.recursive == 0 || config.recursive > 1)?;

    let formatter = output::OutputFormatter::new(format);

    // Setup signal handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Simple Ctrl-C handling
    {
        let _running_flag = r;
        std::thread::spawn(move || {
            // Platform-specific signal handling is best-effort
            #[cfg(windows)]
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            #[cfg(unix)]
            {
                use std::io::Read as _;
                let mut stdin = std::io::stdin();
                let mut buf = [0u8; 1];
                loop {
                    if stdin.read(&mut buf).is_err() {
                        _running_flag.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });
    }

    // Print initial git status for summary/pretty modes
    if matches!(format, output::OutputFormat::Pretty | output::OutputFormat::Summary) {
        if let Some(monitor) = &git_monitor {
            if let Ok(info) = monitor.get_status() {
                println!("{}", formatter.format_git_info(&info));
                println!();
            }
        }
    }

    // Main event loop
    let receiver = file_watcher.receiver();
    let mut pending_events: Vec<(
        watcher::WatchEvent,
        Option<git::FileStatus>,
    )> = Vec::new();

    while running.load(Ordering::SeqCst) {
        match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(event) => {
                pending_events.push((event, None));

                while let Ok(extra) = receiver.try_recv() {
                    pending_events.push((extra, None));
                }

                if let Some(monitor) = &git_monitor {
                    monitor.invalidate_cache();
                    if let Ok(info) = monitor.get_status() {
                        let repo_root = monitor.repo_root();
                        for (evt, status_slot) in &mut pending_events {
                            if let Some(path) = evt.path() {
                                *status_slot =
                                    git::GitMonitor::resolve_file_status(&info, path, repo_root);
                            }
                        }
                    }
                }

                for (evt, git_status) in pending_events.drain(..) {
                    let line = formatter.format_event(&evt, git_status.as_ref());
                    if !line.is_empty() {
                        println!("{line}");
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if matches!(format, output::OutputFormat::Summary) {
                    if let Some(monitor) = &git_monitor {
                        if let Ok(info) = monitor.get_status() {
                            if !info.file_statuses.is_empty() {
                                print!("\r{}", formatter.format_git_info(&info));
                                use std::io::Write;
                                std::io::stdout().flush().ok();
                            }
                        }
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    println!("\nWatcher stopped");
    Ok(())
}
