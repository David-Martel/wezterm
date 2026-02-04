//! WezTerm Module Framework
//!
//! This crate provides a framework for building WezTerm modules that can:
//! - Create custom panes (like the FS Explorer)
//! - Run background services (like file watching)
//! - Register Lua APIs for user configuration
//!
//! ## Architecture
//!
//! Modules implement the `Module` trait and declare their required capabilities.
//! The framework manages module lifecycle (init, start, stop, reload) and
//! provides safe access to the Mux through `ModuleContext`.
//!
//! ## Example
//!
//! ```rust,ignore
//! use wezterm_module_framework::{Module, ModuleContext, Capabilities, ModuleState};
//! use async_trait::async_trait;
//!
//! struct MyModule {
//!     state: ModuleState,
//! }
//!
//! #[async_trait(?Send)]
//! impl Module for MyModule {
//!     fn module_id(&self) -> &str { "my-module" }
//!     fn display_name(&self) -> &str { "My Module" }
//!     fn required_capabilities(&self) -> Capabilities {
//!         Capabilities::FILESYSTEM_READ
//!     }
//!     fn state(&self) -> ModuleState { self.state }
//!
//!     async fn init(&mut self, _ctx: &ModuleContext) -> anyhow::Result<()> {
//!         self.state = ModuleState::Initialized;
//!         Ok(())
//!     }
//!
//!     async fn start(&mut self, _ctx: &ModuleContext) -> anyhow::Result<()> {
//!         self.state = ModuleState::Running;
//!         Ok(())
//!     }
//!
//!     async fn stop(&mut self) -> anyhow::Result<()> {
//!         self.state = ModuleState::Stopped;
//!         Ok(())
//!     }
//! }
//! ```

pub mod context;
pub mod modules;
pub mod registry;

// Re-export main types
pub use context::ModuleContext;
pub use registry::ModuleRegistry;

use async_trait::async_trait;
use bitflags::bitflags;
use mux::MuxNotification;

bitflags! {
    /// Capabilities that a module can request.
    ///
    /// Modules must declare their required capabilities upfront.
    /// This allows the framework to enforce security boundaries and
    /// inform users about what permissions a module needs.
    pub struct Capabilities: u32 {
        /// Read files from the filesystem.
        const FILESYSTEM_READ  = 0b00000001;
        /// Write files to the filesystem.
        const FILESYSTEM_WRITE = 0b00000010;
        /// Spawn external processes.
        const PROCESS_SPAWN    = 0b00000100;
        /// Execute shell commands.
        const SHELL_EXEC       = 0b00001000;
        /// Create and manage panes in the UI.
        const UI_CREATE_PANE   = 0b00010000;
        /// Access window management APIs.
        const UI_WINDOW        = 0b00100000;
        /// Access the system clipboard.
        const CLIPBOARD        = 0b01000000;
        /// Show system notifications.
        const NOTIFICATIONS    = 0b10000000;
        /// Network access for HTTP/WebSocket.
        const NETWORK          = 0b100000000;
    }
}

/// The lifecycle state of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleState {
    /// Module has been registered but not initialized.
    #[default]
    Registered,
    /// Module has been initialized (resources allocated).
    Initialized,
    /// Module is actively running.
    Running,
    /// Module is paused (temporarily inactive).
    Paused,
    /// Module has been stopped.
    Stopped,
    /// Module encountered an error.
    Error,
}

impl ModuleState {
    /// Check if the module is in a state where it can be started.
    pub fn can_start(&self) -> bool {
        matches!(self, ModuleState::Initialized | ModuleState::Paused | ModuleState::Stopped)
    }

    /// Check if the module is currently active.
    pub fn is_active(&self) -> bool {
        matches!(self, ModuleState::Running)
    }
}

/// Trait for implementing WezTerm modules.
///
/// Modules provide extensions to WezTerm such as:
/// - Custom pane types (e.g., file explorer, image viewer)
/// - Background services (e.g., file watching, sync)
/// - Lua API extensions
///
/// ## Lifecycle
///
/// 1. `init()` - Allocate resources, but don't start processing
/// 2. `start()` - Begin active operation
/// 3. `stop()` - Gracefully shutdown
/// 4. `reload()` - Hot-reload configuration (optional)
#[async_trait(?Send)]
pub trait Module: Send + Sync {
    /// Unique identifier for this module.
    fn module_id(&self) -> &str;

    /// Human-readable display name.
    fn display_name(&self) -> &str;

    /// Capabilities required by this module.
    fn required_capabilities(&self) -> Capabilities;

    /// Current state of the module.
    fn state(&self) -> ModuleState;

    /// Initialize the module.
    ///
    /// Called once when the module is first loaded.
    /// Allocate resources but don't start processing yet.
    async fn init(&mut self, ctx: &ModuleContext) -> anyhow::Result<()>;

    /// Start the module.
    ///
    /// Begin active operation. This may spawn background tasks,
    /// register event handlers, etc.
    async fn start(&mut self, ctx: &ModuleContext) -> anyhow::Result<()>;

    /// Stop the module.
    ///
    /// Gracefully shutdown. Stop background tasks, cleanup resources.
    async fn stop(&mut self) -> anyhow::Result<()>;

    /// Reload module configuration.
    ///
    /// Called when WezTerm's configuration is reloaded.
    /// Default implementation stops and restarts the module.
    async fn reload(&mut self, ctx: &ModuleContext) -> anyhow::Result<()> {
        self.stop().await?;
        self.start(ctx).await
    }

    /// Handle a notification from the Mux.
    ///
    /// Called when events occur in the terminal multiplexer,
    /// such as pane creation, tab changes, etc.
    fn on_mux_notification(&mut self, _notification: &MuxNotification) {}

    /// Register Lua API functions for this module.
    ///
    /// Called during Lua context setup to register module-specific APIs.
    fn register_lua_api(&self, _lua: &mlua::Lua) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Metadata about a registered module.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Required capabilities.
    pub capabilities: Capabilities,
    /// Current state.
    pub state: ModuleState,
}

impl ModuleInfo {
    /// Create module info from a module instance.
    pub fn from_module(module: &dyn Module) -> Self {
        Self {
            id: module.module_id().to_string(),
            name: module.display_name().to_string(),
            capabilities: module.required_capabilities(),
            state: module.state(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities() {
        let caps = Capabilities::FILESYSTEM_READ | Capabilities::FILESYSTEM_WRITE;
        assert!(caps.contains(Capabilities::FILESYSTEM_READ));
        assert!(caps.contains(Capabilities::FILESYSTEM_WRITE));
        assert!(!caps.contains(Capabilities::NETWORK));
    }

    #[test]
    fn test_module_state_transitions() {
        assert!(ModuleState::Initialized.can_start());
        assert!(ModuleState::Paused.can_start());
        assert!(ModuleState::Stopped.can_start());
        assert!(!ModuleState::Running.can_start());
        assert!(!ModuleState::Error.can_start());

        assert!(ModuleState::Running.is_active());
        assert!(!ModuleState::Initialized.is_active());
    }
}
