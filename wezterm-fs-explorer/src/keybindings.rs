/// Keyboard shortcut definitions for the help overlay.
///
/// Provides a central registry of key bindings displayed when the user
/// presses `?` in Normal mode.
pub struct KeyBindings;

impl KeyBindings {
    pub fn get_help_text() -> Vec<(&'static str, &'static str)> {
        vec![
            ("j/\u{2193}", "Move down"),
            ("k/\u{2191}", "Move up"),
            ("h/\u{2190}", "Go to parent directory"),
            ("l/\u{2192}", "Enter directory"),
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
            ("?", "Show this help"),
            ("q/Esc", "Quit"),
            ("Ctrl+c", "Force quit"),
        ]
    }
}
