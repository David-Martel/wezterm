# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WezTerm is a GPU-accelerated cross-platform terminal emulator and multiplexer written in Rust. It uses wgpu for rendering, supports terminal multiplexing (panes, tabs, windows), includes an SSH client with native tabs, and features Lua configuration with hot-reloading.

### Fork Policy

This is a **downstream fork** of `wezterm/wezterm` in David-Martel's GitHub account. It pulls meaningful updates from upstream but **never commits back**. The `upstream` remote is configured as fetch-only (push URL disabled). `gh` defaults to `David-Martel/wezterm`. No agent (Claude, Codex, Jules, Gemini) should create PRs, push, or contribute changes to `wezterm/wezterm`.

## Build and Development Commands

### Building the Project

**Windows (PowerShell)**:
```powershell
# Build using Just (recommended - includes sccache)
just build              # Standard build with sccache
just release            # Release build with optimizations
just clippy            # Run clippy linter (without sccache due to probe issues)
just test              # Run tests with sccache
just full-verify       # Full quality check (fmt, clippy, test, docs, sccache stats)

# Build all custom utilities (wezterm-fs-explorer, wezterm-watch)
.\build-all.ps1        # Builds and installs to $env:USERPROFILE\bin
```

**Unix/Linux/macOS (Make)**:
```bash
# Build all main binaries (wezterm, wezterm-gui, wezterm-mux-server, strip-ansi-escapes)
make build

# Build specific binary with release optimizations
cargo build --release -p wezterm-gui

# Quick type checking during development
cargo check
# Check specific no_std crates
cargo check -p wezterm-escape-parser
```

### Running Tests

```bash
# Run all tests using nextest (preferred test runner)
make test
# Or directly:
cargo nextest run

# Run tests for specific package
cargo nextest run -p wezterm-escape-parser

# Run single test with verbose output
cargo nextest run -p <package> <test_name>

# Run tests with standard cargo test
cargo test --all
```

### Code Formatting, Linting, and ast-grep

```bash
# Format + lint (Windows: use Just — routes through tools/hooks/ wrappers)
just fmt               # cargo fmt --all
just clippy            # strict clippy lane for custom crates + fs-explorer
just clippy-workspace  # explicit full-workspace lint; legacy warning debt still exists
just lint-ast-grep     # full ast-grep scan on custom crates / backlog surfacing
just lint-ast-grep-gate # changed-file safe gate used by build/CI paths
just ast-grep-fix-safe # auto-fix safe rules (prefer-expect-over-allow, remove-redundant-format)
just quick-check       # check + fmt + ast-grep + clippy (runs before every build)

# ast-grep (Microsoft Rust Guidelines enforcement)
sg scan -c sgconfig.yml                            # Config-based scan
sg scan -c sgconfig.yml wezterm-utils-daemon/src/  # Scan specific crate
sg scan -c sgconfig.yml --update-all --filter 'prefer-expect-over-allow|remove-redundant-format'  # Safe auto-fix only

# Direct cargo (any platform)
cargo fmt --all --check
cargo clippy --workspace --all-targets --no-deps -- -D warnings -A clippy::type_complexity
```

**ast-grep Rules** (`rules/rust/`): config-based enforcement for Microsoft Rust Guidelines. Safe auto-fix is intentionally limited to syntax-preserving rewrites such as `prefer-expect-over-allow` and `remove-redundant-format`. The broader unwrap/panic backlog is still tracked in [TODO.md](./TODO.md), so treat full-scan failures as useful debt discovery rather than a hook wiring failure.

**Git Hooks** (two systems available):
- **lefthook** (preferred): `lefthook.yml` — pre-commit (fmt, ast-grep, clippy) + pre-push (full tests)
- **pre-commit**: `.pre-commit-config.yaml` — same hooks via `pre-commit install`
- Hook scripts: `tools/hooks/Invoke-AstGrep.ps1`, `tools/hooks/Invoke-WorkspaceRustChecks.ps1`
- Machine note: this workstation currently uses a global `core.hooksPath=~/.git-hooks`; do not reset or override that globally without coordinating through [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) and [TODO.md](./TODO.md).

### Documentation

