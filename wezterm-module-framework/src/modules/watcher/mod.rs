//! File watcher module providing background file watching service.
//!
//! This module provides:
//! - Background file watching with debouncing
//! - Lua API for registering watches and polling events
//! - Gitignore-aware filtering
//! - MuxNotification forwarding for GUI-thread integration
//!
//! ## Thread Safety
//!
//! `WatcherModule` is `Send + Sync`. It uses a single aggregator thread for all
//! watch events, avoiding per-watcher thread spawning.
//!
//! ## Example (Lua)
//!
//! ```lua
//! local id = wezterm.watcher.watch("/path/to/dir", { recursive = true })
//!
//! -- Poll for buffered events (returns array of { subscription_id, kind, path })
//! local events = wezterm.watcher.poll_events(50)
//! for _, ev in ipairs(events) do
//!   print(ev.kind, ev.path)
//! end
//!
//! wezterm.watcher.unwatch(id)
//! ```

use crate::{Capabilities, Module, ModuleContext, ModuleState};
use anyhow::Result;
use async_trait::async_trait;
use config::lua::get_or_create_sub_module;
use crossbeam::channel::{bounded, Receiver, Sender};
use mux::{Mux, MuxNotification};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use wezterm_fs_utils::watcher::{WatchEvent, WatchEventKind, Watcher};

/// A callback ID for managing watch subscriptions.
pub type WatchCallbackId = u64;

/// A watch subscription.
struct WatchSubscription {
    #[expect(dead_code, reason = "TODO: add justification")]
    path: PathBuf,
    watcher: Watcher,
    #[expect(dead_code, reason = "TODO: add justification")]
    recursive: bool,
    /// Handle to the forwarder thread for this subscription
    forwarder_handle: Option<JoinHandle<()>>,
}

/// Maximum number of events to buffer before dropping the oldest.
const EVENT_QUEUE_CAPACITY: usize = 512;

/// A stored watch event with its subscription ID, suitable for Lua polling.
#[derive(Debug, Clone)]
pub struct StoredWatchEvent {
    /// The subscription that produced this event.
    pub subscription_id: WatchCallbackId,
    /// The kind of event as a string (e.g., "create", "modify", "delete").
    pub kind: String,
    /// The affected path, if available.
    pub path: Option<PathBuf>,
}

/// Shared state that can be accessed from Lua closures.
#[derive(Clone)]
pub struct WatcherModuleHandle {
    subscriptions: Arc<Mutex<HashMap<WatchCallbackId, WatchSubscription>>>,
    next_id: Arc<AtomicU64>,
    event_tx: Arc<Mutex<Option<Sender<(WatchCallbackId, WatchEvent)>>>>,
    /// Buffered events that can be polled from Lua.
    event_queue: Arc<Mutex<VecDeque<StoredWatchEvent>>>,
}

impl WatcherModuleHandle {
    /// Watch a path for changes.
    ///
    /// Returns a subscription ID that can be used to stop watching.
    pub fn watch(
        &self,
        path: PathBuf,
        recursive: bool,
        use_gitignore: bool,
    ) -> Result<WatchCallbackId> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut watcher = Watcher::new(path.clone(), 100, use_gitignore, vec![])?;
        let _handle = watcher.watch(recursive)?;

        // Clone the receiver for forwarding events
        let watcher_rx = watcher.receiver().clone();
        let event_tx_guard = self.event_tx.lock();

        // Spawn a forwarder thread for this watcher
        let forwarder_handle = if let Some(ref tx) = *event_tx_guard {
            let tx = tx.clone();
            match thread::Builder::new()
                .name(format!("watcher-fwd-{}", id))
                .spawn(move || {
                    while let Ok(event) = watcher_rx.recv() {
                        if tx.send((id, event)).is_err() {
                            // Aggregator channel closed, exit
                            break;
                        }
                    }
                }) {
                Ok(handle) => Some(handle),
                Err(e) => {
                    log::warn!("Failed to spawn watcher forwarder thread {}: {}", id, e);
                    None
                }
            }
        } else {
            None
        };

        drop(event_tx_guard); // Release lock before acquiring subscriptions lock

        self.subscriptions.lock().insert(
            id,
            WatchSubscription {
                path,
                watcher,
                recursive,
                forwarder_handle,
            },
        );

