//! Module registry for managing module lifecycle.
//!
//! The registry tracks all registered modules and provides methods
//! for initialization, startup, and shutdown.

use crate::{Capabilities, Module, ModuleContext, ModuleInfo, ModuleState};
use anyhow::{bail, Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Global module registry.
///
/// Manages the lifecycle of all registered modules.
pub struct ModuleRegistry {
    /// Registered modules by ID.
    modules: RwLock<HashMap<String, Arc<RwLock<Box<dyn Module>>>>>,
    /// Granted capabilities per module.
    capabilities: RwLock<HashMap<String, Capabilities>>,
}

impl ModuleRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global registry instance.
    ///
    /// Creates a new instance if one doesn't exist.
    pub fn global() -> &'static ModuleRegistry {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<ModuleRegistry> = OnceLock::new();
        INSTANCE.get_or_init(ModuleRegistry::new)
    }

    /// Register a module with the registry.
    ///
    /// The module must have a unique ID.
    pub fn register(&self, module: Box<dyn Module>) -> Result<()> {
        let id = module.module_id().to_string();
        let capabilities = module.required_capabilities();

        let mut modules = self.modules.write();
        if modules.contains_key(&id) {
            bail!("Module '{}' is already registered", id);
        }

        log::info!(
            "Registering module '{}' ({}) with capabilities {:?}",
            id,
            module.display_name(),
            capabilities
        );

        modules.insert(id.clone(), Arc::new(RwLock::new(module)));
        self.capabilities.write().insert(id, capabilities);

        Ok(())
    }

    /// Unregister a module from the registry.
    pub fn unregister(&self, module_id: &str) -> Result<()> {
        let mut modules = self.modules.write();
        if modules.remove(module_id).is_none() {
            bail!("Module '{}' is not registered", module_id);
        }
        self.capabilities.write().remove(module_id);
        log::info!("Unregistered module '{}'", module_id);
        Ok(())
    }

    /// Check if a module is registered.
    pub fn is_registered(&self, module_id: &str) -> bool {
        self.modules.read().contains_key(module_id)
    }

    /// Get information about all registered modules.
    pub fn list_modules(&self) -> Vec<ModuleInfo> {
        self.modules
            .read()
            .values()
            .map(|m| ModuleInfo::from_module(&**m.read()))
            .collect()
    }

    /// Get information about a specific module.
    pub fn get_module_info(&self, module_id: &str) -> Option<ModuleInfo> {
        self.modules
            .read()
            .get(module_id)
            .map(|m| ModuleInfo::from_module(&**m.read()))
    }

    /// Initialize all registered modules.
    pub async fn init_all(&self) -> Result<()> {
        let module_ids: Vec<String> = self.modules.read().keys().cloned().collect();

        for id in module_ids {
            self.init_module(&id).await?;
        }

        Ok(())
    }

    /// Initialize a specific module.
    pub async fn init_module(&self, module_id: &str) -> Result<()> {
        let module = self
            .modules
            .read()
            .get(module_id)
            .cloned()
            .context(format!("Module '{}' not found", module_id))?;

        let capabilities = self
            .capabilities
            .read()
            .get(module_id)
            .copied()
            .unwrap_or(Capabilities::empty());

        let ctx = ModuleContext::new(capabilities);

        log::info!("Initializing module '{}'", module_id);
        module.write().init(&ctx).await?;

        Ok(())
    }

    /// Start all initialized modules.
    pub async fn start_all(&self) -> Result<()> {
        let module_ids: Vec<String> = self.modules.read().keys().cloned().collect();

        for id in module_ids {
            let state = self
                .modules
                .read()
                .get(&id)
                .map(|m| m.read().state())
                .unwrap_or(ModuleState::Error);

            if state.can_start() {
                self.start_module(&id).await?;
            }
        }

        Ok(())
    }

    /// Start a specific module.
    pub async fn start_module(&self, module_id: &str) -> Result<()> {
        let module = self
            .modules
            .read()
            .get(module_id)
            .cloned()
            .context(format!("Module '{}' not found", module_id))?;

        let state = module.read().state();
        if !state.can_start() {
            bail!(
                "Module '{}' cannot be started from state {:?}",
                module_id,
                state
            );
        }

        let capabilities = self
            .capabilities
            .read()
            .get(module_id)
            .copied()
            .unwrap_or(Capabilities::empty());

        let ctx = ModuleContext::new(capabilities);

        log::info!("Starting module '{}'", module_id);
        module.write().start(&ctx).await?;

        Ok(())
    }

    /// Stop all running modules.
    pub async fn stop_all(&self) -> Result<()> {
        let module_ids: Vec<String> = self.modules.read().keys().cloned().collect();

        for id in module_ids {
            let state = self
                .modules
                .read()
                .get(&id)
                .map(|m| m.read().state())
                .unwrap_or(ModuleState::Stopped);

            if state.is_active() {
                self.stop_module(&id).await?;
            }
        }

        Ok(())
    }

    /// Stop a specific module.
    pub async fn stop_module(&self, module_id: &str) -> Result<()> {
        let module = self
            .modules
            .read()
            .get(module_id)
            .cloned()
            .context(format!("Module '{}' not found", module_id))?;

        log::info!("Stopping module '{}'", module_id);
        module.write().stop().await?;

        Ok(())
    }

    /// Reload all running modules.
    pub async fn reload_all(&self) -> Result<()> {
        let module_ids: Vec<String> = self.modules.read().keys().cloned().collect();

        for id in module_ids {
            let state = self
                .modules
                .read()
                .get(&id)
                .map(|m| m.read().state())
                .unwrap_or(ModuleState::Stopped);

            if state.is_active() {
                self.reload_module(&id).await?;
            }
        }

        Ok(())
    }

    /// Reload a specific module.
    pub async fn reload_module(&self, module_id: &str) -> Result<()> {
        let module = self
            .modules
            .read()
            .get(module_id)
            .cloned()
            .context(format!("Module '{}' not found", module_id))?;

        let capabilities = self
            .capabilities
            .read()
            .get(module_id)
            .copied()
            .unwrap_or(Capabilities::empty());

        let ctx = ModuleContext::new(capabilities);

        log::info!("Reloading module '{}'", module_id);
        module.write().reload(&ctx).await?;

        Ok(())
    }

    /// Register Lua APIs for all modules.
    pub fn register_all_lua_apis(&self, lua: &mlua::Lua) -> Result<()> {
        let modules = self.modules.read();

        for (id, module) in modules.iter() {
            log::debug!("Registering Lua API for module '{}'", id);
            if let Err(e) = module.read().register_lua_api(lua) {
                log::error!("Failed to register Lua API for module '{}': {}", id, e);
            }
        }

        Ok(())
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRegistry")
            .field("module_count", &self.modules.read().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestModule {
        id: String,
        state: ModuleState,
    }

    #[async_trait(?Send)]
    impl Module for TestModule {
        fn module_id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            "Test Module"
        }

        fn required_capabilities(&self) -> Capabilities {
            Capabilities::FILESYSTEM_READ
        }

        fn state(&self) -> ModuleState {
            self.state
        }

        async fn init(&mut self, _ctx: &ModuleContext) -> Result<()> {
            self.state = ModuleState::Initialized;
            Ok(())
        }

        async fn start(&mut self, _ctx: &ModuleContext) -> Result<()> {
            self.state = ModuleState::Running;
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.state = ModuleState::Stopped;
            Ok(())
        }
    }

    #[test]
    fn test_register_module() {
        let registry = ModuleRegistry::new();
        let module = Box::new(TestModule {
            id: "test-module".to_string(),
            state: ModuleState::Registered,
        });

        assert!(registry.register(module).is_ok());
        assert!(registry.is_registered("test-module"));
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = ModuleRegistry::new();

        let module1 = Box::new(TestModule {
            id: "test-module".to_string(),
            state: ModuleState::Registered,
        });
        let module2 = Box::new(TestModule {
            id: "test-module".to_string(),
            state: ModuleState::Registered,
        });

        assert!(registry.register(module1).is_ok());
        assert!(registry.register(module2).is_err());
    }

    #[test]
    fn test_list_modules() {
        let registry = ModuleRegistry::new();

        registry
            .register(Box::new(TestModule {
                id: "module-a".to_string(),
                state: ModuleState::Registered,
            }))
            .unwrap();

        registry
            .register(Box::new(TestModule {
                id: "module-b".to_string(),
                state: ModuleState::Registered,
            }))
            .unwrap();

        let modules = registry.list_modules();
        assert_eq!(modules.len(), 2);
    }
}
