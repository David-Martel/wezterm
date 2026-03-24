# WezTerm Resource Coordination

This document defines the shared resource-use protocol for concurrent Codex and Claude work in the `wezterm` repo.

The goal is simple: keep parallel work fast without tripping over shared locks, installed binaries, runtime state, or user-facing config.

## Active Locks

Active locks are tracked on the direct `codex` <-> `claude` bus thread, not by repeatedly editing this file.

Use the `wezterm-resource-coordination-20260324` thread and the `RESOURCE_START` / `RESOURCE_DONE` message formats below as the live source of truth.

If a task needs a continuously visible lock record, post a fresh `RESOURCE_START` update with the current command, namespace, and ETA rather than mutating this document.

## Principles

1. Default to isolation, not waiting.
2. Share caches, isolate artifacts.
3. Serialize installs, config mutations, and release outputs.
4. Notify before taking an exclusive resource.
5. Release the resource explicitly when done.
6. If there is a conflict, prefer rerouting to a private namespace over blocking another agent.

## Resource Classes

### Shared Cooperative Resources

These are safe to use concurrently without prior approval:

- `sccache` lookups and writes
- read-only repo inspection (`rg`, `git status`, docs/spec review, benchmarks review)
- agent-bus reads and normal status messages
- local scratch logs written to agent-specific paths

### Shared Resources Requiring Unique Namespacing

These may run concurrently only when each agent uses a private output path:

| Resource | Rule |
|----------|------|
| `cargo build/check/test/nextest/bench` | Set a private `CARGO_TARGET_DIR` per agent/task |
| `cargo llvm-cov`, timing reports, nextest archives | Use agent-specific output filenames/directories |
| ad hoc benchmark output | Write to agent-specific files under `.cache/`, `target/`, or temp dirs |

Recommended per-agent target-dir pattern:

- `codex`: `C:/Users/david/.cache/codex/<task>`
- `claude`: `C:/Users/david/.cache/claude/<task>`

Using the repo-default `target/` is not the default path for parallel work.

### Exclusive Resources

These require a direct bus notification before use, and must not be used concurrently:

| Resource | Why exclusive |
|----------|---------------|
| repo-default `target/` | shared cargo artifact lock |
| `Cargo.lock` and dependency graph mutations | affects all workspace builds |
| `~/bin/*` installs and overwrites | changes shared runtime binaries |
| `~/.local/wezterm/*` runtime payloads | changes shared launch/runtime behavior |
| `build-all.ps1`, `install-verification.ps1`, `uninstall.ps1` when executing install/uninstall flows | mutates shared outputs |
| `C:/Users/david/.wezterm.lua` and `C:/Users/david/.config/wezterm/**` | changes the live terminal runtime |
| Windows Terminal `settings.json` | changes the shared launcher/runtime integration |
| GUI smoke tests against installed binaries | can collide with installs and live config reloads |
| `sccache --zero-stats`, `sccache --stop-server`, manual cache deletion | mutates shared compiler cache state |

## Notification Protocol

Use the direct `codex` <-> `claude` channel for exclusive resources and high-cost shared actions.

### Start Message

Send before taking an exclusive resource:

```text
RESOURCE_START
resource=<path or name>
mode=<exclusive|shared-namespaced>
cmd=<command summary>
namespace=<target dir / output path / n/a>
eta=<rough duration>
```

### Completion Message

Send after finishing:

```text
RESOURCE_DONE
resource=<path or name>
status=<ok|failed|abandoned>
follow_up=<anything the other agent now needs to know>
```

### Ack Requirement

Ack is expected for:

- installs to `~/bin` or `~/.local/wezterm`
- `.wezterm.lua` / `~/.config/wezterm/**` mutations
- Windows Terminal config changes
- use of repo-default `target/`
- `Cargo.lock` changes
- any cache-reset operation

If no ack arrives and the work can be namespaced, reroute it to a private namespace instead of waiting.

## Operational Rules

1. Rust compilation work should use private `CARGO_TARGET_DIR` values by default.
2. `sccache` is a shared service, not a lock. Do not stop, zero, or delete it without notice and ack.
3. Installing rebuilt binaries into `~/bin` is exclusive work. Announce it first, then announce completion.
4. User-facing runtime config (`.wezterm.lua`, `ui-preferences.lua`, `codex_ui/*.lua`, WT `settings.json`) is exclusive while being edited or smoke-tested.
5. If a task only needs validation, prefer smoke tests against namespaced build outputs before touching installed binaries.
6. If one agent already holds an exclusive resource, the other should either wait or switch to a non-conflicting lane.

## Current Recommended Split

This is the current default split unless the agents agree otherwise on the bus:

| Agent | Preferred ownership |
|-------|---------------------|
| `codex` | `.wezterm.lua`, `~/.config/wezterm/**`, `wezterm-fs-explorer`, render-path investigation |
| `claude` | daemon, module framework, watcher, workspace-wide Rust integration, repo-wide validation |

This is a coordination default, not a hard prohibition. Shared ownership is allowed when explicitly coordinated.

## Examples

### Parallel nextest without contention

- `claude`: `CARGO_TARGET_DIR=C:/Users/david/.cache/claude/nextest cargo nextest run --workspace`
- `codex`: `CARGO_TARGET_DIR=C:/Users/david/.cache/codex/nextest cargo nextest run --workspace`

Both can share `sccache` safely.

### Exclusive install flow

1. Post `RESOURCE_START` for `~/bin/wezterm-fs-explorer.exe`
2. Copy/install the binary
3. Run smoke test
4. Post `RESOURCE_DONE`

### Exclusive config flow

1. Post `RESOURCE_START` for `C:/Users/david/.wezterm.lua`
2. Edit config
3. Smoke test with the intended runtime
4. Post `RESOURCE_DONE`
