//! Module framework startup and initialization.
//!
//! Provides the entry point for wezterm-gui to initialize all built-in
//! modules during application startup. This bridges the module framework
//! with the WezTerm GUI lifecycle.
//!
//! ## Usage from wezterm-gui
//!
//! ```rust,ignore
//! // In wezterm-gui/src/main.rs or similar startup path:
//! wezterm_module_framework::initialize_modules();
//! ```

use crate::modules::fs_explorer::FsExplorerModule;
use crate::modules::watcher::WatcherModule;
use crate::registry::ModuleRegistry;
use crate::Capabilities;

/// Initialize all built-in modules and register them with the global registry.
///
/// This should be called once during WezTerm startup, after the Mux is
/// available but before the GUI event loop starts.
///
/// Modules are registered but not started — they will be initialized and
/// started when their Lua APIs are first called or when the module registry's
/// `init_all()` / `start_all()` methods are invoked.
pub fn initialize_modules() {
    let registry = ModuleRegistry::global();

    // Register built-in modules
    register_fs_explorer(registry);
    register_watcher(registry);

    let modules = registry.list_modules();
    log::info!(
        "Module framework initialized: {} modules registered",
        modules.len()
    );
    for info in &modules {
        log::debug!(
            "  Module '{}' ({}) - capabilities: {:?}, state: {:?}",
            info.id, info.name, info.capabilities, info.state
        );
    }
}

/// Register all module Lua APIs with a Lua context.
///
/// Called during Lua context setup (config reload, new window, etc.)
/// to make module APIs available to the user's .wezterm.lua.
pub fn register_lua_apis(lua: &mlua::Lua) -> anyhow::Result<()> {
    let registry = ModuleRegistry::global();
    registry.register_all_lua_apis(lua)?;
    log::debug!("Module Lua APIs registered");
    Ok(())
}

fn register_fs_explorer(registry: &ModuleRegistry) {
    let module = FsExplorerModule::new(dirs::home_dir());

    if let Err(e) = registry.register(Box::new(module)) {
        log::warn!("Failed to register fs-explorer module: {}", e);
    }
}

fn register_watcher(registry: &ModuleRegistry) {
    let module = WatcherModule::new();

    if let Err(e) = registry.register(Box::new(module)) {
        log::warn!("Failed to register watcher module: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_modules_idempotent() {
        // Should not panic even if called multiple times
        // (second call will fail to register duplicates, which is fine)
        initialize_modules();
        // Second call — modules already registered, should log warnings but not panic
        initialize_modules();
    }

    #[test]
    fn test_registry_has_modules_after_init() {
        initialize_modules();
        let registry = ModuleRegistry::global();
        let modules = registry.list_modules();
        // At least fs-explorer and watcher should be registered
        assert!(modules.len() >= 2, "Expected at least 2 modules, got {}", modules.len());
    }
}
