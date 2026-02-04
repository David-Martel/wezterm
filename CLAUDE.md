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
.\build-all.ps1        # Builds and installs to $env:USERPROFILE\.local\bin
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

This is a Cargo workspace with 19+ member crates organized by functionality.

**Shared Target Directory** (`.cargo/config.toml`):
- Windows: `C:\Users\david\.cargo\shared-target\`
- Binaries: `shared-target\release\*.exe` or `shared-target\debug\*.exe`
- Benefits: Shared compilation artifacts across builds, reduced disk usage
- Note: Can be overridden by project-specific `./target/` if config not present

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

**Supporting Libraries**:
- `termwiz/` - Reusable terminal utilities library
- `promise/` - Async/promise utilities
- `wezterm-dynamic/` - Dynamic type system for configuration

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
SCCACHE_SERVER_PORT = "4226"
SCCACHE_CACHE_COMPRESSION = "zstd"
SCCACHE_CACHE_SIZE = "30G"
SCCACHE_DIR = "T:/RustCache/sccache"
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

### Custom WezTerm Utilities

This repository includes custom Rust utilities:

**wezterm-watch**: File watcher with git integration (workspace member)
- Location: `wezterm-watch/`
- Features: Real-time monitoring, git status, multiple output formats
- Build: `cargo build --release -p wezterm-watch`
- Docs: See `wezterm-watch/README.md`

**wezterm-fs-explorer**: High-performance filesystem explorer (standalone, not in workspace)
- Location: `wezterm-fs-explorer/`
- Features: Vim keybindings, git integration, Nerd Font icons, IPC support
- Build: `cd wezterm-fs-explorer && cargo build --release`
- Docs: See `wezterm-fs-explorer/README.md`
- Note: Has its own `Cargo.lock`; not part of the main workspace

**Build All Utilities** (Windows):
```powershell
.\build-all.ps1                # Builds and installs both utilities
.\build-all.ps1 -Force         # Force rebuild
.\build-all.ps1 -SkipTests     # Skip verification tests
```

Binaries install to: `$env:USERPROFILE\.local\bin\` (added to PATH automatically)

### New Utility Modules (wezterm-fs-explorer)

| Module | Description | Key Features |
|--------|-------------|--------------|
| `ipc.rs` | Cross-platform IPC | UDS Windows (uds_windows), tokio UnixStream on Unix |
| `path_utils.rs` | WSL path translation | C:\ ↔ /mnt/c/ conversion, path type detection |
| `shell.rs` | Shell detection | PowerShell, Git Bash, WSL, CMD auto-detection |
| `search.rs` | Fuzzy search | nucleo-based file search (Ctrl+F / `/`) |

## Integrated Build Tools Framework

### PowerShell Build Integration (`tools/`)

**Build-Integration.ps1** - Master build tools integration (1,280 lines):
```powershell
# Install all Rust build tools
.\tools\Build-Integration.ps1 -Install

# Test tool health
.\tools\Build-Integration.ps1 -HealthCheck

# Optimize build environment
.\tools\Build-Integration.ps1 -Optimize

# Smart release workflow
.\tools\Build-Integration.ps1 -Release -DryRun
```

Key functions:
- `Install-RustBuildTools` - cargo-binstall, nextest, llvm-cov, git-cliff, cargo-smart-release
- `Test-BuildToolHealth` - Verify all tools installed and functional
- `Optimize-BuildEnvironment` - Configure sccache, LTO, incremental builds
- `Invoke-SmartRelease` - Automated release with changelog generation

**Invoke-Gix.ps1** - gix CLI wrapper (pure Rust Git):
```powershell
# Repository statistics
.\tools\Invoke-Gix.ps1 -Stats

# Unreleased commits since last tag
.\tools\Invoke-Gix.ps1 -UnreleasedCommits

# Generate changelog
.\tools\Invoke-Gix.ps1 -Changelog

# Suggest version bump
.\tools\Invoke-Gix.ps1 -VersionBump
```

**CargoTools Module** (`tools/CargoTools/`):
- Cargo build wrapping with sccache integration
- Preflight checks for build environment
- Build routing and optimization
- Import: `Import-Module .\tools\CargoTools\CargoTools.psd1`

### Enhanced Justfile Targets (49 total)

**Quick Development**:
```bash
just quick-check       # Fast check + fmt + clippy
just dev-cycle         # Full development cycle
just pre-commit        # Pre-commit validation
```

**Build Targets**:
```bash
just build-parallel    # Parallel workspace build
just build-diag        # Build with diagnostics
just rebuild-clean     # Clean rebuild
just build-utils       # Build custom utilities
```

**Release & Changelog**:
```bash
just release-preview   # Preview release changes
just release-execute   # Execute release
just release-with-changelog  # Release + changelog
just changelog         # Generate changelog only
```

**Repository Analysis (gix)**:
```bash
just repo-stats        # Repository statistics
just unreleased-commits # Commits since last tag
just repo-verify       # Verify repository integrity
```

**Tool Management**:
```bash
just bootstrap-tools   # Install all dev tools
just check-tools       # Verify tool installation
just install-dev-tools # Install nextest, llvm-cov, git-cliff
```

**CI/CD**:
```bash
just ci-validate       # Full CI validation
just full-local-ci     # Comprehensive local CI
```

### Release Automation

**cargo-smart-release** (`release.toml`):
- Automated version bumping
- Changelog generation via git-cliff
- Pre-release checks and validation
- Configured for wezterm-fs-explorer and wezterm-watch

**git-cliff** (`cliff.toml`):
- Conventional commits parsing
- Grouped changelog by type (feat, fix, docs, etc.)
- GitHub release notes format

```powershell
# Preview release
just release-dry-run

# Execute patch release
just release-patch

# Generate changelog
just changelog
```

## Planned Features & Design Documents

**AI Assistant Module** (`WEZTERM_AI_MODULE_DESIGN.md`):
- Comprehensive design for integrating local LLM-based AI assistant into WezTerm
- Module framework architecture with capability-based permissions
- LLM integration layer (mistral.rs, gemma.cpp)
- Filesystem and Commander utilities with MCP protocol
- RAG system integration for context-aware assistance
- Performance optimizations (<700MB memory with AI active)
- See full specification: `WEZTERM_AI_MODULE_DESIGN.md`

**Implementation Status**: Design specification complete, implementation pending

## Important Development Notes

### Windows-Specific Considerations

1. **Justfile vs Makefile**:
   - Windows: Use `just` commands (PowerShell-based)
   - Unix/Linux/macOS: Use `make` commands (Bash-based)

2. **sccache Compatibility**:
   - Works with cargo build/test
   - **Does NOT work** with clippy (use `just clippy` which removes wrapper)
   - Check cache: `just sccache-stats`

3. **Shared Target Directory**:
   - Reduces build times and disk usage
   - Configured in `.cargo/config.toml`
   - All workspace members share compilation artifacts

4. **Static Linking**:
   - OpenSSL statically linked on Windows (`crt-static` feature)
   - Required for portable binaries

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