```bash
# Build documentation locally
make docs
# Or directly:
ci/build-docs.sh

# Serve documentation with auto-rebuild on changes
make servedocs
# Or directly:
ci/build-docs.sh serve
```

### Development Iteration

```bash
# Quick type-check during development (fastest feedback loop)
cargo check

# Run in debug mode for testing changes
cargo run

# Run with backtrace for debugging panics
RUST_BACKTRACE=1 cargo run

# Debug with gdb
cargo build
gdb ./target/debug/wezterm
```

## High-Level Architecture

### Workspace Structure

This is a Cargo workspace with 25 member crates organized by functionality.

**Target Directory** (`.cargo/config.toml`):
- Default: `./target/` (shared target dir is available but commented out in config)
- To enable shared: uncomment `target-dir = "C:/Users/david/.cargo/shared-target"` in `.cargo/config.toml`
- `[profile.dev]` uses `incremental = false` — sccache handles caching instead

**Key Configuration**:
```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]  # Static OpenSSL on Windows
```

**Workspace Members** (organized by functionality):

**Core Terminal Engine** (platform-agnostic):
- `term/` - Core terminal emulator implementation
- `wezterm-cell/` - Terminal cell representation
- `wezterm-surface/` - Terminal surface/screen buffer management
- `wezterm-escape-parser/` - ANSI escape sequence parser (no_std compatible)
- `vtparse/` - VT sequence parser

**GUI and Rendering**:
- `wezterm-gui/` - Main GUI application using wgpu for GPU acceleration
- `window/` - Window management abstraction layer
- `wezterm-font/` - Font handling and rendering

**Multiplexer**:
- `mux/` - Core multiplexer implementation
- `wezterm-mux-server/` - Standalone multiplexer server
- `wezterm-mux-server-impl/` - Server implementation details

**Configuration and Scripting**:
- `config/` - Configuration management with hot-reloading support
- `luahelper/` - Lua scripting integration
- `lua-api-crates/` - Collection of 13+ Lua API modules for extensibility

**Cross-Platform Support**:
- `pty/` - Portable pseudo-terminal implementation
- `wezterm-ssh/` - Native SSH client implementation
- `filedescriptor/` - Cross-platform file descriptor utilities

**Custom Utilities & Extensions**:
- `wezterm-module-framework/` - Module framework for AI/plugin integration
- `wezterm-utils-daemon/` - IPC server daemon for utility coordination
- `wezterm-benchmarks/` - Performance benchmark suite
- `wezterm-watch/` - File watcher with git integration (workspace member)
- `wezterm-fs-explorer/` - Filesystem explorer (standalone, not in workspace)

**Supporting Libraries**:
- `termwiz/` - Reusable terminal utilities library
- `promise/` - Async/promise utilities
- `wezterm-dynamic/` - Dynamic type system for configuration
- `wezterm-blob-leases/` - Blob lease management
- `wezterm-uds/` - Unix domain socket abstraction
- `wezterm-fs-utils/` - Filesystem utility helpers
- `wezterm-open-url/` - URL opening abstraction

### Key Design Patterns

1. **Separation of Concerns**: Terminal logic (`term/`) is completely separate from GUI (`wezterm-gui/`) and multiplexer (`mux/`)

2. **Platform Abstraction**: Window management and PTY operations are abstracted to support Linux, macOS, Windows, and BSDs

3. **GPU Acceleration**: Uses wgpu for efficient rendering across different graphics APIs

4. **Hot-Reloading Configuration**: Lua configuration can be changed without restarting the terminal

5. **Vendored Dependencies**: Critical C libraries (cairo, fontconfig, freetype, harfbuzz) are vendored in `deps/` for consistent builds

### Microsoft Rust Guidelines (Mandatory)

All code must follow the [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/). See [AGENTS.md](./AGENTS.md) for detailed agent instructions.

**Key Requirements:**

| Guideline | Rule |
|-----------|------|
| **M-UNSAFE** | Only use `unsafe` for: novel abstractions, benchmarked perf optimization, FFI |
| **M-CONCISE-NAMES** | Avoid weasel words (Service, Manager, Factory) |
| **M-PANIC-IS-STOP** | Panics = "stop now"; use `Result` for recoverable errors |
| **M-THROUGHPUT** | Design APIs for batched operations, exploit cache locality |
| **M-HOTPATH** | Identify and benchmark hot paths early |
| **M-PUBLIC-DEBUG** | All public types must implement `Debug` |
| **M-LINT-OVERRIDE-EXPECT** | Use `#[expect]` instead of `#[allow]` |

