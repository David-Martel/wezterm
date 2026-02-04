# WezTerm Module Framework Context

**Context ID**: ctx-wezterm-module-framework-20260204
**Created**: 2026-02-04
**Branch**: main @ 00c43b6d6
**Schema Version**: 2.0

---

## Project State Summary

Successfully implemented the WezTerm Module Framework, a comprehensive system for extending WezTerm with modular functionality. Created two new workspace crates (`wezterm-fs-utils` and `wezterm-module-framework`) that provide filesystem utilities and a module lifecycle management system following WezTerm's established patterns.

### Recent Changes

| File | Change Description |
|------|-------------------|
| `wezterm-fs-utils/` | New crate - directory walking, fuzzy search, file watching |
| `wezterm-module-framework/` | New crate - Module trait, Capabilities, Registry, Context |
| `Cargo.toml` | Added workspace members and dependencies |
| `.cargo/config.toml` | Added OpenSSL vcpkg configuration |
| `wezterm-version/build.rs` | Removed git2, fixed Windows linker errors |
| `wezterm-version/Cargo.toml` | Removed git2 build-dependency |

### Work in Progress
- All planned tasks completed
- Pending: Research russh/thrussh as pure-Rust SSH alternatives

### Blockers
- None - all builds passing

---

## Implementation Details

### wezterm-fs-utils Crate

**Location**: `wezterm-fs-utils/`

**Purpose**: Consolidated filesystem utilities for WezTerm modules

