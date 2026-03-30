use crate::file_entry::{FileEntry, FileType};
use crate::git_status::GitStatus;
use crate::operations::FileOperation;
use crate::search::FuzzySearch;
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Help,
    Input(InputMode),
    Confirmation(ConfirmationMode),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Rename,
    New,
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmationMode {
    Delete,
}

pub struct App {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub selected_entries: Vec<usize>,
    pub mode: AppMode,
    pub search_query: String,
    pub input_buffer: String,
    pub show_hidden: bool,
    pub show_preview: bool,
    pub git_status: Option<GitStatus>,
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    fuzzy_search: FuzzySearch,
    filtered_entries: Vec<usize>,
}

impl App {
    pub fn new(start_dir: PathBuf) -> Result<Self> {
        let mut app = Self {
            current_dir: start_dir.clone(),
            entries: Vec::new(),
            selected_index: 0,
            selected_entries: Vec::new(),
            mode: AppMode::Normal,
            search_query: String::new(),
            input_buffer: String::new(),
            show_hidden: false,
            show_preview: false,
            git_status: GitStatus::from_repo(&start_dir),
            scroll_offset: 0,
            error_message: None,
            fuzzy_search: FuzzySearch::new(),
            filtered_entries: Vec::new(),
        };

        app.load_directory()?;
        Ok(app)
    }

    pub fn load_directory(&mut self) -> Result<()> {
        self.entries = FileEntry::read_directory(&self.current_dir, self.show_hidden)?;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.git_status = GitStatus::from_repo(&self.current_dir);

        // Update fuzzy search index
        self.update_search_index();

        Ok(())
    }

    pub fn refresh_entries(&mut self) -> Result<()> {
        let current_selection = self.current_entry().map(|e| e.path.clone());
        self.entries = FileEntry::read_directory(&self.current_dir, self.show_hidden)?;

        // Try to restore selection
        if let Some(selected_path) = current_selection {
            if let Some(index) = self.entries.iter().position(|e| e.path == selected_path) {
                self.selected_index = index;
            }
        }

        self.git_status = GitStatus::from_repo(&self.current_dir);

        // Update fuzzy search index
        self.update_search_index();

        Ok(())
    }

    pub fn move_down(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn go_top(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn go_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            let _ = self.load_directory();
        }
    }

    pub fn enter_directory(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let entry = &self.entries[self.selected_index];
        if entry.file_type == FileType::Directory {
            self.current_dir = entry.path.clone();
            self.load_directory()?;
        }

        Ok(())
    }

    pub fn toggle_selection(&mut self) {
        if let Some(pos) = self
            .selected_entries
            .iter()
            .position(|&i| i == self.selected_index)
        {
            self.selected_entries.remove(pos);
        } else {
            self.selected_entries.push(self.selected_index);
        }
    }

    pub fn toggle_hidden_files(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.load_directory()
    }

    pub fn toggle_preview_pane(&mut self) {
        self.show_preview = !self.show_preview;
    }

    pub fn show_help(&mut self) {
        self.mode = AppMode::Help;
    }

