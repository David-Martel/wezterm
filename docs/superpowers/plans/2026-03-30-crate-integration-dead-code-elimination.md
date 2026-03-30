# Custom Crate Integration & Feature Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the partially-built integrations between wezterm custom crates: wire the daemon's event protocol (subscribe/unsubscribe/keep-alive) into the router, connect the daemon client to the module framework for cross-window panel sync, bring wezterm-fs-explorer into the workspace, and add `wezterm daemon`/`watch`/`explore` subcommands so all utilities ship inside wezterm.exe.

**Architecture:** The daemon was designed for cross-window panel state sync (TODO Tier 2.E, 3.K). The module framework is already bootstrapped in wezterm-gui (Tier 3.G). The missing links are: (1) daemon router doesn't call Connection.subscribe/unsubscribe, (2) module framework has no daemon client bridge, (3) utilities aren't accessible as wezterm subcommands. This plan finishes those connections.

**Tech Stack:** Rust, clap (CLI), tokio (async IPC), Named Pipes/UDS, mlua (Lua), smol (GUI async)

**Prior Plans (this plan continues):**
- `.claude/plans/wezterm-customization-plan.md` — Phase 2 (AI Module), Phase 3 (Utility Enhancements)
- `docs/plans/2026-03-23-wezterm-joint-plan.md` — Tiers 2.E, 3.G, 3.K, 4.K-L
- `TODO.md` — Open tiers referenced throughout

---

## Current State (from TODO.md and prior plans)

| Item | Status | What Remains |
|------|--------|-------------|
| Tier 2.E: Daemon IPC client | DONE | Client exists; subscribe/unsubscribe not wired in router |
| Tier 3.G: Module framework GUI bootstrap | DONE | `initialize_modules()` + `register_lua_apis()` wired in `wezterm-gui/src/main.rs` |
| Tier 3.K: Daemon writer path audit | OPEN | `handle_connection` may use fresh channel instead of `Connection.tx` |
| Tier 4.K: Lua context setup hook | PARTIAL | `add_context_setup_func` is called but mux domain registration pending |
| Tier 4.L: Lua API crate for module framework | OPEN | No `lua-api-crates/module-framework/` yet |
| fs-explorer workspace membership | OPEN | Standalone; `.claude/plans` says "now possible with gix" |
| Daemon keep-alive/heartbeat | SCAFFOLDED | `KEEP_ALIVE_INTERVAL` defined, `ping()` exists, polling loop not built |
| Connection metrics (`connected_at`) | SCAFFOLDED | Field exists, no consumer yet |
| Event subscribe/unsubscribe | SCAFFOLDED | `Connection.subscribe()/unsubscribe()` exist but router doesn't call them |

---

## Phase 1: Fix Daemon Router — Wire Subscribe/Unsubscribe (Tier 3.K follow-up)

### Task 1: Wire subscribe/unsubscribe into daemon router

The router's `handle_subscribe()` and `handle_unsubscribe()` currently return success but never call the `Connection` methods. This breaks the pub/sub event delivery that `broadcast_to_subscribers()` depends on.

**Files:**
- Modify: `wezterm-utils-daemon/src/router.rs:172-204`
- Test: `wezterm-utils-daemon/tests/integration_test.rs`

- [ ] **Step 1: Write failing test — subscribe enables broadcast delivery**

In `wezterm-utils-daemon/src/router.rs` tests module, add:

```rust
#[tokio::test]
async fn test_subscribe_enables_broadcast_delivery() {
    let cm = Arc::new(ConnectionManager::new(10));
    let (tx, mut rx) = mpsc::unbounded_channel();
    let conn = cm.add_connection(tx);
    let conn_id = conn.id.clone();

    // Before subscribing, connection should not be subscribed
    assert!(!conn.is_subscribed_to("panel-state"));

    // Simulate subscribe
    conn.subscribe(vec![EventSubscription {
        event_type: "panel-state".to_string(),
    }]);

    assert!(conn.is_subscribed_to("panel-state"));

    // Broadcast should reach this connection
    let notification = JsonRpcRequest::new("event/panel-state", Some(json!({"explorer": true})), None);
    let sent = cm.broadcast_to_subscribers("panel-state", JsonRpcMessage::Request(notification));
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0], conn_id);

    // Verify message was received
    let msg = rx.recv().await;
    assert!(msg.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p wezterm-utils-daemon -- test_subscribe_enables_broadcast`
