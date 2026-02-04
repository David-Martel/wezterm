//! File watcher module providing background file watching service.
//!
//! This module provides:
//! - Background file watching with debouncing
//! - Lua API for registering watch callbacks
//! - Gitignore-aware filtering

use crate::{Capabilities, Module, ModuleContext, ModuleState};
use anyhow::Result;
use async_trait::async_trait;
use config::lua::get_or_create_sub_module;
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use wezterm_fs_utils::watcher::{WatchEvent, Watcher};

/// A callback ID for managing watch subscriptions.
pub type WatchCallbackId = u64;

/// A watch subscription.
struct WatchSubscription {
    path: PathBuf,
    watcher: Watcher,
    #[allow(dead_code)]
    recursive: bool,
}

/// File watcher module.
///
/// Provides background file watching capabilities that can be accessed
/// via Lua API.
pub struct WatcherModule {
    state: ModuleState,
    subscriptions: Arc<Mutex<HashMap<WatchCallbackId, WatchSubscription>>>,
    next_id: std::sync::atomic::AtomicU64,
    event_tx: Option<Sender<(WatchCallbackId, WatchEvent)>>,
    event_rx: Option<Receiver<(WatchCallbackId, WatchEvent)>>,
    shutdown_tx: Option<Sender<()>>,
}

impl WatcherModule {
    /// Create a new watcher module.
    pub fn new() -> Self {
        let (event_tx, event_rx) = bounded(1024);

        Self {
            state: ModuleState::Registered,
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            next_id: std::sync::atomic::AtomicU64::new(1),
            event_tx: Some(event_tx),
            event_rx: Some(event_rx),
            shutdown_tx: None,
        }
    }

    /// Watch a path for changes.
    pub fn watch(
        &self,
        path: PathBuf,
        recursive: bool,
        use_gitignore: bool,
    ) -> Result<WatchCallbackId> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut watcher = Watcher::new(path.clone(), 100, use_gitignore, vec![])?;
        let _handle = watcher.watch(recursive)?;

        // Clone the receiver for forwarding events
        let watcher_rx = watcher.receiver().clone();
        let event_tx = self.event_tx.clone();

        // Spawn a thread to forward events from this watcher
        if let Some(tx) = event_tx {
            thread::spawn(move || {
                while let Ok(event) = watcher_rx.recv() {
                    if tx.send((id, event)).is_err() {
                        break;
                    }
                }
            });
        }

        self.subscriptions.lock().insert(
            id,
            WatchSubscription {
                path,
                watcher,
                recursive,
            },
        );

        log::debug!("Added watch subscription {} for path", id);
        Ok(id)
    }

    /// Stop watching a path.
    pub fn unwatch(&self, id: WatchCallbackId) -> Result<()> {
        let mut subs = self.subscriptions.lock();
        if let Some(mut sub) = subs.remove(&id) {
            sub.watcher.unwatch()?;
            log::debug!("Removed watch subscription {}", id);
        }
        Ok(())
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().len()
    }
}

impl Default for WatcherModule {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WatcherModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatcherModule")
            .field("state", &self.state)
            .field("subscription_count", &self.subscription_count())
            .finish()
    }
}

#[async_trait(?Send)]
impl Module for WatcherModule {
    fn module_id(&self) -> &str {
        "file-watcher"
    }

    fn display_name(&self) -> &str {
        "File Watcher"
    }

    fn required_capabilities(&self) -> Capabilities {
        Capabilities::FILESYSTEM_READ | Capabilities::NOTIFICATIONS
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    async fn init(&mut self, _ctx: &ModuleContext) -> Result<()> {
        log::info!("Initializing file watcher module");
        self.state = ModuleState::Initialized;
        Ok(())
    }

    async fn start(&mut self, _ctx: &ModuleContext) -> Result<()> {
        log::info!("Starting file watcher module");

        // Set up shutdown channel
        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Take the event receiver for the event processing loop
        let event_rx = self.event_rx.take();

        // Spawn event processing thread
        if let Some(rx) = event_rx {
            thread::spawn(move || {
                loop {
                    // Check for shutdown
                    if shutdown_rx.try_recv().is_ok() {
                        log::debug!("Watcher event loop shutting down");
                        break;
                    }

                    // Process events with timeout
                    match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok((id, event)) => {
                            log::debug!(
                                "Watch event from subscription {}: {:?}",
                                id,
                                event.kind
                            );
                            // TODO: Forward to Lua callbacks
                        }
                        Err(crossbeam::channel::RecvTimeoutError::Timeout) => continue,
                        Err(crossbeam::channel::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }

        self.state = ModuleState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping file watcher module");

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Clear all subscriptions
        let mut subs = self.subscriptions.lock();
        for (id, mut sub) in subs.drain() {
            log::debug!("Cleaning up watch subscription {}", id);
            let _ = sub.watcher.unwatch();
        }

        self.state = ModuleState::Stopped;
        Ok(())
    }

    fn register_lua_api(&self, lua: &mlua::Lua) -> Result<()> {
        let watcher_mod = get_or_create_sub_module(lua, "watcher")?;

        // wezterm.watcher.watch(path, callback)
        // Note: Full Lua callback implementation would require more infrastructure
        // For now, provide a placeholder that logs usage
        watcher_mod.set(
            "watch",
            lua.create_function(|_, (path, _recursive): (String, Option<bool>)| {
                log::info!("Lua: watcher.watch called for path: {}", path);
                // Return a dummy ID for now
                Ok(0u64)
            })?,
        )?;

        watcher_mod.set(
            "unwatch",
            lua.create_function(|_, id: u64| {
                log::info!("Lua: watcher.unwatch called for id: {}", id);
                Ok(())
            })?,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_module_creation() {
        let module = WatcherModule::new();
        assert_eq!(module.state(), ModuleState::Registered);
        assert_eq!(module.subscription_count(), 0);
    }

    #[test]
    fn test_module_id() {
        let module = WatcherModule::new();
        assert_eq!(module.module_id(), "file-watcher");
        assert_eq!(module.display_name(), "File Watcher");
    }

    #[test]
    fn test_required_capabilities() {
        let module = WatcherModule::new();
        let caps = module.required_capabilities();
        assert!(caps.contains(Capabilities::FILESYSTEM_READ));
        assert!(caps.contains(Capabilities::NOTIFICATIONS));
    }
}
