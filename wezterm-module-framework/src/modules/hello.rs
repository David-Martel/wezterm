//! Example "Hello World" module demonstrating the WezTerm module framework.
//!
//! This module serves as a minimal, self-contained template for building custom
//! WezTerm modules. It registers a simple Lua API and demonstrates the full
//! module lifecycle without requiring any external resources or capabilities.
//!
//! ## Usage from Lua
//!
//! ```lua
//! -- After registering the hello module in startup.rs:
//! local greeting = wezterm.hello.greet("World")
//! -- Returns: "Hello, World! from WezTerm module framework"
//!
//! local info = wezterm.hello.info()
//! -- Returns: { module_id = "hello", state = "registered", version = "0.1.0" }
//! ```
//!
//! ## Registration (add to startup.rs to activate)
//!
//! ```rust,ignore
//! use crate::modules::hello::HelloModule;
//!
//! fn register_hello(registry: &ModuleRegistry) {
//!     if let Err(e) = registry.register(Box::new(HelloModule::new())) {
//!         log::warn!("Failed to register hello module: {}", e);
//!     }
//! }
//! ```
//!
//! Then call `register_hello(registry)` from [`initialize_modules`] in
//! `startup.rs`.
//!
//! ## Design Notes
//!
//! - **No capabilities required**: The hello module needs no filesystem,
//!   network, or UI access, so `required_capabilities()` returns an empty set.
//! - **Stateless greet function**: `greet()` works even when the module is in
//!   `Registered` state (before `init`/`start`), demonstrating that Lua APIs
//!   can be registered independently of the module lifecycle.
//! - **Lifecycle logging**: Each lifecycle method logs at `info` level so
//!   developers can observe the init → start → stop flow in WezTerm's log.

use crate::{Capabilities, Module, ModuleContext, ModuleState};
use anyhow::Result;
use async_trait::async_trait;
use config::lua::get_or_create_sub_module;

/// The crate version, surfaced via `wezterm.hello.info()`.
const VERSION: &str = "0.1.0";

/// A minimal example module for the WezTerm module framework.
///
/// `HelloModule` demonstrates:
/// - Implementing the [`Module`] trait with all lifecycle methods
/// - Registering a Lua sub-module under `wezterm.hello.*`
/// - Tracking lifecycle state transitions
///
/// This module requires no capabilities and performs no I/O, making it
/// safe to use as a starting point for new module development.
#[derive(Debug)]
pub struct HelloModule {
    /// Current lifecycle state.
    state: ModuleState,
}

impl HelloModule {
    /// Create a new `HelloModule` in the [`ModuleState::Registered`] state.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use wezterm_module_framework::modules::hello::HelloModule;
    ///
    /// let module = HelloModule::new();
    /// assert_eq!(module.module_id(), "hello");
    /// ```
    pub fn new() -> Self {
        Self {
            state: ModuleState::Registered,
        }
    }

    /// Format a greeting string.
    ///
    /// This is the core logic behind `wezterm.hello.greet(name)`.
    /// Extracted as a standalone method for testability.
    pub fn greet(name: &str) -> String {
        format!("Hello, {name}! from WezTerm module framework")
    }

    /// Return the module's display state as a string suitable for Lua.
    fn state_label(&self) -> &'static str {
        match self.state {
            ModuleState::Registered => "registered",
            ModuleState::Initialized => "initialized",
            ModuleState::Running => "running",
            ModuleState::Paused => "paused",
            ModuleState::Stopped => "stopped",
            ModuleState::Error => "error",
        }
    }
}

impl Default for HelloModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Module for HelloModule {
    fn module_id(&self) -> &str {
        "hello"
    }

    fn display_name(&self) -> &str {
        "Hello World (Example)"
    }