**Safety documentation for `unsafe`:**
```rust
// SAFETY: Explain preconditions and invariants maintained
unsafe { ... }
```

### Testing Approach

- Unit tests are colocated with source files using `#[cfg(test)]` modules
- Integration tests use helper classes in `term/src/test/`
- The `k9` assertion library is used for expressive test assertions
- Tests should include comments explaining their intent

**Test Runners**:
- **nextest** (preferred): `cargo nextest run` or `just test-nextest`
- **cargo test**: `cargo test --all` or `just test`

**Pre-commit Testing**:
- Quick tests run on changed crates only during pre-commit
- Full test suite with all features runs during pre-push

### CI/CD Pipeline

The project has extensive CI coverage with 40+ GitHub Actions workflows testing on:
- Linux: Debian, Ubuntu, CentOS, Fedora
- macOS
- Windows
- Various architecture combinations

Key workflows check formatting, run tests, and build packages for distribution.

**Pre-commit Hooks**: Configured via `.pre-commit-config.yaml`
- Install: `pre-commit install --hook-type pre-commit --hook-type pre-push`
- Pre-commit: Fast checks (fmt, clippy, quick tests, deny check, mdbook, doxygen)
- Pre-push: Full checks (clippy --all-features, test --all-features, full deny, mdbook, doxygen)

**Local CI Workflow**: Run full local validation
```bash
just full-local-ci     # Comprehensive validation (fmt, clippy, nextest, docs, arch docs)
```

### Build Optimization with sccache

The project uses `sccache` for accelerated builds via shared compilation cache:

**Configuration** (`.cargo/config.toml`):
```toml
[env]
SCCACHE_SERVER_PORT = "4400"
SCCACHE_CACHE_COMPRESSION = "zstd"
SCCACHE_CACHE_SIZE = "30G"
SCCACHE_DIR = "T:/RustCache/sccache"
OPENSSL_DIR = "C:/codedev/vcpkg/installed/x64-windows"
OPENSSL_NO_VENDOR = "1"
```

**Usage**:
```powershell
# Windows (via Justfile - automatically sets RUSTC_WRAPPER)
just build             # Uses sccache
just sccache-stats     # Show cache statistics
just sccache-zero      # Reset statistics

# Note: Clippy requires sccache disabled due to -vV probe failure
just clippy            # Automatically removes RUSTC_WRAPPER
```

**Manual sccache**:
```bash
# Set wrapper manually
export RUSTC_WRAPPER=sccache  # Unix
$env:RUSTC_WRAPPER="sccache"  # PowerShell

# Build with sccache
cargo build

# Check statistics
sccache --show-stats
```

### Custom Utilities

```powershell
# Build and install all custom utilities (wezterm-watch + wezterm-fs-explorer)
.\build-all.ps1                # Installs to $env:USERPROFILE\bin\
.\build-all.ps1 -Force         # Force rebuild
```

- **wezterm-fs-explorer**: Standalone (own Cargo.lock), build with `cd wezterm-fs-explorer && cargo build --release`
- **wezterm-watch**: Workspace member, build with `cargo build --release -p wezterm-watch`

## Build Tools

### PowerShell Tools (`tools/`)

| Tool | Purpose |
|------|---------|
| `Build-Integration.ps1` | Master build tools: `-Install`, `-HealthCheck`, `-Optimize`, `-Release` |
| `Invoke-Gix.ps1` | gix CLI wrapper: `-Stats`, `-UnreleasedCommits`, `-Changelog`, `-VersionBump` |
| `CargoTools/` module | Cargo build wrapping with sccache, preflight checks (`Import-Module .\tools\CargoTools\CargoTools.psd1`) |

### Justfile Targets

Run `just --list` for all 50+ targets. Key ones:

