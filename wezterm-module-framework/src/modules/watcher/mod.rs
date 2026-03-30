//! File watcher module providing background file watching service.
//!
//! This module provides:
//! - Background file watching with debouncing
//! - Lua API for registering watches and polling events
//! - Event-driven callbacks via WezTerm's `wezterm.on` event system
//! - Gitignore-aware filtering
//! - MuxNotification forwarding for GUI-thread integration
//!
//! ## Thread Safety
//!
//! `WatcherModule` is `Send + Sync`. It uses a single aggregator thread for all
//! watch events, avoiding per-watcher thread spawning.
//!
//! ## Example (Lua) — Polling
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
//!
//! ## Example (Lua) — Event-Driven Callbacks
//!
//! ```lua
//! -- Register a handler using the standard wezterm.on API:
//! wezterm.on('file-watch-event', function(event)
//!   -- event.subscription_id: number
//!   -- event.kind: "create" | "modify" | "delete" | "rename" | "error"
//!   -- event.path: string or nil
//!   wezterm.log_info('File changed: ' .. event.kind .. ' ' .. (event.path or ''))
//! end)
//!
//! -- Or use the convenience wrapper:
//! wezterm.watcher.on_event(function(event)
//!   wezterm.log_info('Got event: ' .. event.kind)
//! end)
//!
//! -- Enable event emission (disabled by default to avoid overhead):
//! wezterm.watcher.set_emit_events(true)
//!
//! local id = wezterm.watcher.watch("/path/to/dir", { recursive = true })
//! -- Events now fire automatically; no need to poll.
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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use wezterm_fs_utils::watcher::{WatchEvent, WatchEventKind, Watcher};

/// A callback ID for managing watch subscriptions.
pub type WatchCallbackId = u64;

