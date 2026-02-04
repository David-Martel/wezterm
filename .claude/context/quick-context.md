# WezTerm Quick Context

## Current State
- **Branch**: main @ c4a77fe91
- **Origin**: github.com/david-t-martel/wezterm (your fork)
- **Upstream**: github.com/wezterm/wezterm (original)

## Last Session (2026-02-04)
- CLAUDE.md audited and corrected (sccache config values, utility workspace status)
- 45 CI workflows disabled → `.github/workflows.disabled/`
- build-all.ps1 enhanced with sccache/lld auto-detection
- Git remotes reconfigured (origin=fork, upstream=wezterm)

## Build Commands

**Windows (Just)**:
```powershell
just build              # Standard build with sccache
just release            # Release build
just clippy             # Linting (no sccache)
just test               # Tests with sccache
just full-local-ci      # Full validation
```

**Unix (Make)**:
```bash
make build              # Build main binaries
make test               # Run nextest
make fmt                # Format code
```

## Custom Utilities

| Utility | Workspace | Build |
|---------|-----------|-------|
| wezterm-watch | Yes | `cargo build -p wezterm-watch` |
| wezterm-fs-explorer | No (standalone) | `cd wezterm-fs-explorer && cargo build` |

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

## Untracked Files (consider .gitignore)
- TODO.md
- clippy-output.txt
- file-operations.log
- server.log

## Working Directory
`C:\Users\david\wezterm`