    fn required_capabilities(&self) -> Capabilities {
        // The hello module needs no capabilities — it performs no I/O,
        // spawns no processes, and creates no UI elements.
        Capabilities::empty()
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    async fn init(&mut self, _ctx: &ModuleContext) -> Result<()> {
        log::info!("HelloModule: initializing");
        self.state = ModuleState::Initialized;
        Ok(())
    }

    async fn start(&mut self, _ctx: &ModuleContext) -> Result<()> {
        log::info!("HelloModule: starting");
        self.state = ModuleState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        log::info!("HelloModule: stopping");
        self.state = ModuleState::Stopped;
        Ok(())
    }

    /// Reload the hello module.
    ///
    /// The default `Module::reload` implementation stops and restarts. For
    /// this simple module that is fine, but we override it to add logging
    /// so developers can see the reload path in the logs.
    async fn reload(&mut self, ctx: &ModuleContext) -> Result<()> {
        log::info!("HelloModule: reloading (stop + start)");
        self.stop().await?;
        self.start(ctx).await
    }

    /// Register the `wezterm.hello` Lua sub-module.
    ///
    /// Exposes:
    /// - `wezterm.hello.greet(name)` — returns a greeting string
    /// - `wezterm.hello.info()` — returns a table with module metadata
    fn register_lua_api(&self, lua: &mlua::Lua) -> Result<()> {
        let hello_mod = get_or_create_sub_module(lua, "hello")?;

        // ------------------------------------------------------------------
        // wezterm.hello.greet(name)
        //
        // Returns: string — "Hello, {name}! from WezTerm module framework"
        //
        // Example:
        //   local msg = wezterm.hello.greet("World")
        //   -- msg == "Hello, World! from WezTerm module framework"
        // ------------------------------------------------------------------
        hello_mod.set(
            "greet",
            lua.create_function(|_, name: String| Ok(HelloModule::greet(&name)))?,
        )?;

        // ------------------------------------------------------------------
        // wezterm.hello.info()
        //
        // Returns: table { module_id, state, version }
        //
        // Example:
        //   local t = wezterm.hello.info()
        //   -- t.module_id == "hello"
        //   -- t.version  == "0.1.0"
        // ------------------------------------------------------------------
        let state_label = self.state_label().to_string();
        hello_mod.set(
            "info",
            lua.create_function(move |lua_ctx, ()| {
                let table = lua_ctx.create_table()?;
                table.set("module_id", "hello")?;
                table.set("state", state_label.as_str())?;
                table.set("version", VERSION)?;
                Ok(table)
            })?,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_module_creation() {
        let module = HelloModule::new();
        assert_eq!(
            module.state(),
            ModuleState::Registered,
            "new HelloModule should be in Registered state"
        );
    }

    #[test]
    fn test_hello_module_id() {
        let module = HelloModule::new();
        assert_eq!(module.module_id(), "hello");
        assert_eq!(module.display_name(), "Hello World (Example)");
    }

    #[test]
    fn test_required_capabilities() {
        let module = HelloModule::new();
        let caps = module.required_capabilities();
        assert!(
            caps.is_empty(),
            "hello module should require no capabilities"
        );
    }

    #[test]
    fn test_greet_function() {
        assert_eq!(
            HelloModule::greet("World"),
            "Hello, World! from WezTerm module framework"
        );
        assert_eq!(
            HelloModule::greet("Rust"),
            "Hello, Rust! from WezTerm module framework"
        );
        assert_eq!(
            HelloModule::greet(""),
            "Hello, ! from WezTerm module framework"
        );
    }

    #[test]
    fn test_state_label() {
        let module = HelloModule::new();
        assert_eq!(module.state_label(), "registered");
    }

    #[test]
    fn test_default_trait() {
        let module = HelloModule::default();
        assert_eq!(module.state(), ModuleState::Registered);
        assert_eq!(module.module_id(), "hello");
    }

    #[test]
    fn test_debug_trait() {
        let module = HelloModule::new();
        let debug_str = format!("{:?}", module);
        assert!(
            debug_str.contains("HelloModule"),
            "Debug output should contain struct name"
        );
        assert!(
            debug_str.contains("Registered"),
            "Debug output should contain state"
        );
    }
}
