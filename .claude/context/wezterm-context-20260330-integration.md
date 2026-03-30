---
id: ctx-wezterm-integration-20260330
title: "WezTerm Integration Session: Upstream Sync + Crate Integration"
date: 2026-03-30
session_date: 2026-03-29
scope: upstream-sync, dep-refresh, crate-integration, lua-api, cli-subcommands
agents: claude, codex
base_commit: d3695a0da
head_commit: 04f6bbec8
uncommitted: 19 files (11 modified, 8 untracked)
previous_context: wezterm-context-20260204-testing-complete.md
---

# WezTerm Integration Session -- 2026-03-29

## Session Scope

Three major work streams completed in a single session:

1. **Upstream sync** -- fork fully caught up with wezterm/wezterm, Cargo.lock refreshed (253 deps touched, 43 new/updated crate entries), 10 stale Dependabot branches pruned.
2. **Code simplification review** -- build profile conflict fixes, dead code cleanup in fs-explorer, Lua config sanitizer edge-case tests, CI caching improvements.
3. **Full crate integration plan** -- 13 tasks across daemon, module-framework, fs-explorer, CLI, and Lua executed end-to-end.

## Commits (14 total: d3695a0da..04f6bbec8)

### Upstream Sync + Dependency Refresh
| Hash | Subject |
|------|---------|
| `23bcb17cb` | chore: refresh deps, fix CI formatting, bump actions/checkout to v5 |

### Crate Integration (12 commits)
| Hash | Subject | Area |
|------|---------|------|
| `88934d995` | feat(daemon): wire subscribe/unsubscribe into router | Daemon |
| `14e8dde11` | docs(daemon): document writer channel flow, verify Tier 3.K is not a bug | Daemon |
| `b70e36907` | feat(watcher): use subscription metadata in unwatch, fix forwarder thread join ordering | Watcher |
| `dd14f76d6` | feat(fs-explorer): complete scaffolded features and replace dead code placeholders | FS Explorer |
| `2f44a322f` | build: add wezterm-fs-explorer to workspace (gix migration enables this) | Build |
| `79b9207a7` | refactor(daemon): split into library + binary targets for embedding | Daemon |
| `f20ed5d88` | feat(module-framework): add optional daemon IPC bridge for cross-window panel sync | Module Framework |
| `7e70a8870` | feat(lua): integrate module APIs and subcommand fallbacks into panel system | Lua/Panels |
| `885037623` | build: fix profile conflicts, improve CI caching | Build |
| `2d65ae3ce` | feat(config): wire test module, add sanitizer edge case tests, debug logging | Config |
| `92e9b00a4` | feat(daemon): add heartbeat cleanup, connection uptime in status, error context | Daemon |
| `04f6bbec8` | feat(module-framework): event-driven watcher callbacks + daemon Lua bindings | Module Framework |
## Architecture Decisions

### 1. Daemon lib+bin split
wezterm-utils-daemon was refactored into src/lib.rs (public API: client, config, connections, error, protocol, router, server) plus src/main.rs (standalone binary entry point). This allows the daemon to be embedded directly into the wezterm CLI binary via "wezterm daemon start" without duplicating the server logic.

### 2. Feature-gated daemon IPC bridge
wezterm-module-framework/src/ipc.rs provides the cross-window panel sync bridge, gated behind the `daemon-ipc` Cargo feature. When disabled, `try_connect()` returns `None` and modules operate standalone. This avoids pulling tokio/daemon deps into builds that do not need IPC.

### 3. Blocking Lua-to-async bridge
Daemon Lua bindings (`wezterm.daemon.ping()`, `.status()`, `.broadcast()`, `.register()`) use a per-call throwaway `tokio::runtime::Builder::new_current_thread()` runtime. This is intentional: Lua calls are infrequent (panel toggles, status checks), the ~1ms overhead is negligible, and it avoids holding long-lived connections across Lua context reloads.

### 4. Four-tier panel launch fallback
codex_ui/panels.lua now resolves panel commands through a priority chain:
- Tier 1: Module framework native API (wezterm.watcher.*, wezterm.explorer.*)
- Tier 2: wezterm subcommand (wezterm explore, wezterm watch, wezterm daemon)
- Tier 3: Standalone binary (~/bin/wezterm-fs-explorer.exe, ~/bin/wezterm-watch.exe)
- Tier 4: Placeholder with build instructions

### 5. fs-explorer added to workspace
Previously standalone with its own Cargo.lock, wezterm-fs-explorer/ now participates in the workspace Cargo.toml. This was blocked until the gix migration was complete; the migration resolved conflicting dependency versions that previously prevented workspace inclusion.

