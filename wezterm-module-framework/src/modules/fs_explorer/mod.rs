//! Filesystem Explorer Module
//!
//! This module provides a terminal-based filesystem explorer pane for WezTerm
//! with vim-style keybindings and git integration support.

pub mod pane;

pub use pane::{allocate_fs_explorer_pane, FsExplorerInput, FsExplorerPane};

use crate::{Capabilities, Module, ModuleContext, ModuleState};
use async_trait::async_trait;
use mux::MuxNotification;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;

/// FsExplorerModule: A module that provides filesystem exploration capabilities
pub struct FsExplorerModule {
    state: Mutex<ModuleState>,
    start_dir: PathBuf,
}

impl FsExplorerModule {
    /// Create a new FsExplorerModule
    pub fn new(start_dir: Option<PathBuf>) -> Self {
        Self {
            state: Mutex::new(ModuleState::Registered),
            start_dir: start_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
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
        // Register Lua API for spawning filesystem explorer panes
        let module_table = lua.create_table()?;

        // TODO: Add Lua functions to spawn fs explorer panes
        // Example: wezterm.fs_explorer.spawn({ dir = "/home/user" })

        lua.globals().set("fs_explorer", module_table)?;

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
