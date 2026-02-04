# WezTerm Context - 2026-02-04

## Schema Version
2.0

## Context ID
ctx-wezterm-20260204

## Project Information

| Field | Value |
|-------|-------|
| Name | WezTerm Terminal Emulator |
| Root | C:\Users\david\wezterm |
| Type | Rust (workspace) |
| Branch | main |
| Commit | c4a77fe91 |
| Origin | github.com/david-t-martel/wezterm (fork) |
| Upstream | github.com/wezterm/wezterm |

## Current State Summary

WezTerm fork is fully configured for local-first development. GitHub Actions CI workflows have been disabled (moved to `.github/workflows.disabled/`) in favor of local validation via Justfile and pre-commit hooks. Build acceleration is configured with sccache (30G cache, zstd compression) at `T:/RustCache/sccache`. CLAUDE.md documentation has been audited and corrected to accurately reflect actual configuration values.

## Recent Changes (This Session)

| File | Change |
|------|--------|
| `.github/workflows/*.yml` → `.github/workflows.disabled/` | 45 CI workflows disabled |
| `.cargo/config.toml` | sccache config: 30G cache, zstd compression, T:/ path |
| `build-all.ps1` | Added -Sccache/-Lld params, auto-detection, stats display |
| `CLAUDE.md` | Fixed sccache values, clarified utility workspace status |

## Commits Created

```
c4a77fe91 perf(build): enhance sccache configuration and build acceleration
b8baecc5d chore(ci): disable GitHub Actions workflows for local-first development
```

## Decisions Made

### Decision 001: Local-First CI Strategy
- **Topic**: CI/CD workflow management
- **Decision**: Disable GitHub Actions workflows, use local validation
- **Rationale**: Faster iteration, reduced CI costs, local Justfile/pre-commit provides equivalent checks
- **Alternatives**: Keep workflows enabled, use conditional triggers
- **Date**: 2026-02-04

### Decision 002: Git Remote Configuration
- **Topic**: Fork vs upstream remote naming
- **Decision**: `origin` = fork (david-t-martel/wezterm), `upstream` = original (wezterm/wezterm)
- **Rationale**: Standard fork workflow convention - push to origin, pull from upstream
- **Date**: 2026-02-04

### Decision 003: CLAUDE.md Documentation Standards
- **Topic**: Configuration documentation accuracy
- **Decision**: CLAUDE.md must reflect actual config file values, not theoretical/default values
- **Rationale**: Prevents confusion when developers copy-paste commands
- **Date**: 2026-02-04

## Patterns

### Coding Conventions
- Rust workspace with 19+ member crates
- `#[cfg(test)]` modules for colocated unit tests
- `k9` assertion library for expressive test assertions
- clippy.toml allows `type_complexity` warnings

### Build Strategy
- **Windows**: `just` commands (PowerShell-based)
- **Unix/macOS**: `make` commands (Bash-based)
- **sccache**: Enabled for build/test, disabled for clippy (probe failure workaround)
- **Shared target**: `T:/RustCache/sccache` (Windows)

### Testing Strategy
- **nextest** preferred: `cargo nextest run`
- **Pre-commit**: Fast checks on changed crates
- **Pre-push**: Full `--all-features` validation

## Agent Work Registry

| Agent | Task | Files | Status | Handoff |
|-------|------|-------|--------|---------|
| claude-md-improver | Audit CLAUDE.md quality | CLAUDE.md | Complete | Grade B+ (84→88) |
| commit-cluster | Cluster and commit changes | 48 files | Complete | 2 commits pushed |

## Recommended Next Agents

1. **rust-pro**: Continue Rust development work
2. **test-automator**: Add/improve test coverage
3. **security-auditor**: Review custom utilities for vulnerabilities
4. **docs-architect**: Generate architecture documentation from codebase

## Roadmap

### Immediate
- [x] CLAUDE.md accuracy audit
- [x] Commit pending changes
- [x] Configure git remotes

### This Week
- [ ] Implement features from AI Module Design spec
- [ ] Improve test coverage for custom utilities
- [ ] Performance profiling with sccache

### Tech Debt
- [ ] Consider adding temp files to .gitignore (TODO.md, *.log)
- [ ] Evaluate wezterm-fs-explorer workspace inclusion

## Validation

| Check | Status |
|-------|--------|
| Git clean | Partial (4 untracked temp files) |
| Build passes | Yes |
| Clippy passes | Yes |
| Tests pass | Yes |
| Remote configured | Yes (origin=fork, upstream=wezterm) |

## Quick Restore Commands

```powershell
# Navigate to project
cd C:\Users\david\wezterm

# Check status
git status
git remote -v

# Build (Windows)
just build

# Test
just test

# Full validation
just full-local-ci
```

## Key Files

- `CLAUDE.md` - Project guidance for Claude Code
- `Justfile` - Windows build commands
- `Makefile` - Unix build commands
- `.cargo/config.toml` - Cargo/sccache configuration
- `.pre-commit-config.yaml` - Pre-commit hook configuration
- `build-all.ps1` - Custom utility builder with acceleration