Expected: FAIL — `add_connection` may not exist with that signature. Adjust to match actual `ConnectionManager` API.

- [ ] **Step 3: Wire subscribe into router handler**

In `router.rs`, modify `handle_subscribe()`:

```rust
async fn handle_subscribe(
    &self,
    connection_id: &str,
    subscriptions: Vec<crate::protocol::EventSubscription>,
) -> Result<Value> {
    let connection = self
        .connection_manager
        .get_connection(connection_id)
        .ok_or_else(|| DaemonError::UtilityNotFound(connection_id.to_string()))?;

    let count = subscriptions.len();
    connection.subscribe(subscriptions);

    info!(
        connection_id = %connection_id,
        count = count,
        "Subscriptions added"
    );

    Ok(json!({
        "status": "subscribed",
        "count": count,
    }))
}
```

- [ ] **Step 4: Wire unsubscribe into router handler**

Same pattern for `handle_unsubscribe()`:

```rust
async fn handle_unsubscribe(
    &self,
    connection_id: &str,
    event_types: Vec<String>,
) -> Result<Value> {
    let connection = self
        .connection_manager
        .get_connection(connection_id)
        .ok_or_else(|| DaemonError::UtilityNotFound(connection_id.to_string()))?;

    let count = event_types.len();
    connection.unsubscribe(&event_types);

    info!(
        connection_id = %connection_id,
        event_types = ?event_types,
        "Unsubscribed from events"
    );

    Ok(json!({
        "status": "unsubscribed",
        "count": count,
    }))
}
```

- [ ] **Step 5: Remove `#[expect(dead_code)]` from subscribe/unsubscribe on Connection**

These methods are now called by the router — remove the dead_code annotations.

- [ ] **Step 6: Run tests**

Run: `cargo test -p wezterm-utils-daemon`
Expected: All tests pass including the new subscribe/broadcast test

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(daemon): wire subscribe/unsubscribe into router — enables event delivery"
```

### Task 2: Implement keep-alive heartbeat polling

The `KEEP_ALIVE_INTERVAL` constant and `ping()` client method exist but no server-side heartbeat loop runs. Implement it so stale connections get cleaned up proactively.

**Files:**
- Modify: `wezterm-utils-daemon/src/server.rs` (add heartbeat task)
- Modify: `wezterm-utils-daemon/src/connections.rs` (use `connected_at` for uptime metrics in status)

- [ ] **Step 1: Add heartbeat cleanup task to server startup**

In the daemon server's main loop (wherever `ConnectionManager` is created), spawn a background task:

```rust
let cm_clone = connection_manager.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        let removed = cm_clone.cleanup_stale_connections();
        if removed > 0 {
            info!("Heartbeat cleanup: removed {} stale connections", removed);
        }
    }
});
```

- [ ] **Step 2: Use `connected_at` in status response**

In `router.rs` `handle_status()`, the `DaemonStatus` struct could include per-connection uptime. At minimum, verify `connected_at` is readable. If the status endpoint doesn't expose per-connection info, add connection count with oldest/newest connected_at to the response.

- [ ] **Step 3: Remove `#[expect(dead_code)]` from `KEEP_ALIVE_INTERVAL` and `connected_at`**

Both are now used — remove the annotations.

- [ ] **Step 4: Run tests**

Run: `cargo test -p wezterm-utils-daemon`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(daemon): add heartbeat cleanup task, use connected_at for status metrics"
```

### Task 3: Audit daemon writer path (Tier 3.K)

TODO.md flags: "current `handle_connection` writer task appears to use a fresh local channel instead of `Connection.tx`." Verify and fix.

**Files:**
- Modify: `wezterm-utils-daemon/src/connections.rs` (or `server.rs`, wherever `handle_connection` lives)

- [ ] **Step 1: Read `handle_connection` and trace the writer channel**

Find where `handle_connection` creates the writer task. Verify whether it uses `Connection.tx` or creates a new channel. If it creates a new channel, that's the bug — messages sent via `Connection.tx` (from the router) would never reach the writer.

- [ ] **Step 2: Fix the writer to use `Connection.tx`**

The fix should ensure the writer task reads from the same `rx` end that corresponds to `Connection.tx`. The typical pattern:

```rust
let (tx, rx) = mpsc::unbounded_channel();
let connection = Connection::new(tx);  // Router sends via this tx
// Writer task reads from rx:
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        // Write msg to the socket
    }
});
```

If `handle_connection` creates a SECOND channel, remove it and use the one from `Connection::new()`.

- [ ] **Step 3: Write test verifying end-to-end message delivery**

Test: send a message via `Connection.tx`, verify it arrives at the writer's output.

- [ ] **Step 4: Run tests**

Run: `cargo test -p wezterm-utils-daemon`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(daemon): use Connection.tx for writer task — fixes message delivery (Tier 3.K)"
```

