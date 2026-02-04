# WezTerm Quick Context

## Current State
- **Branch**: main @ 00c43b6d6
- **Origin**: github.com/david-t-martel/wezterm (your fork)
- **Upstream**: github.com/wezterm/wezterm (original)
- **Tests**: 34 new tests (20 fs-utils + 14 module-framework) + 182 existing

## Latest Session (2026-02-04 Module Framework)

### Major Achievements
- Created `wezterm-fs-utils` crate (walker, search, watcher)
- Created `wezterm-module-framework` crate (Module trait, Capabilities, Registry)
- Implemented FsExplorerPane following TermWizTerminalPane pattern
- Implemented WatcherModule for background file watching
- Fixed Windows libgit2-sys linker errors (removed git2 from wezterm-version)
- Fixed termwiz Position type imports with ANSI escape sequences
- Full workspace build verified (831 crates)

### New Files
```
wezterm-fs-utils/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── walker.rs    # gitignore-aware walking (ignore crate)
    ├── search.rs    # Fuzzy search (nucleo-matcher)
    └── watcher.rs   # File watching (notify-debouncer-full)

wezterm-module-framework/
├── Cargo.toml
└── src/
    ├── lib.rs        # Module trait, Capabilities bitflags
    ├── context.rs    # ModuleContext for safe Mux/config access
    ├── registry.rs   # ModuleRegistry lifecycle management
    └── modules/
        ├── mod.rs
        ├── fs_explorer/
        │   ├── mod.rs     # FsExplorerModule
        │   └── pane.rs    # FsExplorerPane (TermWizTerminalPane pattern)
        └── watcher/
            └── mod.rs     # WatcherModule
```

## Build Commands

**Check new crates**:
```bash
cargo check -p wezterm-fs-utils -p wezterm-module-framework
cargo test -p wezterm-fs-utils -p wezterm-module-framework
```

**Full workspace**:
```bash
cargo check --workspace    # 831 crates
just quick-check           # fmt + clippy + check
```

**Windows (Just)** - 49 targets available:
```powershell
just quick-check        # Fast check + fmt + clippy
just build              # Standard build with sccache
just test-nextest       # Run all tests
just dev-cycle          # Full development cycle
```

## Key Fixes Applied

1. **libgit2-sys linker errors** - Removed git2 from wezterm-version/build.rs
2. **termwiz Position types** - Use ANSI escape sequences via `advance_bytes()`
3. **bitflags 1.x syntax** - No derive attributes (auto-generated)
4. **ConfigHandle type** - Return ConfigHandle instead of Arc<Config>

## Custom Utilities Status

| Utility | Tests | Status |
|---------|-------|--------|
| wezterm-fs-utils | 20 | ✅ Passing |
| wezterm-module-framework | 14 | ✅ Passing |
| wezterm-fs-explorer | 108 | ✅ Passing |
| wezterm-watch | 74 | ✅ Passing |

## Uncommitted Changes

```
wezterm-fs-utils/          # New crate
wezterm-module-framework/  # New crate
wezterm-version/build.rs   # Fixed git2 removal
wezterm-version/Cargo.toml # Removed git2 dependency
Cargo.toml                 # Added workspace members
.cargo/config.toml         # Added OpenSSL vcpkg config
```

## sccache Configuration

```toml
SCCACHE_DIR = "T:/RustCache/sccache"
SCCACHE_CACHE_SIZE = "30G"
SCCACHE_CACHE_COMPRESSION = "zstd"
```

## Git Workflow

```bash
git push                    # Push to your fork
git fetch upstream          # Get upstream changes
git merge upstream/main     # Merge upstream
```

## Recommended Next Steps
1. Commit all new crates to git
2. Add module initialization to wezterm-gui/src/main.rs
3. Register Lua API functions in config module
4. Research russh/thrussh as pure-Rust SSH alternatives (deferred)

## Working Directory
`C:\Users\david\wezterm`

---
*Full context: wezterm-context-2026-02-04-module-framework.md*
