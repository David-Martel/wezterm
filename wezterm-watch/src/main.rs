use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wezterm_watch::git::GitMonitor;
use wezterm_watch::output::{OutputFormat, OutputFormatter};
use wezterm_watch::watcher::FileWatcher;

#[derive(Parser, Debug)]
#[command(name = "wezterm-watch")]
#[command(about = "High-performance file watcher with Git integration for WezTerm")]
#[command(version)]
struct Args {
    /// Directory to watch
    #[arg(value_name = "PATH")]
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Resolve path
    let watch_path = args
        .path
        .canonicalize()
        .context("Failed to resolve watch path")?;

    // Validate output format
    let format = OutputFormat::from_str(&args.format)
        .context("Invalid output format. Use: json, pretty, events, or summary")?;

    // Initialize Git monitor
    let git_enabled = if args.no_git {
        false
    } else if args.git {
        true
    } else {
        // Auto-detect
        GitMonitor::new(&watch_path).is_git_repo()
    };

    let git_monitor = if git_enabled {
        Some(GitMonitor::new(&watch_path))
    } else {
        None
    };

    // Show initial status if requested
    if args.status {
        if let Some(monitor) = &git_monitor {
            let info = monitor.get_status()?;
            let formatter = OutputFormatter::new(format);
            println!("{}", formatter.format_git_info(&info));
        } else {
            eprintln!("Not a git repository or git disabled");
        }
        return Ok(());
    }

    // Initialize file watcher
    let use_gitignore = !args.no_gitignore;
    let mut watcher = FileWatcher::new(
        watch_path.clone(),
        args.interval,
        use_gitignore,
        args.ignore_patterns,
    )?;

    watcher.watch(args.recursive == 0 || args.recursive > 1)?;

    let formatter = OutputFormatter::new(format);

    // Setup signal handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl-C handler")?;

    // Print initial git status for summary/pretty modes
    if matches!(format, OutputFormat::Pretty | OutputFormat::Summary) {
        if let Some(monitor) = &git_monitor {
            if let Ok(info) = monitor.get_status() {
                println!("{}", formatter.format_git_info(&info));
                println!(); // Blank line separator
            }
        }
    }

    // Main event loop
    let receiver = watcher.receiver();
    let mut pending_events: Vec<(
        wezterm_watch::watcher::WatchEvent,
        Option<wezterm_watch::git::FileStatus>,
    )> = Vec::new();

    while running.load(Ordering::SeqCst) {
        match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(event) => {
                pending_events.push((event, None));

                // Drain any additional ready events without blocking
                while let Ok(extra) = receiver.try_recv() {
                    pending_events.push((extra, None));
                }

                // Single git status refresh for the entire batch
                if let Some(monitor) = &git_monitor {
                    monitor.invalidate_cache();
                    // Fetch the status snapshot once, then resolve each path
                    // against it without re-cloning the full HashMap.
                    if let Ok(info) = monitor.get_status() {
                        let repo_root = monitor.repo_root();
                        for (evt, status_slot) in &mut pending_events {
                            if let Some(path) = evt.path() {
                                *status_slot =
                                    GitMonitor::resolve_file_status(&info, path, repo_root);
                            }
                        }
                    }
                }

                // Format and print the whole batch
                for (evt, git_status) in pending_events.drain(..) {
                    let output = formatter.format_event(&evt, git_status.as_ref());
                    if !output.is_empty() {
                        println!("{output}");
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Periodic git status update for summary mode
                if matches!(format, OutputFormat::Summary) {
                    if let Some(monitor) = &git_monitor {
                        if let Ok(info) = monitor.get_status() {
                            // Only print if there are changes
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

// Minimal Ctrl-C handling
mod ctrlc {
    use anyhow::Result;
    use std::sync::atomic::{AtomicBool, Ordering};

    static HANDLER_SET: AtomicBool = AtomicBool::new(false);

    pub fn set_handler<F>(_handler: F) -> Result<()>
    where
        F: Fn() + 'static + Send,
    {
        if HANDLER_SET.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        std::thread::spawn(move || {
            // Simple signal handler that calls the closure
            #[cfg(unix)]
            {
                use std::io::Read;
                let mut stdin = std::io::stdin();
                let mut buf = [0u8; 1];
                loop {
                    if stdin.read(&mut buf).is_err() {
                        handler();
                        break;
                    }
                }
            }

            #[cfg(windows)]
            {
                // Windows: use a simple sleep loop
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        // Basic parsing test
        let args = Args::parse_from(["wezterm-watch", "."]);
        assert_eq!(args.interval, 100);
        assert_eq!(args.format, "pretty");
    }
}
