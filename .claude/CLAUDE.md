# WezTerm Project — Claude Session Instructions

> Project-specific instructions that supplement the root CLAUDE.md.
> This file is auto-discovered by Claude Code when working in the wezterm repo.

## Session Startup Checklist

1. **Check the agent bus** before starting work:
   ```bash
   agent-bus-http.exe read-direct --agent-a claude --agent-b codex --limit 10 --encoding toon
   agent-bus-http.exe presence-list
   ```

2. **Set presence**:
   ```bash
   agent-bus-http.exe presence --agent claude --status online --capability "rust-dev,coordination" --ttl-seconds 1800
   ```

3. **Check TODO.md** for current task assignments and tiers.

4. **Check for resource locks** before building or editing shared files:
   ```bash
   agent-bus-http.exe read-direct --agent-a claude --agent-b codex --limit 5 --encoding toon
   ```

## Build Commands (Always Use These)

```powershell
# Standard build (runs quick-check first: fmt + ast-grep + clippy)
just build

# Release with SIMD optimizations
just release

# Test (preferred — uses nextest + sccache)
just test-nextest

# Full validation (everything: fmt, clippy, ast-grep, nextest, docs)
just full-local-ci

# Custom crate linting only
just lint-ast-grep

# Safe auto-fix (prefer-expect-over-allow, remove-redundant-format)
just ast-grep-fix-safe
```

## Resource Protocol (MANDATORY for multi-agent work)

Before touching exclusive resources, post on the direct channel:

```bash
# Before building on default target/
agent-bus-http.exe post-direct --from-agent claude --to-agent codex \
  --body "RESOURCE_START resource=target/ mode=exclusive cmd=cargo nextest run eta=5min"

# After done
agent-bus-http.exe post-direct --from-agent claude --to-agent codex \
  --body "RESOURCE_DONE resource=target/ status=ok"

# For parallel builds without contention
CARGO_TARGET_DIR=C:/Users/david/.cache/claude/<task> cargo check -p <crate>
```

**Exclusive resources** (require RESOURCE_START/DONE):
- `target/` (default cargo target dir)
- `Cargo.lock` (dependency changes)
- `~/bin/*` (binary installs)
- `~/.wezterm.lua` and `~/.config/wezterm/**` (live config)
- Windows Terminal `settings.json`

## Rust Guidelines Enforcement

All Rust code must follow [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/).

**Automated enforcement**:
- `sg scan` — ast-grep rules in `rules/rust/` (13 rules, P0 blocking)
- `just clippy` — clippy via `tools/hooks/Invoke-WorkspaceRustChecks.ps1`
- `lefthook` / `pre-commit` — runs on every commit

**Key rules to remember**:
- `unwrap()` / `expect()` only in tests — use `?` or `Result` in production code
- Every `unsafe` block needs a `// SAFETY:` comment
- Use `#[expect(lint)]` not `#[allow(lint)]`
- No `static mut` — use `OnceLock`, `Mutex`, or atomics
- All public types implement `Debug`
- No `println!` in library code — use `log::info!` / `tracing::info!`

## Jules Integration

Use Jules for async tasks that don't need immediate results:

```bash
# PR review
jules new --repo David-Martel/wezterm "Review PR #XXXX"

# Test generation
jules new "Write tests for <file>"

# Security audit
jules new "Security audit of <crate>"

# Pull results
jules remote pull --session <ID> --apply
```

Jules config: `.jules` in repo root. Sessions tracked via agent bus.

## File Ownership (Current)

| Owner | Files |
|-------|-------|
| Claude | `wezterm-utils-daemon/src/*`, `wezterm-module-framework/src/*`, `wezterm-watch/src/*`, `wezterm-benchmarks/src/*`, `tools/CargoTools/*` |
| Codex | `wezterm-fs-explorer/src/*`, `codex_ui/*.lua`, `.wezterm.lua`, `wezterm-gui/src/termwindow/render/*` |
| Shared | `CLAUDE.md`, `AGENTS.md`, `TODO.md`, `docs/plans/*` |

Check agent bus for current claims before editing files outside your ownership.

## Critical Gotchas

1. **Config reload storm**: Never write files inside `~/.config/wezterm/` — triggers WezTerm file watcher reload loop. Panel state uses `~/.local/state/wezterm-utils/`.

2. **DLL dependencies**: `wezterm.exe` in `~/bin/` requires `conpty.dll`, `libEGL.dll`, `libGLESv2.dll`, `OpenConsole.exe` alongside it. `build-all.ps1` copies them automatically.

3. **sccache + clippy**: Don't use `RUSTC_WRAPPER=sccache` with clippy — causes `-vV` probe failure. `just clippy` handles this.

4. **Build lock contention**: Multiple `cargo build` commands fight over `target/` lock. Use `CARGO_TARGET_DIR=C:/Users/david/.cache/claude/<task>` for parallel builds.

5. **PR #7686**: Our fork's PR against upstream wezterm. All pushes to `main` auto-update it.

6. **Security findings**: See `SECURITY_AUDIT.md` — HIGH severity symlink following in fs-explorer needs fixing.
