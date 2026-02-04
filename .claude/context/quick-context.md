# WezTerm Quick Context

## Current State
- **Branch**: main @ dbe154194
- **Origin**: github.com/david-t-martel/wezterm (your fork)
- **Upstream**: github.com/wezterm/wezterm (original)
- **Tests**: 182 passing (108 fs-explorer + 74 watch)

## Latest Session (2026-02-04 Build Enhancements)

### Major Achievements
- Migrated git2 -> gix (pure Rust, no native deps)
- Added UDS Windows IPC (`src/ipc.rs`)
- Added WSL path translation (`src/path_utils.rs`)
- Added shell detection (`src/shell.rs`)
- Added fuzzy search with nucleo (`src/search.rs`)
- Enhanced Justfile to 32 targets
- Added cargo-binstall and cargo-smart-release
- Created Windows CI workflow

### New Files
```
wezterm-fs-explorer/src/
├── ipc.rs         # UDS Windows IPC
├── path_utils.rs  # WSL path translation
├── shell.rs       # Shell detection
└── search.rs      # Fuzzy search (nucleo)

Root:
├── release.toml   # cargo-smart-release config
├── cliff.toml     # git-cliff changelog
└── .github/workflows/windows-ci.yml
```

## Build Commands

**Windows (Just)** - 32 targets available:
```powershell
# Development
just quick-check        # Fast check + fmt + clippy
just build-utils        # Build custom utilities
just test-nextest       # Run all tests

# Tools
just install-dev-tools  # Install nextest, llvm-cov, git-cliff

# Coverage
just coverage-open      # Generate and open coverage report

# Release
just release-dry-run    # Preview release
just release-patch      # Bump patch version
just changelog          # Generate changelog
```

**Cargo Aliases** (new):
```bash
cargo b    # build
cargo c    # check
cargo t    # test
cargo nt   # nextest run
cargo br   # build --release
```

## Custom Utilities

| Utility | Tests | Status |
|---------|-------|--------|
| wezterm-fs-explorer | 108 | Passing |
| wezterm-watch | 74 | Passing |

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

## Recommended Next Agents
1. `test-automator` - Integration tests for new modules
2. `security-auditor` - Review IPC and path translation
3. `performance-engineer` - Profile fuzzy search

## Working Directory
`C:\Users\david\wezterm`

---
*Full context: wezterm-context-2026-02-04-build-enhancements.md*