---

## Phase 2: Complete Module Framework Dead Fields

### Task 4: Implement unwatch using WatchSubscription metadata

The `path` and `recursive` fields on `WatchSubscription` were scaffolded for unwatch/rewatch operations. Implement the unwatch path that uses them.

**Files:**
- Modify: `wezterm-module-framework/src/modules/watcher/mod.rs`

- [ ] **Step 1: Add unwatch method to WatcherModuleHandle**

The `unwatch()` method should look up the subscription by ID, log the path being unwatched (using the `path` field), drop the watcher, and join the forwarder thread:

```rust
pub fn unwatch(&self, id: WatchCallbackId) -> bool {
    let mut subs = self.subscriptions.lock();
    if let Some(mut sub) = subs.remove(&id) {
        log::info!("Unwatching {:?} (recursive={})", sub.path, sub.recursive);
        drop(sub.watcher);
        if let Some(handle) = sub.forwarder_handle.take() {
            let _ = handle.join();
        }
        true
    } else {
        false
    }
}
```

- [ ] **Step 2: Wire unwatch into the Lua API**

Find where `wezterm.watcher.watch()` is registered in the Lua API. Add a corresponding `wezterm.watcher.unwatch(id)` Lua function that calls `handle.unwatch(id)`.

- [ ] **Step 3: Remove `#[expect(dead_code)]` from path and recursive fields**

Both fields are now read by `unwatch()` — remove the annotations.

- [ ] **Step 4: Run tests**

Run: `cargo test -p wezterm-module-framework`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(watcher): implement unwatch using subscription metadata — completes Lua API"
```

---

## Phase 3: Complete fs-explorer Dead Code

### Task 5: Finish fs-explorer scaffolded features

The fs-explorer has 5 dead code items that are partially-built features from the customization plan (Phase 3.1).

**Files:**
- Modify: `wezterm-fs-explorer/src/error.rs`
- Modify: `wezterm-fs-explorer/src/ipc.rs`
- Modify: `wezterm-fs-explorer/src/keybindings.rs`
- Modify: `wezterm-fs-explorer/src/search.rs`

- [ ] **Step 1: Read each file to understand what was intended**

Read `error.rs`, `ipc.rs` (specifically `connect_timeout`), `keybindings.rs`, `search.rs` (the `indices` field). For each:
- If the feature has a clear completion path (e.g., `SearchResult::indices` for highlighting), implement it
- If it's scaffolding for a feature that doesn't have consumers yet (e.g., `ExplorerError` if nothing uses it), wire it in or add the TODO justification with a concrete plan

- [ ] **Step 2: For `SearchResult::indices` — wire into UI highlighting**

The `indices` field was meant for highlighting matched characters in fuzzy search results. Find where `SearchResult` is consumed in the UI rendering and use `indices` to apply highlight styling.

- [ ] **Step 3: For `ExplorerError` — use it or convert to TODO with plan**

If `ExplorerError` has variants that map to existing `anyhow::Error` uses, switch those call sites to use `ExplorerError`. If it's too early, add a concrete reason to the `#[expect]`.

- [ ] **Step 4: For `connect_timeout` — decide keep or defer**

If `connect_timeout` is used by the IPC client connection logic, wire it in. If the base `connect()` is sufficient, convert the `#[expect]` reason to explain when it would be needed (e.g., "for daemon health-check probe with bounded wait").

- [ ] **Step 5: For `KeyBindings` — wire into help display or remove**

If there's a `--help-keys` flag or `?` keybinding in the TUI, wire `KeyBindings` into the help display. Otherwise, if the keybinding help is rendered inline in `ui.rs`, this struct may be redundant.

