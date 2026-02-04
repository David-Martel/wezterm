//! FsExplorerPane: A terminal-based filesystem explorer pane
//!
//! This pane provides a file explorer interface with vim-style keybindings,
//! integrated into WezTerm's pane system. It uses wezterm_term::Terminal for
//! rendering and follows the TermWizTerminalPane pattern.

use anyhow::{anyhow, Result};
use config::keyassignment::ScrollbackEraseMode;
use crossbeam::channel::{unbounded as channel, Receiver, Sender};
use filedescriptor::{FileDescriptor, Pipe};
use mux::domain::DomainId;
use mux::pane::{
    alloc_pane_id, CachePolicy, CloseReason, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId,
    WithPaneLines,
};
use mux::renderable::*;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rangeset::RangeSet;
use std::fmt::Write as FmtWrite;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use termwiz::surface::{Line, SequenceNo};
use url::Url;
use wezterm_term::color::ColorPalette;
use wezterm_term::{
    KeyCode, KeyModifiers, MouseEvent, StableRowIndex, Terminal, TerminalConfiguration,
    TerminalSize,
};

/// Input events sent to the file explorer
#[derive(Debug, Clone)]
pub enum FsExplorerInput {
    /// Key press event
    Key(KeyCode, KeyModifiers),
    /// Mouse event
    Mouse(MouseEvent),
    /// Resize event
    Resize { rows: usize, cols: usize },
}

/// Simplified file entry for display
#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

/// State of the file explorer application
struct FsExplorerState {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected_index: usize,
    scroll_offset: usize,
    show_hidden: bool,
    error_message: Option<String>,
}

impl FsExplorerState {
    fn new(start_dir: PathBuf) -> Result<Self> {
        let mut state = Self {
            current_dir: start_dir,
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            show_hidden: false,
            error_message: None,
        };
        state.load_directory()?;
        Ok(state)
    }

    fn load_directory(&mut self) -> Result<()> {
        self.entries.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.error_message = None;

        // Add parent directory entry
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            });
        }

        // Read directory entries
        let read_dir = std::fs::read_dir(&self.current_dir).map_err(|e| {
            self.error_message = Some(format!("Failed to read directory: {}", e));
            e
        })?;

        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files unless show_hidden is true
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            self.entries.push(FileEntry {
                name,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
            });
        }

        // Sort: directories first, then alphabetically
        self.entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(())
    }

    fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected_index < self.entries.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn page_down(&mut self, viewport_height: usize) {
        if !self.entries.is_empty() {
            self.selected_index = (self.selected_index + viewport_height).min(self.entries.len() - 1);
        }
    }

    fn page_up(&mut self, viewport_height: usize) {
        self.selected_index = self.selected_index.saturating_sub(viewport_height);
    }

    fn go_top(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn go_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    fn enter_selected(&mut self) -> Result<bool> {
        if self.entries.is_empty() {
            return Ok(false);
        }

        let entry = &self.entries[self.selected_index];
        if !entry.is_dir {
            return Ok(false); // Can't enter files
        }

        if entry.name == ".." {
            if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
            }
        } else {
            self.current_dir = self.current_dir.join(&entry.name);
        }

        self.load_directory()?;
        Ok(true)
    }

    fn go_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.load_directory()?;
        }
        Ok(())
    }

    fn toggle_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.load_directory()
    }

    fn update_scroll_offset(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        // Keep selected item visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected_index.saturating_sub(viewport_height - 1);
        }
    }
}

/// FsExplorerPane: A pane that displays a filesystem explorer
pub struct FsExplorerPane {
    pane_id: PaneId,
    domain_id: DomainId,
    terminal: Mutex<Terminal>,
    input_tx: Sender<FsExplorerInput>,
    state: Mutex<FsExplorerState>,
    dead: Mutex<bool>,
    writer: Mutex<Vec<u8>>,
    render_rx: FileDescriptor,
}

impl FsExplorerPane {
    /// Create a new FsExplorerPane
    pub fn new(
        domain_id: DomainId,
        size: TerminalSize,
        start_dir: PathBuf,
        input_tx: Sender<FsExplorerInput>,
        render_rx: FileDescriptor,
        term_config: Option<Arc<dyn TerminalConfiguration + Send + Sync>>,
    ) -> Result<Self> {
        let pane_id = alloc_pane_id();

        let terminal = Mutex::new(Terminal::new(
            size,
            term_config.unwrap_or_else(|| Arc::new(config::TermConfig::new())),
            "WezTerm FS Explorer",
            config::wezterm_version(),
            Box::new(Vec::new()),
        ));

        let state = Mutex::new(FsExplorerState::new(start_dir)?);

        Ok(Self {
            pane_id,
            domain_id,
            terminal,
            input_tx,
            state,
            dead: Mutex::new(false),
            writer: Mutex::new(Vec::new()),
            render_rx,
        })
    }

