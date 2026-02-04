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
use mlua::RegistryKey;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use wezterm_fs_utils::watcher::{WatchEvent, Watcher};

/// A callback ID for managing watch subscriptions.
pub type WatchCallbackId = u64;

/// A watch subscription with optional Lua callback.
struct WatchSubscription {
    #[allow(dead_code)]
    path: PathBuf,
    watcher: Watcher,
    #[allow(dead_code)]
    recursive: bool,
    /// Lua registry key for the callback function (if registered via Lua)
    lua_callback: Option<RegistryKey>,
}

/// Shared state that can be accessed from Lua closures.
#[derive(Clone)]
pub struct WatcherModuleHandle {
    subscriptions: Arc<Mutex<HashMap<WatchCallbackId, WatchSubscription>>>,
    next_id: Arc<AtomicU64>,
    event_tx: Arc<Mutex<Option<Sender<(WatchCallbackId, WatchEvent)>>>>,
    lua_callbacks: Arc<Mutex<HashMap<WatchCallbackId, RegistryKey>>>,
}

impl WatcherModuleHandle {
    /// Watch a path and register a Lua callback.
    pub fn watch_with_callback(
        &self,
        path: PathBuf,
        recursive: bool,
        use_gitignore: bool,
        lua_callback: Option<RegistryKey>,
    ) -> Result<WatchCallbackId> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut watcher = Watcher::new(path.clone(), 100, use_gitignore, vec![])?;
        let _handle = watcher.watch(recursive)?;

        // Clone the receiver for forwarding events
        let watcher_rx = watcher.receiver().clone();
        let event_tx_lock = self.event_tx.lock();

        // Spawn a thread to forward events from this watcher
        if let Some(ref tx) = *event_tx_lock {
            let tx = tx.clone();
            thread::spawn(move || {
                while let Ok(event) = watcher_rx.recv() {
                    if tx.send((id, event)).is_err() {
                        break;
                    }
                }
            });
        }

        // Store the Lua callback if provided
        if let Some(ref key) = lua_callback {
            self.lua_callbacks
                .lock()
                .insert(id, unsafe { std::ptr::read(key) });
        }

        self.subscriptions.lock().insert(
            id,
            WatchSubscription {
                path,
                watcher,
                recursive,
                lua_callback,
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
            // Remove Lua callback reference
            self.lua_callbacks.lock().remove(&id);
            log::debug!("Removed watch subscription {}", id);
        }
        Ok(())
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().len()
    }
}

/// File watcher module.
///
/// Provides background file watching capabilities that can be accessed
/// via Lua API.
pub struct WatcherModule {
    state: ModuleState,
    handle: WatcherModuleHandle,
    event_rx: Option<Receiver<(WatchCallbackId, WatchEvent)>>,
    shutdown_tx: Option<Sender<()>>,
}

impl WatcherModule {
    /// Create a new watcher module.
    pub fn new() -> Self {
        let (event_tx, event_rx) = bounded(1024);

        Self {
            state: ModuleState::Registered,
            handle: WatcherModuleHandle {
                subscriptions: Arc::new(Mutex::new(HashMap::new())),
                next_id: Arc::new(AtomicU64::new(1)),
                event_tx: Arc::new(Mutex::new(Some(event_tx))),
                lua_callbacks: Arc::new(Mutex::new(HashMap::new())),
            },
            event_rx: Some(event_rx),
            shutdown_tx: None,
        }
    }

    /// Get a handle that can be shared with Lua closures.
    pub fn handle(&self) -> WatcherModuleHandle {
        self.handle.clone()
    }

    /// Watch a path for changes.
    pub fn watch(
        &self,
        path: PathBuf,
        recursive: bool,
        use_gitignore: bool,
    ) -> Result<WatchCallbackId> {
        self.handle
            .watch_with_callback(path, recursive, use_gitignore, None)
    }

    /// Stop watching a path.
    pub fn unwatch(&self, id: WatchCallbackId) -> Result<()> {
        self.handle.unwatch(id)
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.handle.subscription_count()
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
        // Note: Lua callback invocation requires the Lua runtime context,
        // which is not available in this background thread. Events are logged
        // and could be forwarded to the GUI thread via Mux notifications.
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
                            // Log the event with full details
                            if let Some(ref path) = event.path {
                                log::info!(
                                    "File watch event [subscription {}]: {:?} - {}",
                                    id,
                                    event.kind,
                                    path.display()
                                );
                            } else {
                                log::info!(
                                    "File watch event [subscription {}]: {:?}",
                                    id,
                                    event.kind
                                );
                            }
                            // Events can be forwarded to GUI via MuxNotification system
                            // or through a dedicated event channel to the Lua runtime
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
        let mut subs = self.handle.subscriptions.lock();
        for (id, mut sub) in subs.drain() {
            log::debug!("Cleaning up watch subscription {}", id);
            let _ = sub.watcher.unwatch();
        }

        // Clear lua callbacks
        self.handle.lua_callbacks.lock().clear();

        // Clear the event sender
        *self.handle.event_tx.lock() = None;

        self.state = ModuleState::Stopped;
        Ok(())
    }

    fn register_lua_api(&self, lua: &mlua::Lua) -> Result<()> {
        let watcher_mod = get_or_create_sub_module(lua, "watcher")?;

        // Get a handle that can be moved into Lua closures
        let handle = self.handle();

        // wezterm.watcher.watch(path, options)
        // options: { recursive = true/false, gitignore = true/false }
        // Returns: watch_id (number)
        let watch_handle = handle.clone();
        watcher_mod.set(
            "watch",
            lua.create_function(
                move |_, (path, options): (String, Option<mlua::Table>)| {
                    let recursive = options
                        .as_ref()
                        .and_then(|t| t.get::<_, bool>("recursive").ok())
                        .unwrap_or(true);
                    let use_gitignore = options
                        .as_ref()
                        .and_then(|t| t.get::<_, bool>("gitignore").ok())
                        .unwrap_or(true);

                    let path_buf = PathBuf::from(&path);
                    match watch_handle.watch_with_callback(path_buf, recursive, use_gitignore, None)
                    {
                        Ok(id) => {
                            log::info!(
                                "Lua: Started watching '{}' (recursive={}, gitignore={}) -> id {}",
                                path,
                                recursive,
                                use_gitignore,
                                id
                            );
                            Ok(id)
                        }
                        Err(e) => {
                            log::error!("Lua: Failed to watch '{}': {}", path, e);
                            Err(mlua::Error::RuntimeError(format!(
                                "Failed to watch path: {}",
                                e
                            )))
                        }
                    }
                },
            )?,
        )?;

        // wezterm.watcher.unwatch(watch_id)
        // Stops watching the path associated with watch_id
        let unwatch_handle = handle.clone();
        watcher_mod.set(
            "unwatch",
            lua.create_function(move |_, id: u64| {
                match unwatch_handle.unwatch(id) {
                    Ok(()) => {
                        log::info!("Lua: Stopped watching id {}", id);
                        Ok(())
                    }
                    Err(e) => {
                        log::error!("Lua: Failed to unwatch id {}: {}", id, e);
                        Err(mlua::Error::RuntimeError(format!(
                            "Failed to unwatch: {}",
                            e
                        )))
                    }
                }
            })?,
        )?;

        // wezterm.watcher.count()
        // Returns the number of active watch subscriptions
        let count_handle = handle;
        watcher_mod.set(
            "count",
            lua.create_function(move |_, ()| Ok(count_handle.subscription_count()))?,
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
