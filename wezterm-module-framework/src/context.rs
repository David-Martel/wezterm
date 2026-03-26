//! Module context for safe Mux access.
//!
//! Provides a controlled interface to the Mux that modules can use
//! without direct access to internal Mux state.

use crate::Capabilities;
use anyhow::{bail, Context, Result};
use mux::domain::DomainId;
use mux::pane::PaneId;
use mux::tab::TabId;
use mux::window::WindowId;
use mux::Mux;
use std::sync::Arc;
use wezterm_term::TerminalSize;

/// Context provided to modules for interacting with WezTerm.
///
/// This provides a safe, controlled interface to Mux operations
/// based on the capabilities granted to the module.
#[derive(Clone)]
pub struct ModuleContext {
    /// Capabilities granted to this context.
    capabilities: Capabilities,
}

impl ModuleContext {
    /// Create a new module context with the specified capabilities.
    pub fn new(capabilities: Capabilities) -> Self {
        Self { capabilities }
    }

    /// Create a context with all capabilities (for internal use).
    pub fn full_access() -> Self {
        Self {
            capabilities: Capabilities::all(),
        }
    }

    /// Get the capabilities of this context.
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    /// Check if a capability is granted.
    pub fn has_capability(&self, cap: Capabilities) -> bool {
        self.capabilities.contains(cap)
    }

    /// Get the Mux instance if available.
    fn get_mux(&self) -> Result<Arc<Mux>> {
        Mux::try_get().context("Mux not available")
    }

    // === Window Operations ===

    /// Get the active window ID.
    pub fn active_window(&self) -> Result<Option<WindowId>> {
        let mux = self.get_mux()?;
        // Return the first window for now
        Ok(mux.iter_windows().into_iter().next())
    }

    /// Get all window IDs.
    pub fn all_windows(&self) -> Result<Vec<WindowId>> {
        let mux = self.get_mux()?;
        Ok(mux.iter_windows())
    }

    // === Pane Operations (requires UI_CREATE_PANE) ===

    /// Get a pane by ID.
    pub fn get_pane(&self, pane_id: PaneId) -> Result<Option<Arc<dyn mux::pane::Pane>>> {
        let mux = self.get_mux()?;
        Ok(mux.get_pane(pane_id))
    }

    /// Add a pane to the Mux.
    ///
    /// Requires `UI_CREATE_PANE` capability.
    pub fn add_pane(&self, pane: &Arc<dyn mux::pane::Pane>) -> Result<()> {
        if !self.has_capability(Capabilities::UI_CREATE_PANE) {
            bail!("Module does not have UI_CREATE_PANE capability");
        }
        let mux = self.get_mux()?;
        mux.add_pane(pane)?;
        Ok(())
    }

    /// Remove a pane from the Mux.
    ///
    /// Requires `UI_CREATE_PANE` capability.
    pub fn remove_pane(&self, pane_id: PaneId) -> Result<()> {
        if !self.has_capability(Capabilities::UI_CREATE_PANE) {
            bail!("Module does not have UI_CREATE_PANE capability");
        }
        let mux = self.get_mux()?;
        mux.remove_pane(pane_id);
        Ok(())
    }

    // === Tab Operations ===

    /// Get a tab by ID.
    pub fn get_tab(&self, tab_id: TabId) -> Result<Option<Arc<mux::tab::Tab>>> {
        let mux = self.get_mux()?;
        Ok(mux.get_tab(tab_id))
    }

    /// Add a tab to a window.
    ///
    /// Requires `UI_CREATE_PANE` capability.
    pub fn add_tab_to_window(&self, tab: &Arc<mux::tab::Tab>, window_id: WindowId) -> Result<()> {
        if !self.has_capability(Capabilities::UI_CREATE_PANE) {
            bail!("Module does not have UI_CREATE_PANE capability");
        }
        let mux = self.get_mux()?;
        mux.add_tab_to_window(tab, window_id)?;
        Ok(())
    }

    // === Domain Operations ===

    /// Get the default domain ID.
    pub fn default_domain_id(&self) -> Result<DomainId> {
        let mux = self.get_mux()?;
        Ok(mux.default_domain().domain_id())
    }

    /// Get a domain by ID.
    pub fn get_domain(&self, domain_id: DomainId) -> Result<Option<Arc<dyn mux::domain::Domain>>> {
        let mux = self.get_mux()?;
        Ok(mux.get_domain(domain_id))
    }

    /// Add a domain to the Mux.
    ///
    /// Requires `UI_CREATE_PANE` capability.
    pub fn add_domain(&self, domain: &Arc<dyn mux::domain::Domain>) -> Result<()> {
        if !self.has_capability(Capabilities::UI_CREATE_PANE) {
            bail!("Module does not have UI_CREATE_PANE capability");
        }
        let mux = self.get_mux()?;
        mux.add_domain(domain);
        Ok(())
    }

    // === Workspace Operations ===

    /// Get the active workspace name.
    pub fn active_workspace(&self) -> Result<String> {
        let mux = self.get_mux()?;
        Ok(mux.active_workspace())
    }

    /// Get all workspace names.
    pub fn all_workspaces(&self) -> Result<Vec<String>> {
        let mux = self.get_mux()?;
        Ok(mux.iter_workspaces())
    }

    // === Configuration ===

    /// Get the default terminal size from configuration.
    pub fn default_terminal_size(&self) -> TerminalSize {
        config::configuration().initial_size(0, None)
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> config::ConfigHandle {
        config::configuration()
    }
}

impl std::fmt::Debug for ModuleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleContext")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_capabilities() {
        let ctx = ModuleContext::new(Capabilities::FILESYSTEM_READ | Capabilities::NOTIFICATIONS);

        assert!(ctx.has_capability(Capabilities::FILESYSTEM_READ));
        assert!(ctx.has_capability(Capabilities::NOTIFICATIONS));
        assert!(!ctx.has_capability(Capabilities::NETWORK));
    }

    #[test]
    fn test_full_access_context() {
        let ctx = ModuleContext::full_access();
        assert!(ctx.has_capability(Capabilities::all()));
    }
}
