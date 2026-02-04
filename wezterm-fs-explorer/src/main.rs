mod app;
mod error;
mod file_entry;
mod git_status;
mod icons;
mod ipc;
mod ipc_client;
mod keybindings;
mod operations;
mod path_utils;
mod search;
mod shell;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ipc_client::IpcClient;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io, path::{Path, PathBuf}, time::Duration};

#[derive(Parser, Debug)]
#[command(name = "wezterm-fs-explorer")]
#[command(about = "High-performance filesystem explorer for WezTerm", long_about = None)]
struct Args {
    /// Starting directory path
    #[arg(value_name = "DIRECTORY")]
    directory: Option<PathBuf>,

    /// Output selected paths as JSON
    #[arg(long)]
    json: bool,

    /// IPC socket path for communication with wezterm-utils-daemon
    #[arg(long, value_name = "PATH")]
    ipc_socket: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    let start_dir = args
        .directory
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    if !start_dir.exists() {
        anyhow::bail!("Directory does not exist: {}", start_dir.display());
    }

    // Initialize IPC client if socket path provided
    let mut ipc_client = if let Some(socket_path) = args.ipc_socket {
        let mut client = IpcClient::new(socket_path);
        if let Err(e) = client.connect().await {
            eprintln!("Warning: Failed to connect to IPC daemon: {}", e);
            eprintln!("Running in standalone mode");
        }
        Some(client)
    } else {
        None
    };

    // Run the application
    let result = if args.json {
        run_json_mode(&start_dir)
    } else {
        run_interactive_mode(&start_dir, ipc_client.as_mut()).await
    };

    // Handle result
    match result {
        Ok(selected_paths) => {
            if args.json {
                println!("{}", serde_json::to_string(&selected_paths)?);
            } else {
                for path in selected_paths {
                    println!("{}", path.display());
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_interactive_mode(
    start_dir: &Path,
    mut ipc_client: Option<&mut IpcClient>,
) -> Result<Vec<PathBuf>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(start_dir.to_path_buf())?;

    // Start IPC event listener if client exists
    if let Some(client) = ipc_client.as_mut() {
        if client.is_connected() {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            client.start_event_listener(tx).await?;

            // Send initial watch directory message
            client
                .send_message(ipc_client::IpcMessage::WatchDirectory {
                    path: start_dir.to_path_buf(),
                })
                .await?;
        }
    }

    // Run event loop
    let result = run_app(&mut terminal, &mut app, ipc_client).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut ipc_client: Option<&mut IpcClient>,
) -> Result<Vec<PathBuf>> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Check for IPC messages
        if let Some(client) = ipc_client.as_mut() {
            if let Some(msg) = client.try_recv() {
                handle_ipc_message(app, msg)?;
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(vec![]);
                    }
                    (KeyCode::Char('q'), _) => {
                        return Ok(vec![]);
                    }
                    (KeyCode::Esc, _) => {
                        if app.mode == app::AppMode::Search {
                            app.exit_search();
                        } else {
                            return Ok(vec![]);
                        }
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(selected) = app.get_selected_paths() {
                            // Send open file message via IPC
                            if let Some(client) = ipc_client.as_mut() {
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
                        app.move_down();
                    }
                    (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                        app.move_up();
                    }
                    (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
                        app.go_parent();
                        // Notify IPC of directory change
                        if let Some(client) = ipc_client.as_mut() {
                            client
                                .send_message(ipc_client::IpcMessage::WatchDirectory {
                                    path: app.current_dir.clone(),
                                })
                                .await?;
                        }
                    }
                    (KeyCode::Char('l'), _) | (KeyCode::Right, _) => {
                        app.enter_directory()?;
                        // Notify IPC of directory change
                        if let Some(client) = ipc_client.as_mut() {
                            client
                                .send_message(ipc_client::IpcMessage::WatchDirectory {
                                    path: app.current_dir.clone(),
                                })
                                .await?;
                        }
                    }
                    (KeyCode::Char('g'), _) => {
                        app.go_top();
                    }
                    (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                        app.go_bottom();
                    }
                    (KeyCode::Char('/'), _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                        app.start_search();
                    }
                    (KeyCode::Char(' '), _) => {
                        app.toggle_selection();
                    }
                    (KeyCode::Char('.'), _) => {
                        app.toggle_hidden_files()?;
                    }
                    (KeyCode::Tab, _) => {
                        app.toggle_preview_pane();
                    }
                    (KeyCode::Char('d'), _) => {
                        app.start_delete_mode();
                    }
                    (KeyCode::Char('r'), _) => {
                        app.start_rename_mode();
                    }
                    (KeyCode::Char('c'), _) => {
                        app.start_copy_mode();
                    }
                    (KeyCode::Char('m'), _) => {
                        app.start_move_mode();
                    }
                    (KeyCode::Char('n'), _) => {
                        app.start_new_mode();
                    }
                    (KeyCode::Char('y'), _) => {
                        if app.is_confirmation_mode() {
                            app.confirm_action()?;
                        }
                    }
                    (KeyCode::Char(c), _) => {
                        if app.is_input_mode() {
                            app.handle_input(c);
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        if app.is_input_mode() {
                            app.backspace_input();
                        }
                    }
                    _ => {}
                }
            }
        }

        app.update()?;
    }
}

fn run_json_mode(start_dir: &Path) -> Result<Vec<PathBuf>> {
    // For JSON mode, just return the directory
    Ok(vec![start_dir.to_path_buf()])
}

fn handle_ipc_message(app: &mut App, msg: ipc_client::IpcMessage) -> Result<()> {
    match msg {
        ipc_client::IpcMessage::RefreshFile { path, change_type } => {
            log::info!("IPC: Refresh file {} ({})", path.display(), change_type);
            app.refresh_entries()?;
        }
        ipc_client::IpcMessage::Navigate { directory } => {
            log::info!("IPC: Navigate to {}", directory.display());
            app.current_dir = directory;
            app.refresh_entries()?;
        }
        ipc_client::IpcMessage::OpenFile { path, line, column } => {
            log::info!("IPC: Open file {} at {:?}:{:?}", path.display(), line, column);
            if let Err(e) = ipc_client::open_file_in_editor(&path, line, column) {
                app.error_message = Some(format!("Failed to open file: {}", e));
            }
        }
        _ => {
            log::debug!("IPC: Unhandled message: {:?}", msg);
        }
    }
    Ok(())
}