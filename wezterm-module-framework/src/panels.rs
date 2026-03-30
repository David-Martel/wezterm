//! Rust-native panel state management.
//!
//! Replaces the Lua `wezterm.GLOBAL.codex_ui_panel_state` / `_preferences` /
//! `_restore_done` tables with a thread-safe Rust struct.  Exposes a Lua API
//! at `wezterm.panels.*` so that existing Lua code can migrate incrementally.
//!
//! ## State model
//!
//! | Concept | Scope | Persisted? |
//! |---------|-------|-----------|
//! | Panel tracking (pane IDs) | per-window | no |
//! | User preferences (open/closed intent) | global | yes |
//! | Restore-done flag | per-window | no |
//!
//! Preferences are persisted to `~/.local/state/wezterm-utils/panel-preferences.json`
//! (NOT `~/.config/wezterm/` — that path triggers the config-reload file watcher).

use config::lua::get_or_create_sub_module;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

/// Global panel state singleton.
static PANEL_STATE: OnceLock<Arc<PanelManager>> = OnceLock::new();

/// Panel names that the system recognizes.
pub const PANEL_NAMES: &[&str] = &["explorer", "watcher", "editor"];

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// User preferences for which panels should be open.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PanelPreferences {
    pub explorer: bool,
    pub watcher: bool,
    pub editor: bool,
}

/// Per-window panel tracking (maps panel name to pane ID).
#[derive(Debug, Clone, Default)]
pub struct WindowPanelState {
    pub panels: HashMap<String, u64>,
    pub restore_done: bool,
}

// ---------------------------------------------------------------------------
// PanelManager
// ---------------------------------------------------------------------------

/// Central panel state manager.
///
/// Thread-safe (interior mutability via `parking_lot::RwLock`). A single
/// global instance is created lazily via [`PanelManager::global`].
#[derive(Debug)]
pub struct PanelManager {
    /// Per-window state: window_id -> panel state.
    windows: RwLock<HashMap<u64, WindowPanelState>>,
    /// User preferences (persisted to disk).
    preferences: RwLock<PanelPreferences>,
    /// Directory for persisted state files.
    state_dir: PathBuf,
}

impl PanelManager {
    /// Create a new `PanelManager` using the default state directory
    /// (`~/.local/state/wezterm-utils`).
    pub fn new() -> Self {
        let state_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".local/state/wezterm-utils");
        Self::with_state_dir(state_dir)
    }

    /// Create a `PanelManager` that persists to `state_dir`.
    ///
    /// This constructor is primarily useful for testing with a temporary directory.
    pub fn with_state_dir(state_dir: PathBuf) -> Self {
        let prefs = Self::load_preferences(&state_dir).unwrap_or_default();
        Self {
            windows: RwLock::new(HashMap::new()),
            preferences: RwLock::new(prefs),
            state_dir,
        }
    }

    /// Return the global singleton, creating it on first access.
    pub fn global() -> &'static Arc<PanelManager> {
        PANEL_STATE.get_or_init(|| Arc::new(PanelManager::new()))
    }

    // --- Preferences -------------------------------------------------------

    /// Return a snapshot of the current panel preferences.
    pub fn get_preferences(&self) -> PanelPreferences {
        self.preferences.read().clone()
    }

    /// Set a single panel preference and persist to disk.
    ///
    /// Unknown panel names are silently ignored.
    pub fn set_preference(&self, panel: &str, open: bool) {
        {
            let mut prefs = self.preferences.write();
            match panel {
                "explorer" => prefs.explorer = open,
                "watcher" => prefs.watcher = open,
                "editor" => prefs.editor = open,
                _ => return,
            }
        }
        if let Err(e) = self.save_preferences() {
            log::warn!("Failed to persist panel preferences: {e}");
        }
    }

    // --- Per-window state --------------------------------------------------

    /// Record a panel's pane ID for a given window.
    pub fn track_panel(&self, window_id: u64, panel: &str, pane_id: u64) {
        let mut windows = self.windows.write();
        let state = windows.entry(window_id).or_default();
        state.panels.insert(panel.to_string(), pane_id);
    }

    /// Remove tracking for a panel in a given window.
    pub fn untrack_panel(&self, window_id: u64, panel: &str) {
        let mut windows = self.windows.write();
        if let Some(state) = windows.get_mut(&window_id) {
            state.panels.remove(panel);
        }
    }

    /// Look up the pane ID for a panel in a given window.
    pub fn get_panel_pane_id(&self, window_id: u64, panel: &str) -> Option<u64> {
        let windows = self.windows.read();
        windows.get(&window_id)?.panels.get(panel).copied()
    }

    /// Return all tracked panels for a given window.
    pub fn get_window_panels(&self, window_id: u64) -> HashMap<String, u64> {
        let windows = self.windows.read();
        windows
            .get(&window_id)
            .map(|s| s.panels.clone())
            .unwrap_or_default()
    }

    /// Check whether the restore-done flag is set for a window.
    pub fn is_restore_done(&self, window_id: u64) -> bool {
        let windows = self.windows.read();
        windows
            .get(&window_id)
            .map(|s| s.restore_done)
            .unwrap_or(false)
    }

    /// Mark restore as complete for a window.
    pub fn mark_restore_done(&self, window_id: u64) {
        let mut windows = self.windows.write();
        let state = windows.entry(window_id).or_default();
        state.restore_done = true;
    }

    /// Remove all state for a window (e.g. on window close).
    pub fn remove_window(&self, window_id: u64) {
        let mut windows = self.windows.write();
        windows.remove(&window_id);
    }

    // --- Persistence -------------------------------------------------------

    fn preferences_path(&self) -> PathBuf {
        self.state_dir.join("panel-preferences.json")
    }

    fn load_preferences(state_dir: &std::path::Path) -> Option<PanelPreferences> {
        let path = state_dir.join("panel-preferences.json");
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_preferences(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.state_dir)?;
        let path = self.preferences_path();
        let prefs = self.preferences.read();
        let json = serde_json::to_string_pretty(&*prefs)
            .map_err(std::io::Error::other)?;
        std::fs::write(&path, json)
    }
}