    pub fn hide_help(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn start_search(&mut self) {
        self.mode = AppMode::Search;
        self.search_query.clear();
        self.filtered_entries.clear();
    }

    pub fn exit_search(&mut self) {
        self.mode = AppMode::Normal;
        self.search_query.clear();
        self.filtered_entries.clear();
        self.selected_index = 0;
    }

    pub fn start_delete_mode(&mut self) {
        if !self.entries.is_empty() {
            self.mode = AppMode::Confirmation(ConfirmationMode::Delete);
        }
    }

    pub fn start_rename_mode(&mut self) {
        if !self.entries.is_empty() {
            self.mode = AppMode::Input(InputMode::Rename);
            self.input_buffer = self.entries[self.selected_index].name.clone();
        }
    }

    pub fn start_copy_mode(&mut self) {
        if !self.entries.is_empty() {
            self.mode = AppMode::Input(InputMode::Copy);
            self.input_buffer.clear();
        }
    }

    pub fn start_move_mode(&mut self) {
        if !self.entries.is_empty() {
            self.mode = AppMode::Input(InputMode::Move);
            self.input_buffer.clear();
        }
    }

    pub fn start_new_mode(&mut self) {
        self.mode = AppMode::Input(InputMode::New);
        self.input_buffer.clear();
    }

    pub fn is_confirmation_mode(&self) -> bool {
        matches!(self.mode, AppMode::Confirmation(_))
    }

    pub fn is_input_mode(&self) -> bool {
        matches!(self.mode, AppMode::Input(_))
    }

    pub fn handle_input(&mut self, c: char) {
        if matches!(self.mode, AppMode::Search) {
            self.search_query.push(c);
            self.update_fuzzy_search();
        } else if self.is_input_mode() {
            self.input_buffer.push(c);
        }
    }

    pub fn backspace_input(&mut self) {
        if matches!(self.mode, AppMode::Search) {
            self.search_query.pop();
            self.update_fuzzy_search();
        } else if self.is_input_mode() {
            self.input_buffer.pop();
        }
    }

    pub fn confirm_action(&mut self) -> Result<()> {
        match self.mode {
            AppMode::Confirmation(ConfirmationMode::Delete) => {
                self.delete_selected()?;
            }
            AppMode::Input(InputMode::Rename) => {
                self.rename_selected()?;
            }
            AppMode::Input(InputMode::New) => {
                self.create_new()?;
            }
            AppMode::Input(InputMode::Copy) => {
                self.copy_selected()?;
            }
            AppMode::Input(InputMode::Move) => {
                self.move_selected()?;
            }
            _ => {}
        }

        self.mode = AppMode::Normal;
        self.input_buffer.clear();
        Ok(())
    }

    fn delete_selected(&mut self) -> Result<()> {
        let indices = if self.selected_entries.is_empty() {
            vec![self.selected_index]
        } else {
            self.selected_entries.clone()
        };

        for &idx in indices.iter().rev() {
            if idx < self.entries.len() {
                FileOperation::delete(&self.entries[idx].path)?;
            }
        }

        self.selected_entries.clear();
        self.load_directory()?;
        Ok(())
    }

    fn rename_selected(&mut self) -> Result<()> {
        if !self.entries.is_empty() && !self.input_buffer.is_empty() {
            let old_path = &self.entries[self.selected_index].path;
            let parent = old_path
                .parent()
                .with_context(|| format!("Cannot rename root path {}", old_path.display()))?;
            let new_path = parent.join(&self.input_buffer);
            FileOperation::rename(old_path, &new_path)?;
            self.load_directory()?;
        }
        Ok(())
    }

    fn create_new(&mut self) -> Result<()> {
        if !self.input_buffer.is_empty() {
            let new_path = self.current_dir.join(&self.input_buffer);
            if self.input_buffer.ends_with('/') {
                FileOperation::create_directory(&new_path)?;
            } else {
                FileOperation::create_file(&new_path)?;
            }
            self.load_directory()?;
        }
        Ok(())
    }

    fn copy_selected(&mut self) -> Result<()> {
        if !self.entries.is_empty() && !self.input_buffer.is_empty() {
            let source = &self.entries[self.selected_index].path;
            let dest = self.current_dir.join(&self.input_buffer);
            FileOperation::copy(source, &dest)?;
            self.load_directory()?;
        }
        Ok(())
    }

    fn move_selected(&mut self) -> Result<()> {
        if !self.entries.is_empty() && !self.input_buffer.is_empty() {
            let source = &self.entries[self.selected_index].path;
            let dest = self.current_dir.join(&self.input_buffer);
            FileOperation::rename(source, &dest)?;
            self.load_directory()?;
        }
        Ok(())
    }

    pub fn get_selected_paths(&self) -> Option<Vec<PathBuf>> {
        if self.entries.is_empty() {
            return None;
        }

        if self.selected_entries.is_empty() {
            Some(vec![self.entries[self.selected_index].path.clone()])
        } else {
            Some(
                self.selected_entries
                    .iter()
                    .filter_map(|&idx| self.entries.get(idx).map(|e| e.path.clone()))
                    .collect(),
            )
        }
    }

    pub fn update(&mut self) -> Result<()> {
        // Update logic (e.g., watch for file system changes)
        Ok(())
    }

    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        if self.search_query.is_empty() {
            self.entries.iter().collect()
        } else if matches!(self.mode, AppMode::Search) {
            // Use fuzzy search filtered results
            self.filtered_entries
                .iter()
                .filter_map(|&idx| self.entries.get(idx))
                .collect()
        } else {
            // Fallback to simple substring search
            self.entries
                .iter()
                .filter(|e| {
                    e.name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                })
                .collect()
        }
    }

    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    /// Update the fuzzy search index with current entries
    fn update_search_index(&mut self) {
        self.fuzzy_search.clear();
        let paths: Vec<PathBuf> = self.entries.iter().map(|e| e.path.clone()).collect();
        self.fuzzy_search.populate(paths);
    }

    /// Perform fuzzy search and update filtered entries
    fn update_fuzzy_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_entries.clear();
            return;
        }

        let results = self.fuzzy_search.search(&self.search_query, 1000);

        // Map results back to entry indices
        self.filtered_entries.clear();
        for result in results {
            if let Some(idx) = self.entries.iter().position(|e| e.path == result.path) {
                self.filtered_entries.push(idx);
            }
        }

        // Reset selection when search results change
        self.selected_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn rename_selected_without_parent_returns_error() {
        let root_path = if cfg!(windows) {
            PathBuf::from(r"C:\")
        } else {
            PathBuf::from("/")
        };

        let mut app = App {
            current_dir: PathBuf::from("."),
            entries: vec![FileEntry {
                path: root_path,
                name: "root".to_string(),
                file_type: FileType::Directory,
                size: 0,
                modified: SystemTime::now(),
                permissions: "rw-".to_string(),
                is_hidden: false,
            }],
            selected_index: 0,
            selected_entries: Vec::new(),
            mode: AppMode::Input(InputMode::Rename),
            search_query: String::new(),
            input_buffer: "renamed".to_string(),
            show_hidden: false,
            show_preview: false,
            git_status: None,
            scroll_offset: 0,
            error_message: None,
            fuzzy_search: FuzzySearch::new(),
            filtered_entries: Vec::new(),
        };

        let error = app
            .rename_selected()
            .expect_err("rename should fail gracefully for parentless paths");

        assert!(error.to_string().contains("Cannot rename root path"));
    }
}