- [ ] **Step 6: Verify compilation and tests**

Run: `cd wezterm-fs-explorer && cargo check && cargo test`
Expected: Clean

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(fs-explorer): complete scaffolded features — search highlighting, error types, keybindings"
```

---

## Phase 4: Bring fs-explorer into workspace

### Task 6: Add wezterm-fs-explorer to workspace

Per customization plan Phase 3.1: "Add to workspace (optional — now possible with gix)". The gix migration is complete, so this is now safe.

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `wezterm-fs-explorer/Cargo.toml`

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`:

```toml
members = [
  # ... existing ...
  "wezterm-fs-explorer",
]
```

- [ ] **Step 2: Remove standalone workspace declaration**

In `wezterm-fs-explorer/Cargo.toml`, remove any `[workspace]` section.

- [ ] **Step 3: Align shared dependencies to workspace versions**

For deps that already exist in `[workspace.dependencies]` (serde, serde_json, anyhow, clap, gix, tokio, etc.), change fs-explorer's Cargo.toml to use `dep.workspace = true`. Keep TUI-specific deps (ratatui, crossterm) as direct deps.

- [ ] **Step 4: Verify workspace build**

Run: `cargo check --workspace`
Expected: All workspace members compile

- [ ] **Step 5: Commit**

```bash
git commit -m "build: add wezterm-fs-explorer to workspace (gix migration enables this)"
```

---

## Phase 5: Split daemon into lib+bin, add subcommands

### Task 7: Split daemon into library + binary targets

**Files:**
- Modify: `wezterm-utils-daemon/Cargo.toml`
- Create: `wezterm-utils-daemon/src/lib.rs`
- Modify: `wezterm-utils-daemon/src/main.rs`

- [ ] **Step 1: Add lib target to Cargo.toml**

```toml
[lib]
name = "wezterm_utils_daemon"
path = "src/lib.rs"

[[bin]]
name = "wezterm-utils-daemon"
path = "src/main.rs"
```

- [ ] **Step 2: Create lib.rs re-exporting public API**

```rust
pub mod client;
pub mod config;
pub mod connections;
pub mod error;
pub mod protocol;
pub mod router;
pub mod server;
```

- [ ] **Step 3: Refactor main.rs to import from library**

Replace `mod` declarations with `use wezterm_utils_daemon::*` imports. Extract the server startup logic into a public function (`pub async fn run_server(...)`) in `server.rs` if not already exposed.

- [ ] **Step 4: Verify both targets compile**

Run: `cargo check -p wezterm-utils-daemon --lib && cargo check -p wezterm-utils-daemon`
Expected: Both clean

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(daemon): split into library + binary targets for embedding"
```

### Task 8: Add `wezterm daemon`, `wezterm watch`, `wezterm explore` subcommands

**Files:**
- Modify: `wezterm/Cargo.toml`
- Create: `wezterm/src/daemon.rs`
- Create: `wezterm/src/watch_cmd.rs`
- Create: `wezterm/src/explore.rs`
- Modify: `wezterm/src/main.rs`
- Modify: `wezterm-watch/src/lib.rs` (expose CLI entry point)
- Modify: `wezterm-fs-explorer/src/lib.rs` (expose CLI entry point)

- [ ] **Step 1: Add dependencies to wezterm/Cargo.toml**

```toml
wezterm-utils-daemon = { path = "../wezterm-utils-daemon" }
wezterm-watch = { path = "../wezterm-watch" }
wezterm-fs-explorer = { path = "../wezterm-fs-explorer" }
tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2: Expose entry points from library crates**

In `wezterm-watch/src/lib.rs`, add a public function that runs the watcher with given args.
In `wezterm-fs-explorer/src/lib.rs`, add a public function that runs the explorer TUI.
The daemon already exposes its server via the new lib.rs from Task 7.

- [ ] **Step 3: Create subcommand modules**

Create `wezterm/src/daemon.rs`, `watch_cmd.rs`, `explore.rs` — each with a clap `Parser` struct and a `run()` method that delegates to the library.

- [ ] **Step 4: Wire into SubCommand enum**

In `wezterm/src/main.rs`, add:

