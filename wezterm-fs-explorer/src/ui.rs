use crate::app::{App, AppMode, ConfirmationMode, InputMode};
use crate::file_entry::FileType;
use crate::icons::Icons;
use crate::keybindings::KeyBindings;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.size());

    draw_title_bar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    // Draw help overlay on top of everything when in Help mode
    if app.mode == AppMode::Help {
        draw_help_overlay(f);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let title = format!("  WezTerm File Explorer - {}", app.current_dir.display());
    let title_widget = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(title_widget, area);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    if app.show_preview {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        draw_file_list(f, app, chunks[0]);
        draw_preview_pane(f, app, chunks[1]);
    } else {
        draw_file_list(f, app, area);
    }
}

fn draw_file_list(f: &mut Frame, app: &App, area: Rect) {
    let visible_entries = app.visible_entries();

    let items: Vec<ListItem> = visible_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let icon = Icons::get_icon(entry);
            let color = Icons::get_color(entry);

            let git_indicator = app
                .git_status
                .as_ref()
                .and_then(|gs| gs.get_indicator(&entry.path))
                .unwrap_or(" ");

            let selection_marker = if app.selected_entries.contains(&idx) {
                "✓"
            } else {
                " "
            };

            let size = if entry.file_type == FileType::Directory {
                String::from("<DIR>")
            } else {
                entry.format_size()
            };

            let content = format!(
                "{} {} {} {:>10}  {}",
                selection_marker, git_indicator, icon, size, entry.name
            );

            let mut style = Style::default().fg(color);

            if idx == app.selected_index {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Files")
            .style(Style::default()),
    );

    f.render_widget(list, area);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let preview_text = if let Some(entry) = app.current_entry() {
        let mut lines = vec![
            format!("Name: {}", entry.name),
            format!("Type: {:?}", entry.file_type),
            format!("Size: {}", entry.format_size()),
            format!("Modified: {}", entry.format_modified()),
            format!("Permissions: {}", entry.permissions),
        ];

        if let Some(ext) = entry.extension() {
            lines.push(format!("Extension: {}", ext));
        }

        if entry.file_type == FileType::File && entry.size < 1024 * 100 {
            // Preview small text files
            if let Ok(content) = std::fs::read_to_string(&entry.path) {
                lines.push(String::new());
                lines.push("Content Preview:".to_string());
                lines.push("─".repeat(40));
                lines.extend(content.lines().take(20).map(|l| l.to_string()));
            }
        } else if entry.file_type == FileType::Directory {
            if let Ok(entries) = std::fs::read_dir(&entry.path) {
                let count = entries.count();
                lines.push(format!("Items: {}", count));
            }
        }

        lines.join("\n")
    } else {
        String::from("No file selected")
    };

    let preview = Paragraph::new(preview_text)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .wrap(Wrap { trim: true });

    f.render_widget(preview, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.mode {
        AppMode::Normal => {
            let help_hint = "Press ? for help, q to quit";
            let entry_count = format!("{} items", app.entries.len());
            let selected_count = if !app.selected_entries.is_empty() {
                format!(" | {} selected", app.selected_entries.len())
            } else {
                String::new()
            };
            format!("{} {} | {}", entry_count, selected_count, help_hint)
        }
        AppMode::Search => {
            format!("Search: {}_", app.search_query)
        }
        AppMode::Input(InputMode::Rename) => {
            format!("Rename to: {}_", app.input_buffer)
        }
        AppMode::Input(InputMode::New) => {
            format!("New file/dir (end with / for dir): {}_", app.input_buffer)
        }
        AppMode::Input(InputMode::Copy) => {
            format!("Copy to: {}_", app.input_buffer)
        }
        AppMode::Input(InputMode::Move) => {
            format!("Move to: {}_", app.input_buffer)
        }
        AppMode::Confirmation(ConfirmationMode::Delete) => String::from("Delete selected? (y/n)"),
        AppMode::Help => String::from("Press any key to close help"),
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(status, area);

    // Show error message if present
    if let Some(ref error) = app.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Error"));

        // Center the error dialog
        let error_area = centered_rect(60, 20, f.size());
        f.render_widget(error_widget, error_area);
    }
}

fn draw_help_overlay(f: &mut Frame) {
    let area = centered_rect(60, 80, f.size());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let bindings = KeyBindings::get_help_text();
    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("{:12}", key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(desc.to_string()),
            ])
        })
        .collect();

    let help_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Keyboard Shortcuts (press any key to close) ")
                .style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(help_widget, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
