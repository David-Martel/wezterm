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

pub mod app;
pub mod error;
pub mod file_entry;
pub mod git_status;
pub mod icons;
pub mod ipc;
pub mod ipc_client;
pub mod keybindings;
pub mod operations;
pub mod path_utils;
pub mod search;
pub mod shell;
pub mod ui;

// Re-export commonly used types
pub use ipc::{IpcClient, IpcServer, IpcStream};
pub use ipc_client::{
    open_file_in_editor, IpcMessage, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
};
pub use path_utils::{detect_path_type, normalize_path, to_windows_path, to_wsl_path, PathType};
pub use search::{FuzzySearch, SearchResult};
pub use shell::{
    detect_shell, execute_command, translate_command, translate_path_in_command, Shell, ShellError,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::FutureExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{any::Any, io, path::Path, path::PathBuf, time::Duration};

/// Configuration for the filesystem explorer.
pub struct ExploreConfig {
    /// Output selected paths as JSON instead of interactive mode
    pub json: bool,
    /// IPC socket path for communication with wezterm-utils-daemon
    pub ipc_socket: Option<String>,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self {
            json: false,
            ipc_socket: None,
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restored: bool,
}

impl TerminalSession {
    fn enter() -> anyhow::Result<Self> {
        use anyhow::Context;
        enable_raw_mode().context("failed to enable raw mode")?;

        let session = (|| -> anyhow::Result<Self> {
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
                .context("failed to enter alternate screen")?;
            let backend = CrosstermBackend::new(stdout);
            let terminal =
                Terminal::new(backend).context("failed to initialize terminal backend")?;
            Ok(Self {
                terminal,
                restored: false,
            })
        })();

        if session.is_err() {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        }

        session
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    fn restore(&mut self) -> anyhow::Result<()> {
        use anyhow::Context;
        if self.restored {
            return Ok(());
        }

        disable_raw_mode().context("failed to disable raw mode")?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .context("failed to leave alternate screen")?;
        self.terminal
            .show_cursor()
            .context("failed to restore cursor state")?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Run the filesystem explorer on the given directory.
///
/// This is the library entry point equivalent to `wezterm-fs-explorer`'s `main()`.
pub async fn run_explorer(start_dir: &Path, config: ExploreConfig) -> anyhow::Result<()> {
    if !start_dir.exists() {
        anyhow::bail!("Directory does not exist: {}", start_dir.display());
    }

    if !start_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", start_dir.display());
    }

    // Initialize IPC client if socket path provided
    let mut ipc_client_inst = if let Some(socket_path) = config.ipc_socket {
        let mut client = ipc_client::IpcClient::new(socket_path);
        if let Err(e) = client.connect().await {
            eprintln!("Warning: Failed to connect to IPC daemon: {}", e);
            eprintln!("Running in standalone mode");
        }
        Some(client)
    } else {
        None
    };

    if config.json {
        println!("{}", serde_json::to_string(&[start_dir])?);
        return Ok(());
    }

    let selected_paths = run_interactive(start_dir, ipc_client_inst.as_mut()).await?;
    for path in selected_paths {
        println!("{}", path.display());
    }
    Ok(())
}

async fn run_interactive(
    start_dir: &Path,
    mut ipc_client_ref: Option<&mut ipc_client::IpcClient>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut terminal_session = TerminalSession::enter()?;

    let mut explorer_app = app::App::new(start_dir.to_path_buf())?;

    if let Some(client) = ipc_client_ref.as_mut() {
        if client.is_connected() {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            client.start_event_listener(tx).await?;

            client
                .send_message(ipc_client::IpcMessage::WatchDirectory {
                    path: start_dir.to_path_buf(),
                })
                .await?;
        }
    }

    let result = {
        let terminal = terminal_session.terminal_mut();
        std::panic::AssertUnwindSafe(run_app_loop(terminal, &mut explorer_app, ipc_client_ref))
            .catch_unwind()
            .await
    };

    terminal_session.restore()?;

    match result {
        Ok(result) => result,
        Err(payload) => Err(anyhow::anyhow!(
            "wezterm-fs-explorer panicked: {}",
            panic_payload_message(payload)
        )),
    }
}

async fn run_app_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    explorer_app: &mut app::App,
    mut ipc_client_ref: Option<&mut ipc_client::IpcClient>,
) -> anyhow::Result<Vec<PathBuf>> {
    loop {
        terminal.draw(|f| ui::draw(f, explorer_app))?;

        if let Some(client) = ipc_client_ref.as_mut() {
            if let Some(msg) = client.try_recv() {
                handle_ipc_msg(explorer_app, msg)?;
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(vec![]);
                    }
                    _ if explorer_app.mode == app::AppMode::Help => {
                        explorer_app.hide_help();
                    }
                    (KeyCode::Char('q'), _) => {
                        return Ok(vec![]);
                    }
                    (KeyCode::Esc, _) => {
                        if explorer_app.mode == app::AppMode::Search {
                            explorer_app.exit_search();
                        } else {
                            return Ok(vec![]);
                        }
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(selected) = explorer_app.get_selected_paths() {
                            if let Some(client) = ipc_client_ref.as_mut() {
                                for path in &selected {
                                    if path.is_file() {
                                        client
                                            .send_message(ipc_client::IpcMessage::OpenFile {
                                                path: path.clone(),
                                                line: None,
                                                column: None,
                                            })
                                            .await?;
                                    }
                                }
                            }
                            return Ok(selected);
                        }
                    }
                    (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                        explorer_app.move_down();
                    }
                    (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                        explorer_app.move_up();
                    }
                    (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
                        explorer_app.go_parent();
                        if let Some(client) = ipc_client_ref.as_mut() {
                            client
                                .send_message(ipc_client::IpcMessage::WatchDirectory {
                                    path: explorer_app.current_dir.clone(),
                                })
                                .await?;
                        }
                    }
                    (KeyCode::Char('l'), _) | (KeyCode::Right, _) => {
                        explorer_app.enter_directory()?;
                        if let Some(client) = ipc_client_ref.as_mut() {
                            client
                                .send_message(ipc_client::IpcMessage::WatchDirectory {
                                    path: explorer_app.current_dir.clone(),
                                })
                                .await?;
                        }
                    }
                    (KeyCode::Char('g'), _) => {
                        explorer_app.go_top();
                    }
                    (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                        explorer_app.go_bottom();
                    }
                    (KeyCode::Char('/'), _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                        explorer_app.start_search();
                    }
                    (KeyCode::Char(' '), _) => {
                        explorer_app.toggle_selection();
                    }
                    (KeyCode::Char('.'), _) => {
                        explorer_app.toggle_hidden_files()?;
                    }
                    (KeyCode::Tab, _) => {
                        explorer_app.toggle_preview_pane();
                    }
                    (KeyCode::Char('d'), _) => {
                        explorer_app.start_delete_mode();
                    }
                    (KeyCode::Char('r'), _) => {
                        explorer_app.start_rename_mode();
                    }
                    (KeyCode::Char('c'), _) => {
                        explorer_app.start_copy_mode();
                    }
                    (KeyCode::Char('m'), _) => {
                        explorer_app.start_move_mode();
                    }
                    (KeyCode::Char('n'), _) => {
                        explorer_app.start_new_mode();
                    }
                    (KeyCode::Char('?'), _) => {
                        explorer_app.show_help();
                    }
                    (KeyCode::Char('y'), _) => {
                        if explorer_app.is_confirmation_mode() {
                            explorer_app.confirm_action()?;
                        }
                    }
                    (KeyCode::Char(c), _) => {
                        if explorer_app.is_input_mode() {
                            explorer_app.handle_input(c);
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        if explorer_app.is_input_mode() {
                            explorer_app.backspace_input();
                        }
                    }
                    _ => {}
                }
            }
        }

        explorer_app.update()?;
    }
}

fn panic_payload_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn handle_ipc_msg(explorer_app: &mut app::App, msg: ipc_client::IpcMessage) -> anyhow::Result<()> {
    match msg {
        ipc_client::IpcMessage::RefreshFile { path, change_type } => {
            log::info!("IPC: Refresh file {} ({})", path.display(), change_type);
            explorer_app.refresh_entries()?;
        }
        ipc_client::IpcMessage::Navigate { directory } => {
            log::info!("IPC: Navigate to {}", directory.display());
            explorer_app.current_dir = directory;
            explorer_app.refresh_entries()?;
        }
        ipc_client::IpcMessage::OpenFile { path, line, column } => {
            log::info!(
                "IPC: Open file {} at {:?}:{:?}",
                path.display(),
                line,
                column
            );
            if let Err(e) = ipc_client::open_file_in_editor(&path, line, column) {
                explorer_app.error_message = Some(format!("Failed to open file: {}", e));
            }
        }
        _ => {
            log::debug!("IPC: Unhandled message: {:?}", msg);
        }
    }
    Ok(())
}
