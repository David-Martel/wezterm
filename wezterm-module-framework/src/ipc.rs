//! Optional IPC bridge to wezterm-utils-daemon for cross-window panel state sync.
//!
//! When the `daemon-ipc` feature is enabled, modules can use the daemon
//! client to register, subscribe to events, and broadcast state changes
//! across WezTerm windows.
//!
//! When the feature is disabled, [`try_connect`] returns `None` immediately,
//! allowing modules to operate in standalone mode without conditional
//! compilation at the call site.

#[cfg(feature = "daemon-ipc")]
pub use wezterm_utils_daemon::client::DaemonClient;

/// Try to connect to the running daemon instance.
///
/// Returns `Some(DaemonClient)` if the daemon is running and reachable,
/// `None` otherwise. Modules should fall back to standalone mode when
/// this returns `None`.
#[cfg(feature = "daemon-ipc")]
pub async fn try_connect() -> Option<DaemonClient> {
    match DaemonClient::connect().await {
        Ok(client) => {
            log::info!("Connected to wezterm-utils-daemon");
            Some(client)
        }
        Err(e) => {
            log::debug!("Daemon not available (standalone mode): {e}");
            None
        }
    }
}

/// Stub when the `daemon-ipc` feature is disabled.
///
/// Always returns `None` -- modules continue in standalone mode.
#[cfg(not(feature = "daemon-ipc"))]
pub async fn try_connect() -> Option<()> {
    log::debug!("Daemon IPC disabled at compile time");
    None
}
