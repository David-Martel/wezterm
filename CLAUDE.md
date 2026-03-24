# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WezTerm is a GPU-accelerated cross-platform terminal emulator and multiplexer written in Rust. It uses wgpu for rendering, supports terminal multiplexing (panes, tabs, windows), includes an SSH client with native tabs, and features Lua configuration with hot-reloading.

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

### Code Formatting and Linting

```bash
# Format all code (Windows: use Just)
just fmt               # PowerShell/Windows
make fmt               # Unix/Linux/macOS
cargo fmt --all        # Direct cargo (any platform)

# Check formatting without applying changes
cargo fmt --all --check

# Run clippy (Windows: Just handles sccache wrapper issues)
just clippy            # Windows (disables RUSTC_WRAPPER automatically)
cargo clippy --workspace --all-targets -- -D warnings -A clippy::type_complexity

# Note: clippy.toml allows clippy::type_complexity warnings
```

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

Run `just --list` for all 49+ targets. Key ones:

```bash
just quick-check          # Fast: check + fmt + clippy
just full-local-ci        # Full: fmt, clippy, nextest, docs, arch docs
just release-dry-run      # Preview release (cargo-smart-release + git-cliff)
just bootstrap-tools      # Install all dev tools (nextest, llvm-cov, git-cliff)
just coverage             # Coverage report via llvm-cov
```

### Release Automation

Configured via `release.toml` (cargo-smart-release) and `cliff.toml` (git-cliff conventional commits).

## Planning & Design Documents

All plans consolidated under `docs/`:
- **[TODO.md](./TODO.md)** — Current task tracking and agent ownership
- **[docs/plans/](./docs/plans/)** — Development plans (joint plan, UX redesign, customization roadmap, test plan)
- **[docs/specs/](./docs/specs/)** — Approved design specifications (UX redesign 4-phase spec)
- **[docs/design/](./docs/design/)** — Architecture documents (AI module design)
- **[JULES.md](./JULES.md)** — Jules (Google) async agent: CI/CD PR review, test generation, parallel exploration

**AI Assistant Module** ([docs/design/WEZTERM_AI_MODULE_DESIGN.md](./docs/design/WEZTERM_AI_MODULE_DESIGN.md)):
- Design spec for local LLM-based AI assistant integration
- `wezterm-module-framework/` crate provides the plugin/module infrastructure
- Module framework wired into GUI bootstrap (`wezterm-gui/src/main.rs`)

**Implementation Status**: Module framework integrated into GUI startup; daemon IPC client ready; AI/LLM integration pending

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

**Protocol**:
```bash
# Announce presence
curl -s -X POST http://localhost:8400/messages -H "Content-Type: application/json" \
  -d '{"sender":"<agent-id>","recipient":"all","topic":"status","body":"<message>","tags":["repo:wezterm"]}'

# Claim file before editing
curl -s -X POST http://localhost:8400/channels/arbitrate/<file> \
  -H "Content-Type: application/json" -d '{"agent":"<id>","reason":"<why>"}'

# Check for messages
curl -s "http://localhost:8400/messages?agent=<id>&since=10&encoding=toon"
```

**Rules**:
- Claim files via `/channels/arbitrate/` before editing shared files
- Check bus every 2-3 tool calls for coordination messages
- Use stable agent IDs: `claude`, `claude-docs`, `claude-ux`, `codex`, `gemini`
- Post completion summary when done; poll for follow-up tasks
- See [AGENTS.md](./AGENTS.md) for agent-specific coordination guidelines
- See [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) for shared resource contention protocol (build locks, install serialization, config exclusivity)
- See [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) for the full cross-agent IPC protocol (bus CLI, channels, presence, TOON encoding)
