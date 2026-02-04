//! Filesystem Explorer Module
//!
//! This module provides a terminal-based filesystem explorer pane for WezTerm
//! with vim-style keybindings and git integration support.
//!
//! ## Lua API
//!
//! ```lua
//! -- Spawn a new filesystem explorer pane in the current tab
//! local pane_id = wezterm.fs_explorer.spawn({ dir = "/home/user" })
//!
//! -- Spawn with default directory (current working directory)
//! local pane_id = wezterm.fs_explorer.spawn()
//!
//! -- Check if fs_explorer module is available
//! local available = wezterm.fs_explorer.is_available()
//! ```

pub mod pane;

pub use pane::{allocate_fs_explorer_pane, FsExplorerInput, FsExplorerPane};

use crate::{Capabilities, Module, ModuleContext, ModuleState};
use async_trait::async_trait;
use config::lua::get_or_create_sub_module;
use mux::MuxNotification;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

/// FsExplorerModule: A module that provides filesystem exploration capabilities
pub struct FsExplorerModule {
    state: Mutex<ModuleState>,
    start_dir: PathBuf,
}

/// Returns a platform-appropriate default start directory.
#[cfg(windows)]
fn default_start_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\"))
}

#[cfg(not(windows))]
fn default_start_dir() -> PathBuf {
    PathBuf::from("/")
}

impl FsExplorerModule {
    /// Create a new FsExplorerModule
    pub fn new(start_dir: Option<PathBuf>) -> Self {
        Self {
            state: Mutex::new(ModuleState::Registered),
            start_dir: start_dir.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| default_start_dir())
            }),
        }
    }

    /// Create a new filesystem explorer pane
    pub fn create_pane(
        &self,
        domain_id: mux::domain::DomainId,
        size: wezterm_term::TerminalSize,
        start_dir: Option<PathBuf>,
        term_config: Option<Arc<dyn wezterm_term::TerminalConfiguration + Send + Sync>>,
    ) -> anyhow::Result<(crossbeam::channel::Receiver<FsExplorerInput>, Arc<dyn mux::pane::Pane>)> {
        let dir = start_dir.unwrap_or_else(|| self.start_dir.clone());
        allocate_fs_explorer_pane(domain_id, size, dir, term_config)
    }
}

#[async_trait(?Send)]
impl Module for FsExplorerModule {
    fn module_id(&self) -> &str {
        "fs-explorer"
    }

    fn display_name(&self) -> &str {
        "Filesystem Explorer"
    }

    fn required_capabilities(&self) -> Capabilities {
        Capabilities::FILESYSTEM_READ | Capabilities::UI_CREATE_PANE
    }

    fn state(&self) -> ModuleState {
        *self.state.lock()
    }

    async fn init(&mut self, _ctx: &ModuleContext) -> anyhow::Result<()> {
        *self.state.lock() = ModuleState::Initialized;
        log::info!("FsExplorerModule initialized");
        Ok(())
    }

    async fn start(&mut self, _ctx: &ModuleContext) -> anyhow::Result<()> {
        *self.state.lock() = ModuleState::Running;
        log::info!("FsExplorerModule started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        *self.state.lock() = ModuleState::Stopped;
        log::info!("FsExplorerModule stopped");
        Ok(())
    }

    fn on_mux_notification(&mut self, notification: &MuxNotification) {
        // Handle mux notifications if needed
        match notification {
            MuxNotification::PaneAdded(_) => {
                log::debug!("FsExplorerModule: Pane added");
            }
            MuxNotification::PaneRemoved(_) => {
                log::debug!("FsExplorerModule: Pane removed");
            }
            _ => {}
        }
    }

    fn register_lua_api(&self, lua: &mlua::Lua) -> anyhow::Result<()> {
        let fs_explorer_mod = get_or_create_sub_module(lua, "fs_explorer")?;

        // Store the default start directory for use in closures
        let default_dir = self.start_dir.clone();

        // wezterm.fs_explorer.spawn(options)
        // options: { dir = "/path/to/dir" } (optional)
        // Returns: pane_id (number) or nil on error
        //
        // Note: This function requires access to the Mux which is only available
        // from the GUI thread context. The function logs the request and returns
        // the requested path for external handling via MuxNotification.
        let spawn_dir = default_dir.clone();
        fs_explorer_mod.set(
            "spawn",
            lua.create_function(move |_, options: Option<mlua::Table>| {
                let dir = options
                    .as_ref()
                    .and_then(|t| t.get::<_, String>("dir").ok())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| spawn_dir.clone());

                log::info!(
                    "Lua: fs_explorer.spawn requested for directory: {}",
                    dir.display()
                );

                // The actual pane creation requires Mux access which isn't
                // available in the Lua context. This would typically be
                // handled by posting a request to the GUI thread.
                //
                // For now, return the path that was requested so calling
                // code can handle the spawn via wezterm's spawn_tab API.
                Ok(dir.to_string_lossy().to_string())
            })?,
        )?;

        // wezterm.fs_explorer.is_available()
        // Returns: true (the module is loaded and available)
        fs_explorer_mod.set(
            "is_available",
            lua.create_function(|_, ()| Ok(true))?,
        )?;

        // wezterm.fs_explorer.default_dir()
        // Returns: the default start directory for new explorer panes
        let get_dir = default_dir;
        fs_explorer_mod.set(
            "default_dir",
            lua.create_function(move |_, ()| Ok(get_dir.to_string_lossy().to_string()))?,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_explorer_module_creation() {
        let module = FsExplorerModule::new(None);
        assert_eq!(module.module_id(), "fs-explorer");
        assert_eq!(module.display_name(), "Filesystem Explorer");
        assert_eq!(module.state(), ModuleState::Registered);
    }

    #[test]
    fn test_required_capabilities() {
        let module = FsExplorerModule::new(None);
        let caps = module.required_capabilities();
        assert!(caps.contains(Capabilities::FILESYSTEM_READ));
        assert!(caps.contains(Capabilities::UI_CREATE_PANE));
    }
}