    /// Render the file list to the terminal using ANSI escape sequences
    fn render_to_terminal(&self) {
        let mut term = self.terminal.lock();
        let state = self.state.lock();
        let dims = terminal_get_dimensions(&mut term);

        // Calculate viewport
        let header_lines = 2; // Title + separator
        let footer_lines = 1; // Status line
        let viewport_height = dims.viewport_rows.saturating_sub(header_lines + footer_lines);

        // Build ANSI escape sequence buffer
        let mut output = String::new();

        // Clear screen and move to home position
        let _ = write!(output, "\x1b[2J\x1b[H");

        // Title - bold blue
        let _ = write!(
            output,
            "\x1b[1;34m WezTerm FS Explorer - {}\x1b[0m\r\n",
            state.current_dir.display()
        );

        // Separator
        for _ in 0..dims.cols {
            output.push('─');
        }
        let _ = write!(output, "\r\n");

        // Error message if any
        if let Some(ref error) = state.error_message {
            let _ = write!(output, "\x1b[31mError: {}\x1b[0m\r\n", error);
        }

        // File entries
        let start_idx = state.scroll_offset;
        let end_idx = (start_idx + viewport_height).min(state.entries.len());

        for idx in start_idx..end_idx {
            let entry = &state.entries[idx];
            let is_selected = idx == state.selected_index;

            // Selection indicator (reverse video)
            if is_selected {
                let _ = write!(output, "\x1b[7m");
            }

            // Icon and name
            let icon = if entry.is_dir { "📁" } else { "📄" };
            let name_display = format!("{} {}", icon, entry.name);

            let _ = write!(output, " {:<50}", name_display);

            // Size
            if !entry.is_dir && entry.name != ".." {
                let _ = write!(output, " {:>10}", format_size(entry.size));
            }

            if is_selected {
                let _ = write!(output, "\x1b[27m"); // Turn off reverse
            }

            let _ = write!(output, "\r\n");
        }

        // Move cursor to status line position
        let status_y = dims.viewport_rows;
        let _ = write!(output, "\x1b[{};1H", status_y);

        // Status line - white on navy blue background
        let _ = write!(output, "\x1b[37;44m"); // White text, blue background
        let status = format!(
            " {} items | [j/k] move [Enter] open [h] parent [.] hidden [q] quit ",
            state.entries.len()
        );
        let _ = write!(output, "{:width$}", status, width = dims.cols);
        let _ = write!(output, "\x1b[0m"); // Reset attributes

        // Send the output to the terminal
        term.advance_bytes(output.as_bytes());
    }

    /// Handle a key press
    fn handle_key(&self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        let mut state = self.state.lock();
        let dims = terminal_get_dimensions(&mut self.terminal.lock());
        let viewport_height = dims.viewport_rows.saturating_sub(3);

        match key {
            // Navigation
            KeyCode::Char('j') | KeyCode::DownArrow => {
                state.move_down();
            }
            KeyCode::Char('k') | KeyCode::UpArrow => {
                state.move_up();
            }
            KeyCode::Char('g') => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    state.go_bottom();
                } else {
                    state.go_top();
                }
            }
            KeyCode::PageDown | KeyCode::Char('d') if modifiers.contains(KeyModifiers::CTRL) => {
                state.page_down(viewport_height);
            }
            KeyCode::PageUp | KeyCode::Char('u') if modifiers.contains(KeyModifiers::CTRL) => {
                state.page_up(viewport_height);
            }

            // Actions
            KeyCode::Enter => {
                state.enter_selected()?;
            }
            KeyCode::Char('h') | KeyCode::LeftArrow => {
                state.go_parent()?;
            }
            KeyCode::Char('.') => {
                state.toggle_hidden()?;
            }

            // Quit
            KeyCode::Char('q') | KeyCode::Escape => {
                self.kill();
                return Ok(());
            }

            _ => {}
        }

        state.update_scroll_offset(viewport_height);
        drop(state);

        // Re-render after state change
        self.render_to_terminal();

        Ok(())
    }
}