```bash
just quick-check          # check + fmt + ast-grep + clippy (gating — runs before build/release)
just full-local-ci        # fmt, clippy, ast-grep, nextest, docs, arch docs
just full-verify          # fmt, clippy, ast-grep, test, docs, sccache stats
just lint-ast-grep        # ast-grep scan on custom crates only
just ast-grep-fix-safe    # auto-fix safe ast-grep rules
just coverage             # Coverage via llvm-cov (custom crates only)
just release              # Release build with target-cpu=native SIMD optimizations
just bootstrap-tools      # Install all dev tools (nextest, llvm-cov, git-cliff)
```

**Release builds** use `-C target-cpu=native` for SIMD/CPU-native optimizations (set in Justfile and build-all.ps1).

### Release Automation

Configured via `release.toml` (cargo-smart-release) and `cliff.toml` (git-cliff conventional commits).

## Planning & Design Documents

All plans consolidated under `docs/`:
- **[TODO.md](./TODO.md)** — Current task tracking and agent ownership
- **[docs/plans/](./docs/plans/)** — Development plans (joint plan, UX redesign, customization roadmap, test plan)
- **[docs/specs/](./docs/specs/)** — Approved design specifications (UX redesign 4-phase spec)
- **[docs/design/](./docs/design/)** — Architecture documents (AI module design)
- **[JULES.md](./JULES.md)** — Jules (Google) async agent: CI/CD PR review, test generation, parallel exploration

Prompt/guidance files that should stay aligned with the current workflow:
- [AGENTS.md](./AGENTS.md)
- [CLAUDE.md](./CLAUDE.md)
- [.claude/CLAUDE.md](./.claude/CLAUDE.md)
- [JULES.md](./JULES.md)

**AI Assistant Module** ([docs/design/WEZTERM_AI_MODULE_DESIGN.md](./docs/design/WEZTERM_AI_MODULE_DESIGN.md)):
- Design spec for local LLM-based AI assistant integration
- `wezterm-module-framework/` crate provides the plugin/module infrastructure
- Module framework wired into GUI bootstrap (`wezterm-gui/src/main.rs`)

**Implementation Status**: Module framework integrated into GUI startup; daemon IPC client ready; AI/LLM integration pending

**Security**: See [SECURITY_AUDIT.md](./SECURITY_AUDIT.md) for Jules-generated audit (6 findings: 1 HIGH, 3 MEDIUM, 2 LOW)

### Jules (Google Async Agent)

Jules runs asynchronous code reviews, test generation, and security audits against the GitHub repo. See [JULES.md](./JULES.md) for full guide.

```bash
# Review a PR
jules new --repo David-Martel/wezterm "Review PR #XXXX for Rust quality"

# Generate tests
jules new "Write integration tests for wezterm-utils-daemon/src/client.rs"

# Pull results for review first
jules remote pull --session <ID>

# Check session status
jules remote list --session
```

**Jules config**: `.jules` in repo root defines project context, guidelines, and quality gates.
**Current practice**: Jules findings should be posted to the direct bus thread, converted into concrete [TODO.md](./TODO.md) items when actionable, and validated locally with `cargo check`, `cargo nextest`, `sg scan -c sgconfig.yml`, and clippy before any patch is applied. See [JULES.md](./JULES.md) and [TODO.md](./TODO.md) Tier 6 for active sessions.

## Important Development Notes

### Windows-Specific Considerations

1. **Justfile vs Makefile**:
   - Windows: Use `just` commands (PowerShell-based)
   - Unix/Linux/macOS: Use `make` commands (Bash-based)

2. **sccache Compatibility**:
   - Works with cargo build/test
   - **Does NOT work** with clippy (use `just clippy` which removes wrapper)
   - Check cache: `just sccache-stats`

3. **Static Linking**:
   - OpenSSL statically linked on Windows (`crt-static` feature)
   - Required for portable binaries

4. **OpenSSL via vcpkg**:
   - Pre-built OpenSSL from `C:/codedev/vcpkg/installed/x64-windows` (`OPENSSL_NO_VENDOR=1`)
   - Only needed for mux server TLS and legacy SSH backends
   - Default SSH uses pure-Rust `russh` (no OpenSSL required)

### Cross-Platform Development

- WezTerm core is cross-platform (Windows, macOS, Linux, BSDs)
- Custom utilities (fs-explorer, watch) are also cross-platform
- Window management abstracted via `window/` crate
- PTY operations abstracted via `pty/` crate

### Lua Configuration