```rust
mod daemon;
mod watch_cmd;
mod explore;

// In SubCommand enum:
    #[command(name = "daemon", about = "Run the IPC utility daemon")]
    Daemon(daemon::DaemonCommand),

    #[command(name = "watch", about = "Watch files for changes with git integration")]
    Watch(watch_cmd::WatchCommand),

    #[command(name = "explore", about = "Interactive filesystem explorer")]
    Explore(explore::ExploreCommand),
```

- [ ] **Step 5: Verify compilation and help output**

Run: `cargo check -p wezterm && cargo run -p wezterm -- --help`
Expected: New subcommands appear in help

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(cli): add daemon/watch/explore subcommands — all utilities accessible via wezterm.exe"
```

---

## Phase 6: Wire daemon client into module framework

### Task 9: Add daemon IPC bridge to module framework (Tier 4.K follow-up)

The module framework's client.rs doc says it's "designed for use from the module framework." Wire it in as an optional feature.

**Files:**
- Modify: `wezterm-module-framework/Cargo.toml`
- Create: `wezterm-module-framework/src/ipc.rs`
- Modify: `wezterm-module-framework/src/lib.rs`

- [ ] **Step 1: Add optional daemon dependency**

```toml
[dependencies]
wezterm-utils-daemon = { path = "../wezterm-utils-daemon", optional = true }

[features]
default = ["daemon-ipc"]
daemon-ipc = ["dep:wezterm-utils-daemon"]
```

- [ ] **Step 2: Create ipc.rs bridge**

```rust
//! Optional IPC bridge to wezterm-utils-daemon for cross-window panel state sync.

#[cfg(feature = "daemon-ipc")]
pub use wezterm_utils_daemon::client::DaemonClient;

#[cfg(feature = "daemon-ipc")]
pub async fn try_connect() -> Option<DaemonClient> {
    match DaemonClient::connect().await {
        Ok(client) => Some(client),
        Err(e) => {
            log::debug!("Daemon not available (standalone mode): {e}");
            None
        }
    }
}
```

- [ ] **Step 3: Export from lib.rs**

```rust
pub mod ipc;
```

- [ ] **Step 4: Verify with and without feature**

Run: `cargo check -p wezterm-module-framework && cargo check -p wezterm-module-framework --no-default-features`
Expected: Both compile

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(module-framework): add optional daemon IPC bridge for cross-window panel sync"
```

---

## Phase 7: Verification

### Task 10: Final integration verification

- [ ] **Step 1: Grep for remaining dead_code annotations**

```bash
grep -rn "dead_code" wezterm-utils-daemon/src/ wezterm-module-framework/src/ wezterm-watch/src/ wezterm-fs-explorer/src/
```

Expected: Zero results (all items either used or have concrete completion plans with non-TODO reasons)

- [ ] **Step 2: Verify workspace builds**

Run: `cargo check --workspace`

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`

- [ ] **Step 4: Verify subcommands**

```bash
cargo run -p wezterm -- daemon --help
cargo run -p wezterm -- watch --help
cargo run -p wezterm -- explore --help
```

- [ ] **Step 5: Update TODO.md**

Mark completed tiers:
- Tier 3.K: DONE (daemon writer path)
- Update Tier 4 items with new status

- [ ] **Step 6: Commit**

```bash
git commit -m "chore: update TODO.md — daemon integration, subcommands, dead code elimination complete"
```

---

---

## Phase 8: Lua Script Integration (Follow-up)

The `codex_ui` panel system currently launches utilities as **external binaries** (`~/bin/wezterm-fs-explorer.exe`, `~/bin/wezterm-watch.exe`) rather than using the module framework's Lua APIs. Now that all crates are integrated, the Lua layer should be updated.

### Current Lua Architecture

```
.wezterm.lua → loads codex_ui/panels.lua
  ↓
codex_ui/panels.lua → panel toggle events (Alt+1/2/3)
  ↓
codex_ui/wezterm-utils.lua → launches external binaries via SplitPane
  config.explorer_bin = '~/bin/wezterm-fs-explorer.exe'
  config.watcher_bin = '~/bin/wezterm-watch.exe'
```

### Target Lua Architecture

```
.wezterm.lua → loads codex_ui/panels.lua
  ↓
codex_ui/panels.lua → panel toggle events
  ↓