impl Pane for FsExplorerPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn get_cursor_position(&self) -> StableCursorPosition {
        terminal_get_cursor_position(&mut self.terminal.lock())
    }

    fn get_current_seqno(&self) -> SequenceNo {
        self.terminal.lock().current_seqno()
    }

    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        terminal_get_dirty_lines(&mut self.terminal.lock(), lines, seqno)
    }

    fn for_each_logical_line_in_stable_range_mut(
        &self,
        lines: Range<StableRowIndex>,
        for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        terminal_for_each_logical_line_in_stable_range_mut(
            &mut self.terminal.lock(),
            lines,
            for_line,
        );
    }

    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        mux::pane::impl_get_logical_lines_via_get_lines(self, lines)
    }

    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        terminal_with_lines_mut(&mut self.terminal.lock(), lines, with_lines)
    }

    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        terminal_get_lines(&mut self.terminal.lock(), lines)
    }

    fn get_dimensions(&self) -> RenderableDimensions {
        terminal_get_dimensions(&mut self.terminal.lock())
    }

    fn get_title(&self) -> String {
        let state = self.state.lock();
        format!("FS Explorer - {}", state.current_dir.display())
    }

    fn can_close_without_prompting(&self, _reason: CloseReason) -> bool {
        true
    }

    fn send_paste(&self, _text: &str) -> Result<()> {
        // File explorer doesn't support paste
        Ok(())
    }

    fn reader(&self) -> Result<Option<Box<dyn std::io::Read + Send>>> {
        Ok(Some(Box::new(self.render_rx.try_clone()?)))
    }

    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        MutexGuard::map(self.writer.lock(), |writer| {
            let w: &mut dyn std::io::Write = writer;
            w
        })
    }

    fn resize(&self, size: TerminalSize) -> Result<()> {
        self.input_tx.send(FsExplorerInput::Resize {
            rows: size.rows,
            cols: size.cols,
        })?;

        self.terminal.lock().resize(size);
        self.render_to_terminal();

        Ok(())
    }

    fn key_down(&self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        if let Err(e) = self.handle_key(key, modifiers) {
            *self.dead.lock() = true;
            return Err(e);
        }
        Ok(())
    }

    fn key_up(&self, _key: KeyCode, _modifiers: KeyModifiers) -> Result<()> {
        Ok(())
    }

    fn mouse_event(&self, event: MouseEvent) -> Result<()> {
        if let Err(e) = self.input_tx.send(FsExplorerInput::Mouse(event)) {
            *self.dead.lock() = true;
            return Err(e.into());
        }
        Ok(())
    }

    fn set_config(&self, config: Arc<dyn TerminalConfiguration>) {
        self.terminal.lock().set_config(config);
    }

    fn get_config(&self) -> Option<Arc<dyn TerminalConfiguration>> {
        Some(self.terminal.lock().get_config())
    }

    fn perform_actions(&self, actions: Vec<termwiz::escape::Action>) {
        self.terminal.lock().perform_actions(actions)
    }

    fn kill(&self) {
        *self.dead.lock() = true;
    }

    fn is_dead(&self) -> bool {
        *self.dead.lock()
    }

    fn palette(&self) -> ColorPalette {
        self.terminal.lock().palette()
    }

    fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    fn is_mouse_grabbed(&self) -> bool {
        self.terminal.lock().is_mouse_grabbed()
    }

    fn is_alt_screen_active(&self) -> bool {
        self.terminal.lock().is_alt_screen_active()
    }

    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
        let state = self.state.lock();
        Url::from_directory_path(&state.current_dir).ok()
    }

    fn erase_scrollback(&self, erase_mode: ScrollbackEraseMode) {
        match erase_mode {
            ScrollbackEraseMode::ScrollbackOnly => {
                self.terminal.lock().erase_scrollback();
            }
            ScrollbackEraseMode::ScrollbackAndViewport => {
                self.terminal.lock().erase_scrollback_and_viewport();
            }
        }
    }
}

/// Format a file size for display
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Allocate a new FsExplorerPane and return both the pane and the input receiver
pub fn allocate_fs_explorer_pane(
    domain_id: DomainId,
    size: TerminalSize,
    start_dir: PathBuf,
    term_config: Option<Arc<dyn TerminalConfiguration + Send + Sync>>,
) -> Result<(Receiver<FsExplorerInput>, Arc<dyn Pane>)> {
    let render_pipe = Pipe::new().map_err(|e| anyhow!("Failed to create render pipe: {}", e))?;
    let (input_tx, input_rx) = channel();

    let pane = FsExplorerPane::new(
        domain_id,
        size,
        start_dir,
        input_tx,
        render_pipe.read,
        term_config,
    )?;

    // Perform initial render
    pane.render_to_terminal();

    Ok((input_rx, Arc::new(pane)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_fs_explorer_state_new() {
        let temp_dir = std::env::temp_dir();
        let state = FsExplorerState::new(temp_dir);
        assert!(state.is_ok());
    }
}
