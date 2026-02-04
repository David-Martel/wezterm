# WezTerm Project Context - 2026-02-04 (Post-gix Migration)

## Context ID: ctx-wezterm-20260204-gix

**Created**: 2026-02-04T12:30:00Z
**Branch**: main @ 5eeec2431
**Project Type**: Rust (mixed with Lua scripting)

---

## Executive Summary

WezTerm has completed a major **pure-Rust migration** eliminating all C dependencies for SSH and Git operations:

1. **russh backend** - Replaced libssh-rs/ssh2 with pure-Rust russh (0.57)
2. **gix migration** - Replaced git2/libgit2-sys with pure-Rust gix (0.78)
3. **Comprehensive test suite** - 130+ functional tests covering SSH config, graphics, Docker

The build no longer requires:
- vcpkg or OpenSSL for SSH
- libgit2-sys C compilation for Git
- Any external C library dependencies for core SSH/Git functionality

---

## Recent Changes (Last 10 Commits)

| Commit | Description |
|--------|-------------|
| `5eeec2431` | **refactor(plugin): replace git2/libgit2-sys with pure-Rust gix** |
| `2d23d9eff` | test(graphics): add Docker-based terminal graphics tests |
| `ee4eb4336` | test(functional): add comprehensive functional test suite |
| `30318e84e` | chore(context): update project context with all-phases-complete status |
| `db1547b66` | test: expand edge case coverage for walker and russh backend |
| `ff76c3ce6` | docs: add comprehensive documentation to russh backend and fs-utils |
| `566db118d` | perf(search): optimize fuzzy search with bounded heap and binary search |
| `63408c84b` | feat(ssh): implement russh-sftp integration for SFTP operations |
| `60cb5a892` | chore(ssh): add test dependencies and update tokio features |
| `7241a805f` | test(ssh): add comprehensive test framework for russh backend |

---

## Architecture Decisions

### Decision 1: gix over git2
- **Topic**: Git library choice
- **Decision**: Use gix (gitoxide) instead of git2
- **Rationale**: Eliminates libgit2-sys C compilation, aligns with pure-Rust goal
- **Alternatives**: Keep git2, use command-line git
- **Date**: 2026-02-04

### Decision 2: russh over libssh-rs
- **Topic**: SSH library choice
- **Decision**: Use russh 0.57 as default SSH backend
- **Rationale**: Pure Rust, no OpenSSL dependency, native Windows support
- **Alternatives**: libssh-rs, ssh2
- **Date**: 2026-02-04

### Decision 3: Fresh-clone update strategy
- **Topic**: Plugin update mechanism
- **Decision**: Update plugins by backup + fresh clone instead of fetch+merge
- **Rationale**: Simpler, more reliable, avoids complex merge logic
- **Date**: 2026-02-04

---

## Test Coverage

### Test Suite Summary

| Suite | Tests | Status |
|-------|-------|--------|
| wezterm-ssh lib (russh backend) | 39 | ✅ Pass |
| SSH config functional | 22 | ✅ Pass |
| wezterm-fs-utils | 33 | ✅ Pass |
| Terminal rendering | 33 | ✅ Pass |
| Docker graphics | 7 | ✅ Pass |
| Plugin | 1 | ✅ Pass |
| **Total** | **135** | ✅ All Pass |

### Test Files Created

- `wezterm-ssh/tests/ssh_config_functional.rs` - SSH config parsing tests
- `termwiz/tests/terminal_rendering_tests.rs` - ANSI/graphics protocol tests
- `termwiz/tests/graphics_docker_tests.rs` - Docker-based Sixel/iTerm2/Kitty tests
- `wezterm-ssh/tests/docker_ssh_test.rs` - Docker SSH integration tests
- `scripts/run-functional-tests.ps1` - PowerShell test runner

---

## Key Files Modified

### Cargo Configuration
- `Cargo.toml` - Added gix, removed git2 from workspace deps
- `lua-api-crates/plugin/Cargo.toml` - Switched to gix
- `wezterm-ssh/Cargo.toml` - Added docker-tests feature
- `termwiz/Cargo.toml` - Added docker-tests, base64, serde_json dev deps

### Plugin System (gix migration)
- `lua-api-crates/plugin/src/lib.rs` - Rewritten for gix API

### Test Infrastructure
- `scripts/run-functional-tests.ps1` - New test runner

---

## Dependency Changes

### Removed
- `git2` - C-based libgit2 bindings
- `libgit2-sys` - C library (was transitive)

### Added
- `gix` v0.78.0 with features:
  - `blocking-network-client`
  - `blocking-http-transport-reqwest`
  - `worktree-mutation`
  - `credentials`

### Verification
```bash
cargo tree | grep -i "libgit2\|git2"
# Returns nothing - completely removed
```

---

## Build Configuration

### Pure-Rust Stack (No C Dependencies)
```
SSH:  russh 0.57 (ring crypto backend)
Git:  gix 0.78 (reqwest HTTP transport)
TLS:  rustls (via reqwest)
```

### Feature Flags
```toml
# wezterm-ssh/Cargo.toml
[features]
default = ["russh"]
russh = ["dep:russh", "dep:russh-sftp", "dep:async-trait", "dep:tokio"]
docker-tests = []

# termwiz/Cargo.toml
[features]
docker-tests = []
```

---

## Agent Work Registry

| Agent | Task | Files | Status |
|-------|------|-------|--------|
| Manual | gix migration | plugin/*, Cargo.toml | ✅ Complete |
| Manual | Functional test suite | tests/*, scripts/* | ✅ Complete |
| Manual | Docker graphics tests | termwiz/tests/* | ✅ Complete |

---

## Roadmap

### Immediate (Done)
- [x] Migrate git2 → gix
- [x] Add functional test suite
- [x] Docker-based graphics tests
- [x] Verify all tests pass

### Next Steps
- [ ] Consider enabling gix submodule support for recursive plugin clones
- [ ] Add more integration tests with actual SSH servers
- [ ] Performance benchmarking vs old implementation

### Tech Debt
- [ ] Remove dead code warnings in russh_backend/sftp.rs
- [ ] Clean up unused imports in wezterm-ssh

---

## Validation

- **Last Validated**: 2026-02-04T12:30:00Z
- **All Tests**: ✅ 135 passing
- **Build**: ✅ No C dependencies for SSH/Git
- **Is Stale**: No