Two modes:
  1. Module API mode (preferred when GUI running):
     wezterm.fs_explorer.spawn({dir = cwd})    -- native pane
     wezterm.watcher.watch(dir, {recursive=true})  -- background service
     wezterm.watcher.poll_events(50)           -- event polling

  2. Subcommand mode (fallback, SSH, mux-server):
     SplitPane { args = {'wezterm', 'explore', dir} }
     SplitPane { args = {'wezterm', 'watch', dir} }
```

### Task 11: Update codex_ui Lua scripts to use module APIs

**Files:**
- Modify: `codex_ui/wezterm-utils.lua` — Add module API path alongside binary path
- Modify: `codex_ui/panels.lua` — Use module APIs when available

- [ ] **Step 1**: In `wezterm-utils.lua`, add detection for whether the module framework API is available:
```lua
local function has_module_api()
  return wezterm.watcher ~= nil and wezterm.fs_explorer ~= nil
end
```

- [ ] **Step 2**: Add module-API-based panel launchers alongside the existing binary launchers:
```lua
function M.explorer_split_native(window, pane, directory)
  if has_module_api() then
    local id, err = wezterm.fs_explorer.spawn({dir = directory})
    if id then return id end
    wezterm.log_warn('Module spawn failed: ' .. (err or 'unknown'), ', falling back to binary')
  end
  -- Fallback to binary mode
  return M.explorer_split(directory)
end
```

- [ ] **Step 3**: Update panel toggle handlers to prefer module API mode
- [ ] **Step 4**: Add daemon status Lua binding (Tier 4.K follow-up):
```lua
-- Future: wezterm.daemon.status(), wezterm.daemon.ping()
-- Requires new Lua API registration in module framework
```

### Task 12: Add event-driven file watch callbacks (replaces polling)

**Files:**
- Modify: `wezterm-module-framework/src/modules/watcher/mod.rs` — Add WezTerm event emission
- Modify: Lua scripts to use `wezterm.on('file-watch-event', handler)` instead of polling

Currently Lua must poll with `wezterm.watcher.poll_events()`. The preferred pattern is event-driven:

```lua
wezterm.on('file-watch-event', function(window, pane, event)
  wezterm.log_info('File changed: ' .. event.path .. ' (' .. event.kind .. ')')
end)
```

This requires the watcher module to emit WezTerm events via the Mux notification system rather than buffering in an internal queue.

### Task 13: Add daemon client Lua bindings

**Files:**
- Modify: `wezterm-module-framework/src/ipc.rs` — Add Lua API registration
- Create: Lua API surface for `wezterm.daemon.*`

```lua
-- Planned API:
wezterm.daemon.ping()          -- health check
wezterm.daemon.status()        -- connection count, uptime
wezterm.daemon.broadcast(event_type, data)  -- cross-window state sync
wezterm.daemon.subscribe(event_type)        -- receive events
```

This connects the daemon IPC bridge (Task 9) to the Lua layer, enabling cross-window panel state sync from `.wezterm.lua`.

---

## Scope Exclusions

- **wezterm-benchmarks**: Dev-only crate with heavy deps (criterion, prometheus, warp, mimalloc). Stays as separate binaries. Not integrated into wezterm.exe.
- **AI/LLM integration (Tier 4.N)**: Long-term goal from customization plan. Not in scope here.
- **Upstream wezterm code**: No changes to upstream crates.

## Risk Notes

1. **tokio/smol runtime boundary**: The daemon uses tokio; wezterm-gui uses smol. The `wezterm daemon` subcommand creates its own tokio runtime (fine). If module-framework IPC is called from GUI code (smol context), use `tokio::runtime::Builder` to create a dedicated runtime for IPC calls, or use blocking bridge.

2. **fs-explorer workspace migration**: DONE — resolved by aligning shared deps, keeping gix at 0.68.

3. **Daemon writer path (Tier 3.K)**: DONE — verified correct, not a bug. Writer uses Connection.tx properly.

4. **Lua module API availability**: The module framework Lua APIs (`wezterm.watcher`, `wezterm.fs_explorer`) are only available when running inside wezterm-gui. CLI subcommands and mux-server mode don't have these APIs. The Lua scripts must detect availability and fall back to binary/subcommand mode.

5. **codex_ui ownership**: The `codex_ui/*.lua` files are Codex-owned. Lua changes in Tasks 11-13 should be coordinated via the agent bus.