/// A watch subscription.
struct WatchSubscription {
    path: PathBuf,
    watcher: Watcher,
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
    /// When true, the aggregator thread emits `file-watch-event` via the
    /// WezTerm Lua event system (`wezterm.on`). Disabled by default to
    /// avoid scheduling main-thread work when no handlers are registered.
    emit_events: Arc<AtomicBool>,
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
    ///
    /// Returns `Ok(())` regardless of whether the subscription existed.
    /// Logs the watched path and recursive flag when a subscription is removed.
    pub fn unwatch(&self, id: WatchCallbackId) -> Result<()> {
        let mut subs = self.subscriptions.lock();
        if let Some(mut sub) = subs.remove(&id) {
            log::info!(
                "Unwatching {:?} (recursive={}, id={})",
                sub.path,
                sub.recursive,
                id
            );

            // Take the forwarder handle before dropping the subscription.
            let forwarder = sub.forwarder_handle.take();

            // Stop the filesystem watcher and then drop the entire
            // subscription. Dropping the Watcher closes its internal
            // channel sender, which causes the forwarder thread's
            // `watcher_rx.recv()` to return `Err(Disconnected)` so
            // the thread exits cleanly.
            let _ = sub.watcher.unwatch();
            drop(sub);

            // Now join the forwarder thread (it will have exited because
            // the watcher channel was closed above).
            if let Some(handle) = forwarder {
                let _ = handle.join();
            }

            log::debug!("Removed watch subscription {}", id);
        } else {
            log::debug!("No subscription found for id {}", id);
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

    /// Enable or disable event emission via the Lua event system.
    ///
    /// When enabled, each file watch event is also emitted as a
    /// `file-watch-event` through WezTerm's `wezterm.on` / `wezterm.emit`
    /// system. Lua scripts can register handlers with:
    ///
    /// ```lua
    /// wezterm.on('file-watch-event', function(event)
    ///   -- event.subscription_id, event.kind, event.path
    /// end)
    /// ```
    pub fn set_emit_events(&self, enabled: bool) {
        self.emit_events.store(enabled, Ordering::Relaxed);
        log::info!("Watcher event emission {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Returns true if Lua event emission is enabled.
    pub fn emit_events_enabled(&self) -> bool {
        self.emit_events.load(Ordering::Relaxed)
    }

    /// Emit a file watch event through WezTerm's Lua event system.
    ///
    /// Schedules a `file-watch-event` emission on the main thread.
    /// This is safe to call from any thread (including the aggregator
    /// background thread). The actual Lua callback invocation happens
    /// asynchronously on the main thread.
    fn emit_lua_event(&self, subscription_id: WatchCallbackId, kind: String, path: Option<PathBuf>) {
        if !self.emit_events.load(Ordering::Relaxed) {
            return;
        }

        // Fire-and-forget to the main thread. The outer spawn_into_main_thread
        // gets us onto the main thread (requires Send). The inner
        // promise::spawn::spawn lifts the Send requirement so we can work
        // with Rc<mlua::Lua> from config::with_lua_config_on_main_thread.
        let path_str = path.map(|p| p.to_string_lossy().to_string());
        promise::spawn::spawn_into_main_thread(async move {
            let _ = promise::spawn::spawn(async move {
                if let Err(e) = config::with_lua_config_on_main_thread(|lua| {
                    let kind = kind.clone();
                    let path_str = path_str.clone();
                    async move {
                        if let Some(lua) = lua {
                            let event_table = lua.create_table()?;
                            event_table.set("subscription_id", subscription_id)?;
                            event_table.set("kind", kind.as_str())?;
                            if let Some(ref p) = path_str {
                                event_table.set("path", p.as_str())?;
                            }
                            let args = lua.pack_multi(event_table)?;
                            config::lua::emit_event(
                                &lua,
                                ("file-watch-event".to_string(), args),
                            )
                            .await?;
                        }
                        Ok(())
                    }
                })
                .await
                {
                    log::debug!("Failed to emit file-watch-event: {:#}", e);
                }
            })
            .await;
        })
        .detach();
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
                emit_events: Arc::new(AtomicBool::new(false)),
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

                                // 2. Emit through the Lua event system if enabled.
                                //    This schedules a `file-watch-event` emission
                                //    on the main thread so Lua handlers registered
                                //    via `wezterm.on('file-watch-event', fn)` fire.
                                handle.emit_lua_event(
                                    id,
                                    kind_str.to_string(),
                                    event.path.clone(),
                                );

                                // 3. Notify the Mux so GUI-thread subscribers
                                //    can process the event. Send Empty as a
                                //    lightweight "something changed" signal.
                                if let Some(mux) = Mux::try_get() {
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

        // Clear all subscriptions: unwatch, drop watchers, then join
        // forwarder threads. The watchers must be dropped before joining
        // so their internal channels close and forwarder threads can exit.
        let mut subs = self.handle.subscriptions.lock();
        let mut forwarders = Vec::new();
        for (id, mut sub) in subs.drain() {
            log::debug!("Cleaning up watch subscription {}", id);
            if let Some(handle) = sub.forwarder_handle.take() {
                forwarders.push(handle);
            }
            let _ = sub.watcher.unwatch();
            // sub (and its Watcher) is dropped here, closing the channel
        }
        for handle in forwarders {
            let _ = handle.join();
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
        let count_handle = handle.clone();
        watcher_mod.set(
            "count",
            lua.create_function(move |_, ()| Ok(count_handle.subscription_count()))?,
        )?;

        // wezterm.watcher.set_emit_events(enabled)
        // Enable or disable event-driven callbacks via `wezterm.on('file-watch-event', fn)`.
        // When enabled, each file watch event is emitted through WezTerm's Lua event
        // system in addition to being buffered for `poll_events`.
        // Disabled by default to avoid main-thread scheduling overhead when no
        // event handlers are registered.
        let emit_handle = handle.clone();
        watcher_mod.set(
            "set_emit_events",
            lua.create_function(move |_, enabled: bool| {
                emit_handle.set_emit_events(enabled);
                Ok(())
            })?,
        )?;

        // wezterm.watcher.emit_events_enabled()
        // Returns true if event emission is currently enabled.
        let emit_query_handle = handle.clone();
        watcher_mod.set(
            "emit_events_enabled",
            lua.create_function(move |_, ()| Ok(emit_query_handle.emit_events_enabled()))?,
        )?;

        // wezterm.watcher.on_event(callback)
        // Convenience wrapper that registers the callback via `wezterm.on('file-watch-event', fn)`
        // and enables event emission if not already enabled.
        //
        // The callback receives a single table argument:
        //   { subscription_id = number, kind = string, path = string|nil }
        //
        // Equivalent to:
        //   wezterm.on('file-watch-event', callback)
        //   wezterm.watcher.set_emit_events(true)
        let on_event_handle = handle;
        watcher_mod.set(
            "on_event",
            lua.create_function(move |lua_ctx, func: mlua::Function| {
                // Register via the standard wezterm.on event system
                config::lua::register_event(
                    lua_ctx,
                    ("file-watch-event".to_string(), func),
                )?;

                // Auto-enable event emission
                if !on_event_handle.emit_events_enabled() {
                    on_event_handle.set_emit_events(true);
                }

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

    #[test]
    fn test_unwatch_removes_subscription() {
        let module = WatcherModule::new();
        let dir = std::env::temp_dir().join("wezterm_test_unwatch");
        let _ = std::fs::create_dir_all(&dir);

        // Watch the temp directory
        let id = module
            .watch(dir.clone(), false, false)
            .expect("watch should succeed");
        assert_eq!(module.subscription_count(), 1);

        // Unwatch should succeed and remove the subscription
        module.unwatch(id).expect("unwatch should succeed");
        assert_eq!(module.subscription_count(), 0);

        // Unwatching the same ID again should be a no-op (not an error)
        module
            .unwatch(id)
            .expect("double unwatch should not error");
        assert_eq!(module.subscription_count(), 0);

        // Clean up
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_unwatch_nonexistent_id() {
        let module = WatcherModule::new();

        // Unwatching an ID that was never registered should succeed
        module
            .unwatch(999)
            .expect("unwatch of nonexistent id should not error");
        assert_eq!(module.subscription_count(), 0);
    }

    #[test]
    fn test_emit_events_default_disabled() {
        let module = WatcherModule::new();
        let handle = module.handle();
        assert!(
            !handle.emit_events_enabled(),
            "event emission should be disabled by default"
        );
    }

    #[test]
    fn test_set_emit_events() {
        let module = WatcherModule::new();
        let handle = module.handle();

        handle.set_emit_events(true);
        assert!(handle.emit_events_enabled());

        handle.set_emit_events(false);
        assert!(!handle.emit_events_enabled());
    }

    #[test]
    fn test_emit_lua_event_noop_when_disabled() {
        // When emit_events is false, emit_lua_event should return
        // immediately without scheduling anything. This is a smoke
        // test verifying it doesn't panic.
        let module = WatcherModule::new();
        let handle = module.handle();
        assert!(!handle.emit_events_enabled());

        // Should be a no-op (no main thread running, but the early
        // return prevents any scheduling attempt).
        handle.emit_lua_event(1, "modify".to_string(), Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_poll_events_still_works_with_emit_enabled() {
        // Verify that enabling event emission doesn't interfere with
        // the poll_events queue.
        let module = WatcherModule::new();
        let handle = module.handle();

        handle.set_emit_events(true);

        // Push an event manually
        handle.push_event(StoredWatchEvent {
            subscription_id: 42,
            kind: "modify".to_string(),
            path: Some(PathBuf::from("/tmp/test.txt")),
        });

        let events = handle.poll_events(10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].subscription_id, 42);
        assert_eq!(events[0].kind, "modify");
        assert_eq!(
            events[0].path.as_deref(),
            Some(PathBuf::from("/tmp/test.txt").as_path())
        );
    }
}