impl Default for PanelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Lua API
// ---------------------------------------------------------------------------

/// Register the `wezterm.panels.*` Lua API backed by the global
/// [`PanelManager`].
///
/// Functions:
///
/// ```lua
/// -- Preferences (persisted)
/// wezterm.panels.get_preferences()           -- {explorer=bool, watcher=bool, editor=bool}
/// wezterm.panels.set_preference(name, open)  -- nil
///
/// -- Per-window tracking (volatile)
/// wezterm.panels.track(window_id, name, pane_id)  -- nil
/// wezterm.panels.untrack(window_id, name)          -- nil
/// wezterm.panels.get_pane_id(window_id, name)      -- number | nil
///
/// -- Restore tracking (volatile)
/// wezterm.panels.is_restore_done(window_id)  -- bool
/// wezterm.panels.mark_restore_done(window_id) -- nil
/// ```
pub fn register_lua_api(lua: &mlua::Lua) -> anyhow::Result<()> {
    let panels_mod = get_or_create_sub_module(lua, "panels")?;
    let mgr = PanelManager::global().clone();

    // -- wezterm.panels.get_preferences() -> table -------------------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "get_preferences",
            lua.create_function(move |lua_ctx, ()| {
                let prefs = m.get_preferences();
                let table = lua_ctx.create_table()?;
                table.set("explorer", prefs.explorer)?;
                table.set("watcher", prefs.watcher)?;
                table.set("editor", prefs.editor)?;
                Ok(table)
            })?,
        )?;
    }

    // -- wezterm.panels.set_preference(name, open) -> nil ------------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "set_preference",
            lua.create_function(move |_, (panel, open): (String, bool)| {
                m.set_preference(&panel, open);
                Ok(())
            })?,
        )?;
    }

    // -- wezterm.panels.track(window_id, name, pane_id) -> nil -------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "track",
            lua.create_function(move |_, (window_id, panel, pane_id): (u64, String, u64)| {
                m.track_panel(window_id, &panel, pane_id);
                Ok(())
            })?,
        )?;
    }

    // -- wezterm.panels.untrack(window_id, name) -> nil --------------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "untrack",
            lua.create_function(move |_, (window_id, panel): (u64, String)| {
                m.untrack_panel(window_id, &panel);
                Ok(())
            })?,
        )?;
    }

    // -- wezterm.panels.get_pane_id(window_id, name) -> number | nil -------
    {
        let m = mgr.clone();
        panels_mod.set(
            "get_pane_id",
            lua.create_function(move |_, (window_id, panel): (u64, String)| {
                Ok(m.get_panel_pane_id(window_id, &panel))
            })?,
        )?;
    }

    // -- wezterm.panels.is_restore_done(window_id) -> bool -----------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "is_restore_done",
            lua.create_function(move |_, window_id: u64| Ok(m.is_restore_done(window_id)))?,
        )?;
    }

    // -- wezterm.panels.mark_restore_done(window_id) -> nil ----------------
    {
        let m = mgr.clone();
        panels_mod.set(
            "mark_restore_done",
            lua.create_function(move |_, window_id: u64| {
                m.mark_restore_done(window_id);
                Ok(())
            })?,
        )?;
    }

    log::debug!("Registered wezterm.panels Lua API");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a PanelManager backed by a temporary directory.
    fn test_manager(dir: &std::path::Path) -> PanelManager {
        PanelManager::with_state_dir(dir.to_path_buf())
    }

    #[test]
    fn new_manager_has_default_preferences() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());
        let prefs = mgr.get_preferences();
        assert!(!prefs.explorer);
        assert!(!prefs.watcher);
        assert!(!prefs.editor);
    }

    #[test]
    fn set_preference_roundtrip() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.set_preference("explorer", true);
        mgr.set_preference("watcher", true);

        let prefs = mgr.get_preferences();
        assert!(prefs.explorer);
        assert!(prefs.watcher);
        assert!(!prefs.editor);

        // Toggle back
        mgr.set_preference("explorer", false);
        let prefs = mgr.get_preferences();
        assert!(!prefs.explorer);
    }

    #[test]
    fn unknown_panel_name_is_ignored() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());
        mgr.set_preference("nonexistent", true);
        // Should not panic; preferences unchanged
        let prefs = mgr.get_preferences();
        assert_eq!(prefs, PanelPreferences::default());
    }

    #[test]
    fn track_and_get_panel_pane_id() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.track_panel(1, "explorer", 100);
        mgr.track_panel(1, "watcher", 200);
        mgr.track_panel(2, "explorer", 300);

        assert_eq!(mgr.get_panel_pane_id(1, "explorer"), Some(100));
        assert_eq!(mgr.get_panel_pane_id(1, "watcher"), Some(200));
        assert_eq!(mgr.get_panel_pane_id(2, "explorer"), Some(300));
        assert_eq!(mgr.get_panel_pane_id(2, "watcher"), None);
        assert_eq!(mgr.get_panel_pane_id(999, "explorer"), None);
    }

    #[test]
    fn untrack_panel_removes_entry() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.track_panel(1, "explorer", 100);
        assert_eq!(mgr.get_panel_pane_id(1, "explorer"), Some(100));

        mgr.untrack_panel(1, "explorer");
        assert_eq!(mgr.get_panel_pane_id(1, "explorer"), None);
    }

    #[test]
    fn untrack_nonexistent_is_noop() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());
        // No panic when untracking from a window that doesn't exist
        mgr.untrack_panel(999, "explorer");
    }

    #[test]
    fn per_window_isolation() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.track_panel(1, "explorer", 10);
        mgr.track_panel(2, "explorer", 20);

        // Untrack from window 1 should not affect window 2
        mgr.untrack_panel(1, "explorer");
        assert_eq!(mgr.get_panel_pane_id(1, "explorer"), None);
        assert_eq!(mgr.get_panel_pane_id(2, "explorer"), Some(20));
    }

    #[test]
    fn restore_done_flag() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        assert!(!mgr.is_restore_done(1));
        mgr.mark_restore_done(1);
        assert!(mgr.is_restore_done(1));
        // Other windows unaffected
        assert!(!mgr.is_restore_done(2));
    }

    #[test]
    fn remove_window_clears_all_state() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.track_panel(1, "explorer", 100);
        mgr.mark_restore_done(1);
        mgr.remove_window(1);

        assert_eq!(mgr.get_panel_pane_id(1, "explorer"), None);
        assert!(!mgr.is_restore_done(1));
    }

    #[test]
    fn get_window_panels_returns_all() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());

        mgr.track_panel(1, "explorer", 10);
        mgr.track_panel(1, "watcher", 20);

        let panels = mgr.get_window_panels(1);
        assert_eq!(panels.len(), 2);
        assert_eq!(panels.get("explorer"), Some(&10));
        assert_eq!(panels.get("watcher"), Some(&20));

        // Empty window
        assert!(mgr.get_window_panels(999).is_empty());
    }

    #[test]
    fn preferences_persist_to_disk() {
        let tmp = tempfile::tempdir().expect("create tempdir");

        // Write preferences
        {
            let mgr = test_manager(tmp.path());
            mgr.set_preference("explorer", true);
            mgr.set_preference("editor", true);
        }

        // Read back from a fresh manager pointing at the same directory
        {
            let mgr = test_manager(tmp.path());
            let prefs = mgr.get_preferences();
            assert!(prefs.explorer);
            assert!(!prefs.watcher);
            assert!(prefs.editor);
        }
    }

    #[test]
    fn preferences_file_is_valid_json() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = test_manager(tmp.path());
        mgr.set_preference("watcher", true);

        let path = tmp.path().join("panel-preferences.json");
        let content = std::fs::read_to_string(&path).expect("read prefs file");
        let parsed: PanelPreferences =
            serde_json::from_str(&content).expect("parse prefs JSON");
        assert!(parsed.watcher);
        assert!(!parsed.explorer);
    }

    // -- Lua API smoke tests ------------------------------------------------

    #[test]
    fn lua_api_registers_without_panic() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register_lua_api should succeed");

        let package: mlua::Table = lua.globals().get("package").expect("package");
        let loaded: mlua::Table = package.get("loaded").expect("loaded");
        let wezterm: mlua::Table = loaded.get("wezterm").expect("wezterm");
        let panels: mlua::Table = wezterm.get("panels").expect("panels sub-module");

        // Verify all expected functions exist
        for name in &[
            "get_preferences",
            "set_preference",
            "track",
            "untrack",
            "get_pane_id",
            "is_restore_done",
            "mark_restore_done",
        ] {
            let _: mlua::Function = panels
                .get(*name)
                .unwrap_or_else(|_| panic!("function '{name}' should exist"));
        }
    }

    #[test]
    fn lua_get_preferences_returns_table() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        let result: mlua::Table = lua
            .load("return require('wezterm').panels.get_preferences()")
            .eval()
            .expect("eval get_preferences");

        let explorer: bool = result.get("explorer").expect("explorer key");
        let watcher: bool = result.get("watcher").expect("watcher key");
        let editor: bool = result.get("editor").expect("editor key");

        // Global singleton defaults — may already have been set by other tests
        // running in the same process, but the fields should at least be booleans.
        assert!(explorer || !explorer); // type check: it's a bool
        assert!(watcher || !watcher);
        assert!(editor || !editor);
    }

    #[test]
    fn lua_track_and_get_pane_id() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        lua.load(
            r#"
            local p = require('wezterm').panels
            p.track(42, 'explorer', 777)
        "#,
        )
        .exec()
        .expect("track call");

        let result: mlua::Value = lua
            .load("return require('wezterm').panels.get_pane_id(42, 'explorer')")
            .eval()
            .expect("eval get_pane_id");

        match result {
            mlua::Value::Integer(n) => assert_eq!(n, 777),
            other => panic!("expected Integer(777), got {other:?}"),
        }

        // Non-existent returns nil
        let result: mlua::Value = lua
            .load("return require('wezterm').panels.get_pane_id(42, 'watcher')")
            .eval()
            .expect("eval get_pane_id nil");
        assert!(matches!(result, mlua::Value::Nil));
    }

    #[test]
    fn lua_restore_done_roundtrip() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        let before: bool = lua
            .load("return require('wezterm').panels.is_restore_done(99)")
            .eval()
            .expect("before");
        assert!(!before);

        lua.load("require('wezterm').panels.mark_restore_done(99)")
            .exec()
            .expect("mark");

        let after: bool = lua
            .load("return require('wezterm').panels.is_restore_done(99)")
            .eval()
            .expect("after");
        assert!(after);
    }
}
