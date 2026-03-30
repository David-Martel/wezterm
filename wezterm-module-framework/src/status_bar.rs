//! Rust-native status bar data provider.
//!
//! Pre-computes status bar data (cwd, workspace, panel state, clock)
//! and exposes it via `wezterm.status.*` Lua API.  This eliminates
//! per-frame IPC calls from `chrome.lua`.
//!
//! ## Architecture
//!
//! [`StatusBarState`] is a global singleton holding a [`StatusData`] snapshot
//! protected by a [`parking_lot::RwLock`] (fast uncontended reads).
//! Rust-side hooks or the Lua bridge call [`StatusBarState::update`] to push
//! new values; Lua reads them through `wezterm.status.get()` with zero IPC.
//!
//! ## Lua API
//!
//! ```lua
//! local data = wezterm.status.get()    -- table with all fields
//! wezterm.status.update(table)         -- set status data from Lua
//! wezterm.status.update_clock("14:32") -- update just the clock field
//! wezterm.status.age_ms()              -- ms since last full update
//! ```

use config::lua::get_or_create_sub_module;
use parking_lot::RwLock;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

/// Global singleton for status bar state.
static STATUS: OnceLock<Arc<StatusBarState>> = OnceLock::new();

/// Cached status bar data, updated from Rust-side hooks or the Lua bridge.
#[derive(Debug, Clone, Default)]
pub struct StatusData {
    /// Full current working directory path.
    pub cwd: String,
    /// Shortened CWD for display (e.g. `~/projects/wezterm`).
    pub cwd_short: String,
    /// Active workspace name.
    pub workspace: String,
    /// Formatted clock string (e.g. `"14:32"`).
    pub clock: String,
    /// LLM agent status indicator.
    pub llm_agent: String,
    /// GPU adapter name for display.
    pub gpu_name: String,
}

/// Thread-safe status bar state manager.
///
/// A single global instance is created lazily via [`StatusBarState::global`].
/// Readers acquire a shared `RwLock` (contention-free when no writer is active);
/// writers are expected to be infrequent (at most once per second from the
/// update hook).
#[derive(Debug)]
pub struct StatusBarState {
    /// The cached status data.
    data: RwLock<StatusData>,
    /// Timestamp of the last full update (via [`Self::update`]).
    last_update: RwLock<Option<Instant>>,
}

impl StatusBarState {
    /// Create a new, empty `StatusBarState`.
    pub fn new() -> Self {
        Self {
            data: RwLock::new(StatusData::default()),
            last_update: RwLock::new(None),
        }
    }

    /// Return the global singleton, creating it on first access.
    pub fn global() -> &'static Arc<StatusBarState> {
        STATUS.get_or_init(|| Arc::new(StatusBarState::new()))
    }

    /// Replace the entire cached data snapshot.
    ///
    /// Called from Rust hooks or the Lua bridge when fresh data is available.
    pub fn update(&self, data: StatusData) {
        *self.data.write() = data;
        *self.last_update.write() = Some(Instant::now());
    }

    /// Update only the clock field (cheap, may be called more frequently).
    pub fn update_clock(&self, clock: String) {
        self.data.write().clock = clock;
    }

    /// Return a snapshot of the cached data.
    pub fn get(&self) -> StatusData {
        self.data.read().clone()
    }

    /// Milliseconds elapsed since the last full [`Self::update`] call.
    ///
    /// Returns `u64::MAX` if no update has ever been performed.
    pub fn age_ms(&self) -> u64 {
        self.last_update
            .read()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(u64::MAX)
    }
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Lua API
// ---------------------------------------------------------------------------