**Components**:
- `walker.rs` - Directory walking with `.gitignore` support using `ignore` crate
- `search.rs` - Fuzzy file search using `nucleo-matcher` (Helix editor's engine)
- `watcher.rs` - File watching with debouncing using `notify-debouncer-full`

**Key Dependencies**:
```toml
ignore = "0.4"
notify = "6.1"
notify-debouncer-full = "0.3"
nucleo-matcher = { workspace = true }
gix = { version = "0.68", default-features = false, features = ["status", "dirwalk"] }
crossbeam = { workspace = true }
```

**Tests**: 20 unit tests passing

### wezterm-module-framework Crate

**Location**: `wezterm-module-framework/`

**Purpose**: Module lifecycle management and permission system

**Components**:
- `lib.rs` - Module trait, Capabilities bitflags, ModuleState enum
- `registry.rs` - ModuleRegistry for registration and lifecycle management
- `context.rs` - ModuleContext for safe access to config and Mux
- `modules/fs_explorer/` - FsExplorerPane and FsExplorerModule
- `modules/watcher/` - WatcherModule for background file watching

**Key Pattern**: TermWizTerminalPane (from `mux/src/termwiztermtab.rs`)
- Uses `wezterm_term::Terminal` for rendering
- ANSI escape sequences via `advance_bytes()`
- crossbeam channels for input/output

**Tests**: 14 unit tests passing

---

## Architectural Decisions

### Decision 1: Use ANSI Escape Sequences for Rendering

**Topic**: How to render content in custom panes
**Decision**: Use raw ANSI escape sequences via `Terminal::advance_bytes()` instead of `Action` objects
**Rationale**:
- `wezterm_escape_parser::Action` differs from `termwiz::escape::Action`
- Direct ANSI sequences are simpler and more portable
- Follows how real terminal programs output content
**Alternatives Considered**:
- Using termwiz Change/Position API (more complex)
- Using perform_actions with wezterm_escape_parser::Action (limited variants)

### Decision 2: Remove git2 from wezterm-version

**Topic**: Fix Windows MSVC libgit2-sys linker errors
**Decision**: Replace git2::Repository usage with simple filesystem checks
**Rationale**:
- libgit2-sys has Windows MSVC linking issues (unresolved `close`, `_time64`, etc.)
- The actual version tag is obtained via `git` CLI anyway
- Filesystem checks for `.git/HEAD` provide same cache invalidation
**Alternatives Considered**:
- Fix libgit2-sys linking (complex, C library issue)
- Use gix instead (overkill for build script)

### Decision 3: Use bitflags 1.x Syntax

**Topic**: Capabilities bitflags declaration
**Decision**: Use bitflags 1.3 syntax without `#[derive(...)]` attributes
**Rationale**: Workspace uses `bitflags = "1.3"` which auto-derives Clone, Copy, Debug
**Alternatives Considered**: Upgrade to bitflags 2.x (would require workspace-wide changes)

### Decision 4: crossbeam Channels over tokio::mpsc

**Topic**: Thread-safe communication in modules
**Decision**: Use `crossbeam::channel` for event communication
**Rationale**:
- Already used by TermWizTerminalPane pattern
- Works in both async and sync contexts
- Better for cross-thread communication in GUI apps

---

## Code Patterns

### Module Trait Pattern

```rust
#[async_trait(?Send)]
pub trait Module: Send + Sync {
    fn module_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn required_capabilities(&self) -> Capabilities;
    fn state(&self) -> ModuleState;

    async fn init(&mut self, ctx: &ModuleContext) -> anyhow::Result<()>;
    async fn start(&mut self, ctx: &ModuleContext) -> anyhow::Result<()>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    async fn reload(&mut self, ctx: &ModuleContext) -> anyhow::Result<()> { Ok(()) }

    fn on_mux_notification(&mut self, _notification: &MuxNotification) {}
}
```

### Capabilities Pattern

```rust
bitflags! {
    pub struct Capabilities: u32 {
        const FILESYSTEM_READ  = 0b00000001;
        const FILESYSTEM_WRITE = 0b00000010;
        const PROCESS_SPAWN    = 0b00000100;
        const UI_CREATE_PANE   = 0b00010000;
        const CLIPBOARD        = 0b01000000;
        const NOTIFICATIONS    = 0b10000000;
    }
}
```

### ANSI Rendering Pattern

```rust
fn render_to_terminal(&self) {
    let mut output = String::new();

    // Clear screen and home
    let _ = write!(output, "\x1b[2J\x1b[H");

    // Bold blue text
    let _ = write!(output, "\x1b[1;34mTitle\x1b[0m\r\n");

    // Reverse video for selection
    let _ = write!(output, "\x1b[7mSelected\x1b[27m");

    // Send to terminal
    term.advance_bytes(output.as_bytes());
}
```

---

## Agent Work Registry

| Agent | Task | Files | Status | Handoff |
|-------|------|-------|--------|---------|
| rust-pro | Generated FsExplorerPane | pane.rs | Complete | Pattern fixed manually |
| (main) | Created wezterm-fs-utils | walker.rs, search.rs, watcher.rs | Complete | 20 tests passing |
| (main) | Created wezterm-module-framework | lib.rs, registry.rs, context.rs | Complete | 14 tests passing |
| (main) | Fixed Windows build issues | wezterm-version/build.rs | Complete | Full workspace builds |

### Recommended Next Agents

1. **test-automator**: Add integration tests for module lifecycle
2. **code-reviewer**: Review FsExplorerPane for edge cases
3. **docs-architect**: Generate API documentation

---

## Validation

### Build Status

```
✅ cargo check --workspace     - PASS (831 crates)
✅ cargo test -p wezterm-fs-utils -p wezterm-module-framework - PASS (34 tests)
✅ cargo build --release -p wezterm-fs-utils -p wezterm-module-framework - PASS
```

### Key Files Hash

```
wezterm-fs-utils/src/lib.rs
wezterm-module-framework/src/lib.rs
wezterm-version/build.rs
Cargo.toml
```

---

## Roadmap

### Immediate
- [x] Create wezterm-fs-utils crate
- [x] Create wezterm-module-framework crate
- [x] Implement FsExplorerPane
- [x] Implement WatcherModule
- [x] Fix Windows build issues
- [x] Verify full workspace builds

### This Week
- [ ] Commit all changes to git
- [ ] Add module initialization to wezterm-gui/src/main.rs
- [ ] Register Lua API functions in config module
- [ ] Create integration tests

### Tech Debt
- [ ] Fix unused `path` field warning in WatchSubscription
- [ ] Add error handling for FsExplorerState::load_directory failures
- [ ] Implement proper symlink detection in walker

### Future Research (Task #8)
- [ ] Research russh/thrussh as pure-Rust SSH alternatives to libssh
- [ ] Evaluate removing OpenSSL dependency via rustls

---

## Environment Notes

### Windows Build Configuration

**OpenSSL** (vcpkg):
```toml
OPENSSL_DIR = "C:/codedev/vcpkg/installed/x64-windows"
OPENSSL_NO_VENDOR = "1"
```

**Sccache**:
```toml
SCCACHE_DIR = "T:/RustCache/sccache"
SCCACHE_CACHE_SIZE = "30G"
```

**Shared Target**:
```
T:\RustCache\cargo-target\
```

---

## Files Created This Session

```
wezterm-fs-utils/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── walker.rs
    ├── search.rs
    └── watcher.rs

wezterm-module-framework/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── context.rs
    ├── registry.rs
    └── modules/
        ├── mod.rs
        ├── fs_explorer/
        │   ├── mod.rs
        │   ├── pane.rs
        │   └── README.md
        └── watcher/
            └── mod.rs
```

---

## Session Metrics

- **Duration**: ~2 hours
- **Crates Created**: 2
- **Tests Added**: 34
- **Build Fixes**: 4 (termwiz Position, bitflags, ConfigHandle, libgit2)
- **Background Tasks**: 7 completed
