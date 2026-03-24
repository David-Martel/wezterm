#[allow(dead_code)]
pub struct KeyBindings;

#[allow(dead_code)]
impl KeyBindings {
    pub fn get_help_text() -> Vec<(&'static str, &'static str)> {
        vec![
            ("j/↓", "Move down"),
            ("k/↑", "Move up"),
            ("h/←", "Go to parent directory"),
            ("l/→", "Enter directory"),
            ("g", "Go to top"),
            ("G", "Go to bottom"),
            ("/", "Search/filter"),
            ("Space", "Select/multi-select"),
            ("Enter", "Open file/directory"),
            ("d", "Delete (with confirmation)"),
            ("r", "Rename"),
            ("c", "Copy"),
            ("m", "Move"),
            ("n", "New file/directory"),
            (".", "Toggle hidden files"),
            ("Tab", "Toggle preview pane"),
            ("q/Esc", "Quit"),
            ("Ctrl+c", "Force quit"),
        ]
    }

    pub fn format_key_binding(key: &str, description: &str) -> String {
        format!("{:12} {}", key, description)
    }
}