### 6. Event-driven watcher callbacks
WatcherModule now supports two consumption modes: polling (`poll_events()`) and event-driven (`set_emit_events(true)` + `wezterm.on("file-watch-event", fn)`). The event-driven path uses MuxNotification forwarding so callbacks run on the GUI thread.

### 7. Subscription metadata in unwatch
`unwatch()` now uses subscription metadata stored in the connection subscription list to properly identify and remove event subscriptions, fixing a previous issue where unwatch could not find the subscription by event type alone. Thread join ordering in the forwarder was also corrected to prevent a deadlock on shutdown.
## Integration State Diagram

```
    wezterm CLI binary
    ==================
    wezterm/src/main.rs
    |
    +-- daemon_cmd.rs    --> wezterm-utils-daemon (lib)
    +-- watch_cmd.rs     --> wezterm-watch (lib)
    +-- explore_cmd.rs   --> wezterm-fs-explorer (lib)
    +-- validate_config.rs

    wezterm-module-framework
    ========================
    src/lib.rs
    +-- src/startup.rs           (GUI bootstrap hook)
    +-- src/ipc.rs               (daemon IPC bridge, feature-gated)
    |   +-- register_lua_api()   --> wezterm.daemon.{ping,status,broadcast,register}
    |   +-- try_connect()        --> DaemonClient
    +-- src/modules/watcher/     (file watcher module)
        +-- poll_events()
        +-- set_emit_events()
        +-- on_event() callback

    wezterm-utils-daemon
    ====================
    src/lib.rs  (pub: client, config, connections, error, protocol, router, server)
    src/main.rs (standalone binary entry)
    +-- router.rs
    |   +-- handle_subscribe()
    |   +-- handle_unsubscribe()
    |   +-- handle_broadcast()   --> broadcast_to_subscribers()
    |   +-- handle_register()
    |   +-- handle_status()      (includes connection_uptimes, oldest_connection_age)
    +-- connections.rs
    |   +-- Connection { tx, subscriptions, last_activity }
    |   +-- heartbeat_cleanup()  (periodic stale connection removal)
    |   +-- broadcast_to_subscribers()
    +-- client.rs                (async DaemonClient for IPC consumers)

    wezterm-fs-explorer          (now in workspace)
    ====================
    +-- Scaffolded features completed (app.rs, ipc_client.rs, keybindings.rs, search.rs, ui.rs)
    +-- Dead code placeholders removed

    codex_ui/panels.lua
    ====================
    +-- 4-tier fallback launch (module API -> subcommand -> binary -> placeholder)
    +-- wezterm.daemon.* integration for cross-window state sync
```
## What Remains (from TODO.md)

### Tier 2 (Phase 2+3) -- IN PROGRESS
- [ ] **2.F**: Settings feature tab / panel UX follow-up (Codex)

### Tier 3 (Phase 4 Rust Optimization) -- IN PROGRESS
- [ ] **3.H**: Status-bar click zones and richer Rust-side chrome hooks (Codex)
- [ ] **3.I**: Performance profiling -- Lua chrome vs Rust chrome benchmarks (Claude)
- [ ] **3.J**: DirectWrite tuning if rendering quality needs work (Both)

### Tier 4 (Long-Term)
- [ ] **4.K**: Hook module framework into config/src/lua.rs for Lua context setup
- [ ] **4.K2**: Register module domains with mux
- [ ] **4.K3**: Add module configuration parsing
- [ ] **4.L**: Create lua-api-crates/module-framework/ Lua API crate
- [ ] **4.M**: Example "hello world" module
- [ ] **4.N**: AI/LLM integration (mistral.rs, streaming, MCP client)
- [ ] **4.O**: Reach 85% test coverage for custom crates (currently ~68%)

### Tier 5 (Repo-Managed Runtime + Tooling)
- [ ] **5.P**: Validate repo-managed symlinked home config with GUI smoke test
- [ ] **5.Q**: Verify repo-vendored codex_ui stays in sync with live runtime
- [ ] **5.R**: Adopt agent-bus-http.exe TOON/compact-context flow
- [ ] **5.S**: Triage PostgreSQL jsonb serialization warning
- [ ] **5.T**: Validate Windows-local pre-commit + lefthook installation
- [ ] **5.U**: Expand ast-grep rule coverage with test-aware exclusions
- [ ] **5.V**: Extend warnings-as-errors to remaining build helpers
- [ ] **5.W**: Resolve ast-grep inline #[cfg(test)] exclusion parsing
- [ ] **5.X**: Reconcile global git core.hooksPath with repo-local lefthook