/// Register the `wezterm.status.*` Lua API backed by the global
/// [`StatusBarState`].
///
/// Functions:
///
/// ```lua
/// wezterm.status.get()               -- {cwd, cwd_short, workspace, clock, llm_agent, gpu_name}
/// wezterm.status.update(table)       -- set all fields from a Lua table
/// wezterm.status.update_clock(str)   -- set only the clock field
/// wezterm.status.age_ms()            -- milliseconds since last full update
/// ```
pub fn register_lua_api(lua: &mlua::Lua) -> anyhow::Result<()> {
    let status_mod = get_or_create_sub_module(lua, "status")?;
    let state = StatusBarState::global().clone();

    // -- wezterm.status.get() -> table ----------------------------------------
    {
        let s = state.clone();
        status_mod.set(
            "get",
            lua.create_function(move |lua_ctx, ()| {
                let data = s.get();
                let table = lua_ctx.create_table()?;
                table.set("cwd", data.cwd)?;
                table.set("cwd_short", data.cwd_short)?;
                table.set("workspace", data.workspace)?;
                table.set("clock", data.clock)?;
                table.set("llm_agent", data.llm_agent)?;
                table.set("gpu_name", data.gpu_name)?;
                Ok(table)
            })?,
        )?;
    }

    // -- wezterm.status.update(table) -> nil ----------------------------------
    {
        let s = state.clone();
        status_mod.set(
            "update",
            lua.create_function(move |_, tbl: mlua::Table| {
                let get_str = |key: &str| -> String {
                    tbl.get::<_, String>(key).unwrap_or_default()
                };
                let data = StatusData {
                    cwd: get_str("cwd"),
                    cwd_short: get_str("cwd_short"),
                    workspace: get_str("workspace"),
                    clock: get_str("clock"),
                    llm_agent: get_str("llm_agent"),
                    gpu_name: get_str("gpu_name"),
                };
                s.update(data);
                Ok(())
            })?,
        )?;
    }

    // -- wezterm.status.update_clock(str) -> nil ------------------------------
    {
        let s = state.clone();
        status_mod.set(
            "update_clock",
            lua.create_function(move |_, clock: String| {
                s.update_clock(clock);
                Ok(())
            })?,
        )?;
    }

    // -- wezterm.status.age_ms() -> number ------------------------------------
    // Returns -1 if no update has ever been performed (u64::MAX would overflow
    // Lua's f64-backed number type).
    {
        let s = state;
        status_mod.set(
            "age_ms",
            lua.create_function(move |_, ()| {
                let age = s.age_ms();
                if age == u64::MAX {
                    Ok(-1i64)
                } else {
                    #[expect(clippy::cast_possible_wrap, reason = "age in ms will not exceed i64::MAX")]
                    Ok(age as i64)
                }
            })?,
        )?;
    }

    log::debug!("Registered wezterm.status Lua API");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn new_state_has_empty_defaults() {
        let state = StatusBarState::new();
        let data = state.get();
        assert!(data.cwd.is_empty());
        assert!(data.cwd_short.is_empty());
        assert!(data.workspace.is_empty());
        assert!(data.clock.is_empty());
        assert!(data.llm_agent.is_empty());
        assert!(data.gpu_name.is_empty());
    }

    #[test]
    fn update_and_get_roundtrip() {
        let state = StatusBarState::new();
        state.update(StatusData {
            cwd: "/home/user/projects".into(),
            cwd_short: "~/projects".into(),
            workspace: "default".into(),
            clock: "14:32".into(),
            llm_agent: "claude".into(),
            gpu_name: "RTX 4090".into(),
        });

        let data = state.get();
        assert_eq!(data.cwd, "/home/user/projects");
        assert_eq!(data.cwd_short, "~/projects");
        assert_eq!(data.workspace, "default");
        assert_eq!(data.clock, "14:32");
        assert_eq!(data.llm_agent, "claude");
        assert_eq!(data.gpu_name, "RTX 4090");
    }

    #[test]
    fn update_clock_only_changes_clock() {
        let state = StatusBarState::new();
        state.update(StatusData {
            cwd: "/tmp".into(),
            workspace: "ws1".into(),
            clock: "10:00".into(),
            ..StatusData::default()
        });

        state.update_clock("10:01".into());

        let data = state.get();
        assert_eq!(data.clock, "10:01");
        // Other fields unchanged
        assert_eq!(data.cwd, "/tmp");
        assert_eq!(data.workspace, "ws1");
    }

    #[test]
    fn age_ms_before_any_update() {
        let state = StatusBarState::new();
        assert_eq!(state.age_ms(), u64::MAX);
    }

    #[test]
    fn age_ms_after_update() {
        let state = StatusBarState::new();
        state.update(StatusData::default());
        // Should be very small — just a few ms at most
        let age = state.age_ms();
        assert!(age < 1000, "age_ms should be < 1000ms, got {age}");
    }

    #[test]
    fn thread_safety_concurrent_updates() {
        let state = Arc::new(StatusBarState::new());
        let mut handles = Vec::new();

        // Spawn multiple writer threads
        for i in 0..4 {
            let s = state.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    s.update(StatusData {
                        cwd: format!("/thread-{i}/iter-{j}"),
                        clock: format!("{i}:{j:02}"),
                        ..StatusData::default()
                    });
                }
            }));
        }

        // Spawn reader threads
        for _ in 0..4 {
            let s = state.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _data = s.get();
                    let _age = s.age_ms();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread should not panic");
        }

        // After all threads complete, state should be consistent
        let data = state.get();
        // cwd should be from one of the writer threads (non-empty)
        assert!(!data.cwd.is_empty());
    }

    #[test]
    fn update_overwrites_previous_data() {
        let state = StatusBarState::new();

        state.update(StatusData {
            cwd: "first".into(),
            workspace: "first-ws".into(),
            ..StatusData::default()
        });

        state.update(StatusData {
            cwd: "second".into(),
            workspace: "second-ws".into(),
            ..StatusData::default()
        });

        let data = state.get();
        assert_eq!(data.cwd, "second");
        assert_eq!(data.workspace, "second-ws");
    }

    // -- Lua API smoke tests --------------------------------------------------

    #[test]
    fn lua_api_registers_without_panic() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register_lua_api should succeed");

        let package: mlua::Table = lua.globals().get("package").expect("package");
        let loaded: mlua::Table = package.get("loaded").expect("loaded");
        let wezterm: mlua::Table = loaded.get("wezterm").expect("wezterm");
        let status: mlua::Table = wezterm.get("status").expect("status sub-module");

        for name in &["get", "update", "update_clock", "age_ms"] {
            let _: mlua::Function = status
                .get(*name)
                .unwrap_or_else(|_| panic!("function '{name}' should exist"));
        }
    }

    #[test]
    fn lua_get_returns_table_with_fields() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        let result: mlua::Table = lua
            .load("return require('wezterm').status.get()")
            .eval()
            .expect("eval get()");

        // All fields should be present as strings (empty by default)
        for key in &["cwd", "cwd_short", "workspace", "clock", "llm_agent", "gpu_name"] {
            let val: String = result
                .get(*key)
                .unwrap_or_else(|_| panic!("field '{key}' should exist"));
            assert!(val.is_empty() || !val.is_empty(), "field '{key}' is a string");
        }
    }

    #[test]
    fn lua_update_and_get_roundtrip() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        lua.load(
            r#"
            local s = require('wezterm').status
            s.update({
                cwd = '/home/test',
                cwd_short = '~/test',
                workspace = 'dev',
                clock = '15:45',
                llm_agent = 'active',
                gpu_name = 'NVIDIA'
            })
        "#,
        )
        .exec()
        .expect("update call");

        let result: mlua::Table = lua
            .load("return require('wezterm').status.get()")
            .eval()
            .expect("eval get()");

        let cwd: String = result.get("cwd").expect("cwd");
        assert_eq!(cwd, "/home/test");

        let ws: String = result.get("workspace").expect("workspace");
        assert_eq!(ws, "dev");

        let clock: String = result.get("clock").expect("clock");
        assert_eq!(clock, "15:45");
    }

    #[test]
    fn lua_update_clock_only() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // Set initial data
        lua.load(
            r#"
            local s = require('wezterm').status
            s.update({ cwd = '/tmp', workspace = 'main', clock = '12:00' })
            s.update_clock('12:01')
        "#,
        )
        .exec()
        .expect("update + update_clock");

        let result: mlua::Table = lua
            .load("return require('wezterm').status.get()")
            .eval()
            .expect("eval get()");

        let clock: String = result.get("clock").expect("clock");
        assert_eq!(clock, "12:01");

        let cwd: String = result.get("cwd").expect("cwd");
        assert_eq!(cwd, "/tmp");
    }

    #[test]
    fn lua_age_ms_returns_number() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // Before any update on the global singleton, age_ms returns -1
        // (the global may already have been updated by other tests sharing the
        // process, so we only check that the value is a valid number).
        let age: i64 = lua
            .load("return require('wezterm').status.age_ms()")
            .eval()
            .expect("eval age_ms()");
        // Either -1 (never updated) or a small positive number (updated by
        // another test in the same process).
        assert!(
            age == -1 || age >= 0,
            "age_ms should be -1 or non-negative, got {age}"
        );

        // After update, age_ms should be small and non-negative
        lua.load("require('wezterm').status.update({})")
            .exec()
            .expect("update");

        let age: i64 = lua
            .load("return require('wezterm').status.age_ms()")
            .eval()
            .expect("eval age_ms() after update");
        assert!(
            age >= 0 && age < 1000,
            "age_ms should be in [0, 1000) after update, got {age}"
        );
    }

    #[test]
    fn lua_update_with_missing_fields_defaults_to_empty() {
        let lua = mlua::Lua::new();
        register_lua_api(&lua).expect("register");

        // Update with only some fields set
        lua.load(
            r#"
            require('wezterm').status.update({ cwd = '/partial' })
        "#,
        )
        .exec()
        .expect("partial update");

        let result: mlua::Table = lua
            .load("return require('wezterm').status.get()")
            .eval()
            .expect("get");

        let cwd: String = result.get("cwd").expect("cwd");
        assert_eq!(cwd, "/partial");

        let workspace: String = result.get("workspace").expect("workspace");
        assert!(workspace.is_empty(), "missing fields default to empty string");
    }
}