- Configuration hot-reloading supported
- Extensive Lua API via 13+ `lua-api-crates/` modules
- Custom utilities can integrate via Lua callbacks
- See examples in custom utility README files

## Multi-Agent Coordination

Multiple AI agents may work on this repo concurrently. Use the **Agent Bus** (`http://localhost:8400`) for coordination.

**Preferred agent binary**: `~/bin/agent-bus-http.exe` for normal send/read/direct-channel work against the running HTTP service.
**Backend/admin binary**: `~/bin/agent-bus.exe` for MCP stdio, service startup, and backend debugging.

**Protocol**:
```bash
# Health check the service before a long coordination wave
curl.exe -s http://localhost:8400/health

# Announce presence
agent-bus-http.exe send --from-agent <agent-id> --to-agent all --topic status \
  --body "<message>" --tag "repo:wezterm"

# Claim file before editing
agent-bus-http.exe claim <file> --agent <id> --reason "<why>"

# Pairwise planning/review
agent-bus-http.exe read-direct --agent-a <id> --agent-b codex --limit 20 --encoding toon

# Compact recent context before resuming
agent-bus-http.exe compact-context --max-tokens 2000 --since-minutes 120

# Use backend binary for MCP stdio
agent-bus.exe serve --transport stdio
```

**Resource Protocol** (from [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md)):
- **Exclusive resources**: `target/`, `Cargo.lock`, `~/bin/*` installs, `.wezterm.lua`, `~/.config/wezterm/**`, Windows Terminal settings
- **Before using exclusive**: Post `RESOURCE_START resource=<path> mode=exclusive cmd=<what> eta=<time>`
- **After done**: Post `RESOURCE_DONE resource=<path> status=ok follow_up=<notes>`
- **Parallel builds**: Use per-agent `CARGO_TARGET_DIR` (e.g., `C:/Users/david/.cache/claude/<task>`)
- **sccache**: Shared (concurrent reads OK), but never `--zero-stats` or `--stop-server` without ack

**Do:**
- `agent-bus-http.exe read-direct --agent-a codex --agent-b claude --limit 20 --encoding toon` before shared edits
- `agent-bus-http.exe compact-context --max-tokens 2000 --since-minutes 120` before resuming long sessions
- `agent-bus-http.exe claim <file> --agent claude --reason "<why>"` before editing shared files
- `agent-bus-http.exe session-summary --session session:wezterm-wave --encoding compact` when closing a long tranche
- `agent-bus-http.exe post-direct --from-agent claude --to-agent codex --topic status --body "<summary>"` for high-signal handoffs

**Don't:**
- `agent-bus-http.exe read --since-minutes 1440` without narrowing (floods context)
- Use `compact-context` as fully reliable when PostgreSQL `jsonb` warning appears (treat as degraded)
- Use `agent-bus-http.exe` for MCP stdio (use `agent-bus.exe serve --transport stdio` instead)
- Treat `watch --encoding toon` as the canonical source of record in PowerShell; use it as a live probe only
- Edit files under `~/.config/wezterm/` without exclusive lock (triggers reload storm)
- Run `cargo build` on default `target/` without checking for active locks

**Gotchas**:
- Writing files inside `~/.config/wezterm/` triggers WezTerm's file watcher → config reload loop. Panel state uses `~/.local/state/wezterm-utils/` instead.
- `Cargo.lock` is exclusive — concurrent `cargo update` corrupts it.
- DLLs (conpty.dll, libEGL.dll, libGLESv2.dll) must be alongside wezterm.exe in `~/bin/` for GUI to launch.
- Heavy Rust builds/tests should default to private `CARGO_TARGET_DIR` values; reserve repo-default `target/` for explicitly coordinated waves.

**References**:
- [AGENTS.md](./AGENTS.md) — Agent-specific guidelines, coordination examples, positive/negative patterns
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) — Full shared resource protocol
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) — Cross-agent IPC protocol (bus CLI, channels, presence, TOON encoding)
- [JULES.md](./JULES.md) — Jules async agent for CI/CD reviews, test generation, security audits
- `~/.agents/rust-guidelines.txt` — Microsoft Pragmatic Rust Guidelines (canonical local copy)
- `~/.agents/rust-development-guide.md` — local Rust workflow and coordination guide