### Tier 6 (Jules-Assisted)
- [ ] **6.W**: Use Jules to expand integration/coverage for daemon + module-framework

### P0 Cross-Review
- [ ] Claude reviews retained plain-tab-bar cache patch
- [ ] Codex reviews wezterm-gui/src/main.rs module init hook
- [ ] Run full workspace nextest after both patches merged

### Uncommitted Work (19 files)
Modified: Cargo.lock, build-all.ps1, codex_ui/chrome.lua, install-verification.ps1, wezterm-fs-explorer/src/{app,error,lib,main}.rs, wezterm-watch/src/lib.rs, wezterm/Cargo.toml, wezterm/src/main.rs

Untracked: .claude/settings.local.json, codex.TODO.md, codex_ui/validator.lua, docs/superpowers/, wezterm/src/{daemon_cmd,explore_cmd,validate_config,watch_cmd}.rs
## Key File Changes Map

| File | Change Type | Summary |
|------|-------------|---------|
| Cargo.toml (workspace) | Modified | Added wezterm-fs-explorer to workspace members |
| Cargo.lock | Modified | 253 deps refreshed (1402 insertions, 994 deletions) |
| wezterm-utils-daemon/src/lib.rs | New | Library root exposing all daemon modules |
| wezterm-utils-daemon/src/main.rs | Refactored | Thin binary entry calling into lib |
| wezterm-utils-daemon/src/router.rs | Modified | Added handle_subscribe(), handle_unsubscribe(), handle_broadcast() with subscriber matching |
| wezterm-utils-daemon/src/connections.rs | Modified | Added subscribe(), unsubscribe(), broadcast_to_subscribers(), heartbeat cleanup, connection uptime tracking |
| wezterm-module-framework/src/ipc.rs | New | Daemon IPC bridge + wezterm.daemon.* Lua API (4 functions + JSON conversion) |
| wezterm-module-framework/src/lib.rs | Modified | Added pub mod ipc |
| wezterm-module-framework/Cargo.toml | Modified | Added daemon-ipc feature, tokio/mlua/config deps |
| wezterm-module-framework/src/modules/watcher/mod.rs | Modified | Event-driven callbacks, subscription metadata in unwatch, forwarder thread fix |
| wezterm-module-framework/src/startup.rs | Modified | Event-driven watcher integration |
| wezterm-fs-explorer/Cargo.toml | Modified | Converted from standalone to workspace member |
| wezterm-fs-explorer/src/*.rs | Modified | Scaffolded features completed, dead code removed |
| codex_ui/panels.lua | Modified | 4-tier fallback launch, wezterm.daemon.* integration, subcommand detection |
| wezterm/src/main.rs | Modified | Added daemon_cmd, watch_cmd, explore_cmd subcommand modules |
| wezterm/src/daemon_cmd.rs | New (untracked) | wezterm daemon subcommand |
| wezterm/src/watch_cmd.rs | New (untracked) | wezterm watch subcommand |
| wezterm/src/explore_cmd.rs | New (untracked) | wezterm explore subcommand |
| wezterm/src/validate_config.rs | New (untracked) | Config validation subcommand |
| config/src/config.rs | Modified | Debug logging, test wiring |
| config/src/lib.rs | Modified | Test module wiring |
| .github/workflows/windows-ci.yml | Modified | CI caching improvements, actions/checkout v5 |

## Test Counts Per Crate

| Crate | #[test] Count | Notes |
|-------|---------------|-------|
| wezterm-fs-explorer | 108 | Scaffolded features now tested |
| wezterm-watch | 77 | Subscription metadata tests added |
| wezterm-module-framework | 32 | IPC bridge + JSON conversion tests (11 new) |
| wezterm-utils-daemon | 15 | Subscribe/unsubscribe + heartbeat tests |
| config | 14 | Sanitizer edge cases added |
| wezterm-benchmarks | 9 | Benchmark harness tests |
| **Total (custom crates)** | **255** | Up from 182 at last context snapshot |

## UX Redesign Phase Status

| Phase | Completion | Delta |
|-------|-----------|-------|
| Phase 1: Rendering + Config | 95% | No change |
| Phase 2: Chrome Overhaul | 85% | No change |
| Phase 3: Panel System | 85% | +10% (4-tier fallback, daemon integration) |
| Phase 4: Rust Investment | 40% | +15% (daemon lib/bin, IPC bridge, Lua API, CLI subcommands) |

## Previous Context

Testing phase complete (2026-02-04):
- 285 tests passing across wezterm-watch and wezterm-fs-explorer
- 85.54% coverage for wezterm-watch, 59.20% for wezterm-fs-explorer
- See wezterm-context-20260204-testing-complete.md