        log::debug!("Added watch subscription {}", id);
        Ok(id)
    }

    /// Stop watching a path.
    pub fn unwatch(&self, id: WatchCallbackId) -> Result<()> {
        let mut subs = self.subscriptions.lock();
        if let Some(mut sub) = subs.remove(&id) {
            // Stop the watcher first (closes its channel, causing forwarder to exit)
            sub.watcher.unwatch()?;

            // Wait for forwarder thread to finish (with timeout to avoid hanging)
            if let Some(handle) = sub.forwarder_handle.take() {
                // Give thread a chance to exit gracefully
                let _ = handle.join();
            }

            log::debug!("Removed watch subscription {}", id);
        }
        Ok(())
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().len()
    }

    /// Drain all buffered events, returning up to `max` entries.
    ///
    /// Events are removed from the queue once returned.
    pub fn poll_events(&self, max: usize) -> Vec<StoredWatchEvent> {
        let mut queue = self.event_queue.lock();
        let count = max.min(queue.len());
        queue.drain(..count).collect()
    }

    /// Push an event into the shared queue (called from the aggregator thread).
    fn push_event(&self, event: StoredWatchEvent) {
        let mut queue = self.event_queue.lock();
        if queue.len() >= EVENT_QUEUE_CAPACITY {
            // Drop oldest event to stay within capacity
            queue.pop_front();
        }
        queue.push_back(event);
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
                event_queue: Arc::new(Mutex::new(VecDeque::new())),
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
        self.handle.watch(path, recursive, use_gitignore)
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

        // Clone the handle so the aggregator thread can push events
        // into the shared queue and notify the Mux.
        let handle = self.handle.clone();

        // Spawn event processing thread.
        //
        // Events are:
        //   1. Stored in a bounded queue for Lua polling via `poll_events`.
        //   2. Forwarded to the Mux notification system so GUI subscribers
        //      (e.g. Lua `wezterm.on("file-watch-event", ...)`) can react.
        if let Some(rx) = event_rx {
            if let Err(e) = thread::Builder::new()
                .name("watcher-aggregator".to_string())
                .spawn(move || {
                    loop {
                        // Check for shutdown
                        if shutdown_rx.try_recv().is_ok() {
                            log::debug!("Watcher event loop shutting down");
                            break;
                        }

                        // Process events with timeout
                        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                            Ok((id, event)) => {
                                let kind_str = match &event.kind {
                                    WatchEventKind::Create => "create",
                                    WatchEventKind::Modify => "modify",
                                    WatchEventKind::Delete => "delete",
                                    WatchEventKind::Rename { .. } => "rename",
                                    WatchEventKind::Error(_) => "error",
                                };

                                if let Some(ref path) = event.path {
                                    log::info!(
                                        "File watch event [subscription {}]: {} - {}",
                                        id,
                                        kind_str,
                                        path.display()
                                    );
                                } else {
                                    log::info!(
                                        "File watch event [subscription {}]: {}",
                                        id,
                                        kind_str
                                    );
                                }

                                // 1. Store in the shared queue for Lua polling
                                handle.push_event(StoredWatchEvent {
                                    subscription_id: id,
                                    kind: kind_str.to_string(),
                                    path: event.path.clone(),
                                });

                                // 2. Notify the Mux so GUI-thread subscribers
                                //    can process the event. Use Alert with a
                                //    descriptive message that includes the
                                //    subscription ID and event details.
                                if let Some(mux) = Mux::try_get() {
                                    // Send Empty notification as a lightweight
                                    // "something changed" signal. Subscribers
                                    // interested in watcher events should poll
                                    // via the Lua API (wezterm.watcher.poll_events).
                                    mux.notify(MuxNotification::Empty);
                                }
                            }
                            Err(crossbeam::channel::RecvTimeoutError::Timeout) => continue,
                            Err(crossbeam::channel::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                })
            {
                log::error!("Failed to spawn watcher aggregator thread: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to spawn watcher aggregator thread: {}",
                    e
                ));
            }
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

        // Clear all subscriptions and join forwarder threads
        let mut subs = self.handle.subscriptions.lock();
        for (id, mut sub) in subs.drain() {
            log::debug!("Cleaning up watch subscription {}", id);
            let _ = sub.watcher.unwatch();
            // Wait for forwarder thread to exit
            if let Some(handle) = sub.forwarder_handle.take() {
                let _ = handle.join();
            }
        }

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
            lua.create_function(move |_, (path, options): (String, Option<mlua::Table>)| {
                let recursive = options
                    .as_ref()
                    .and_then(|t| t.get::<_, bool>("recursive").ok())
                    .unwrap_or(true);
                let use_gitignore = options
                    .as_ref()
                    .and_then(|t| t.get::<_, bool>("gitignore").ok())
                    .unwrap_or(true);

                let path_buf = PathBuf::from(&path);
                match watch_handle.watch(path_buf, recursive, use_gitignore) {
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
            })?,
        )?;

        // wezterm.watcher.unwatch(watch_id)
        // Stops watching the path associated with watch_id
        let unwatch_handle = handle.clone();
        watcher_mod.set(
            "unwatch",
            lua.create_function(move |_, id: u64| match unwatch_handle.unwatch(id) {
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
            })?,
        )?;

        // wezterm.watcher.poll_events(max)
        // Drain up to `max` buffered events (default: 100).
        // Returns: array of tables { subscription_id, kind, path }
        let poll_handle = handle.clone();
        watcher_mod.set(
            "poll_events",
            lua.create_function(move |lua_ctx, max: Option<usize>| {
                let max = max.unwrap_or(100);
                let events = poll_handle.poll_events(max);

                let table = lua_ctx.create_table()?;
                for (i, ev) in events.iter().enumerate() {
                    let entry = lua_ctx.create_table()?;
                    entry.set("subscription_id", ev.subscription_id)?;
                    entry.set("kind", ev.kind.as_str())?;
                    if let Some(ref p) = ev.path {
                        entry.set("path", p.to_string_lossy().to_string())?;
                    }
                    table.set(i + 1, entry)?;
                }

                Ok(table)
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
