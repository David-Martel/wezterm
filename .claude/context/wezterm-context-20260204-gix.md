# WezTerm Project Context - 2026-02-04 (Post-Refactoring)

## Context ID: ctx-wezterm-20260204-refactor

**Created**: 2026-02-04T13:00:00Z
**Updated**: 2026-02-04T13:02:00Z
**Branch**: main @ b1f4be536
**Project Type**: Rust (mixed with Lua scripting)

---

## Executive Summary

WezTerm has completed a major **pure-Rust migration** and **code quality improvement**:

1. **russh backend** - Pure-Rust SSH replacing libssh-rs/ssh2 (russh 0.57)
2. **gix migration** - Pure-Rust Git replacing git2/libgit2-sys (gix 0.78)
3. **Comprehensive refactoring** - 193 files cleaned via clippy auto-fixes
4. **Test suite expansion** - 52 wezterm-ssh tests, comprehensive coverage

The build no longer requires:
- vcpkg or OpenSSL for SSH (uses russh)
- libgit2-sys C compilation for Git (uses gix)

**OpenSSL is still required for** mux server TLS (wezterm-mux-server-impl).

---

## Recent Commits (This Session)

| Commit | Description |
|--------|-------------|
| `b1f4be536` | docs: expand documentation and add test coverage |
| `56284a0f2` | refactor: apply clippy auto-fixes across workspace |
| `45572cf72` | chore(docs): update project context with gix migration details |
| `5eeec2431` | refactor(plugin): replace git2/libgit2-sys with pure-Rust gix |

---

## Architecture Decisions

### Decision 1: gix over git2
- **Topic**: Git library choice
- **Decision**: Use gix (gitoxide) instead of git2
- **Rationale**: Eliminates libgit2-sys C compilation, aligns with pure-Rust goal
- **Status**: ✅ Complete

### Decision 2: russh over libssh-rs
- **Topic**: SSH library choice
- **Decision**: Use russh 0.57 as default SSH backend
- **Rationale**: Pure Rust, no OpenSSL dependency, native Windows support
- **Status**: ✅ Complete

### Decision 3: Fresh-clone update strategy
- **Topic**: Plugin update mechanism
- **Decision**: Update plugins by backup + fresh clone instead of fetch+merge
- **Rationale**: Simpler, more reliable, avoids complex merge logic
- **Status**: ✅ Complete

### Decision 4: Keep OpenSSL for mux TLS
- **Topic**: TLS library for multiplexer
- **Decision**: Keep OpenSSL-based async_ossl for mux server TLS
- **Rationale**: Migration to rustls requires larger effort, separate from SSH
- **Status**: ⏳ Deferred (future enhancement)

---

## Test Coverage

### Test Suite Summary

| Suite | Tests | Status |
|-------|-------|--------|
| wezterm-ssh lib (russh backend) | 52 | ✅ Pass |
| SSH config functional | 22 | ✅ Pass |
| wezterm-fs-utils | 33 | ✅ Pass |
| Terminal rendering | 33 | ✅ Pass |
| Docker graphics | 7 | ✅ Pass |
| Plugin | 1 | ✅ Pass |
| **Total** | **148+** | ✅ All Pass |

### New Tests Added (This Session)

- SFTP path handling and validation (3 tests)
- Authentication method handling (3 tests)
- Connection configuration validation (4 tests)
- SSH key algorithm support (2 tests)

---

## Code Quality Improvements

### Clippy Fixes Applied (193 files)

| Fix Type | Count |
|----------|-------|
| needless_borrow | Many |
| needless_return | Many |
| derivable_impls | 5+ |
| needless_question_mark | 10+ |
| ptr_eq | 2 |
| useless_transmute | 3 |
| missing_safety_doc | 2 |
| Total files changed | 193 |

### Documentation Improvements

- wezterm-ssh lib.rs: Added comprehensive crate-level docs
- plugin lib.rs: Enhanced gix migration documentation
- russh_backend/mod.rs: Architecture diagrams and usage guide
- .cargo/config.toml: Clarified OpenSSL requirements

---

## Dependency Changes

### Pure-Rust Stack (SSH/Git)

```
SSH:  russh 0.57 (ring crypto backend)
Git:  gix 0.78 (reqwest HTTP transport)
TLS:  rustls (via reqwest for Git HTTPS)
```

### Still Using OpenSSL

```
Mux TLS:  async_ossl (OpenSSL SslStream wrapper)
```

### Deprecated (Will Remove)

```
wezterm-ssh "legacy" feature:
  - libssh-rs
  - ssh2
  - vendored-openssl
```

---

## Build Configuration

### Feature Flags

```toml
# wezterm-ssh/Cargo.toml
[features]
default = ["russh"]  # Pure Rust default

# Legacy backends (DEPRECATED - require OpenSSL)
legacy = ["libssh-rs", "ssh2", "dep:async_ossl"]
```

### OpenSSL Configuration

```toml
# .cargo/config.toml
# OpenSSL is ONLY needed for:
# 1. Mux server TLS (wezterm-mux-server-impl)
# 2. Legacy SSH backends (optional "legacy" feature)
OPENSSL_DIR = "C:/codedev/vcpkg/installed/x64-windows"
```

---

## Roadmap

### Completed (This Session)
- [x] Migrate git2 → gix
- [x] Apply clippy auto-fixes (193 files)
- [x] Expand test coverage (52 SSH tests)
- [x] Add deprecation warnings to legacy SSH
- [x] Documentation expansion

### Next Steps
- [ ] Migrate mux server TLS to rustls (fully eliminate OpenSSL)
- [ ] Remove deprecated legacy SSH backends (after 2 releases)
- [ ] Performance benchmarking vs old implementation
- [ ] Consider gix submodule support for recursive plugin clones

### Tech Debt
- [ ] Clean up remaining clippy warnings in vendored deps
- [ ] Remove dead code warnings in russh_backend/sftp.rs

---

## Validation Commands

```bash
# Verify pure-Rust SSH
cargo build -p wezterm-ssh --no-default-features --features russh
cargo tree -p wezterm-ssh --features russh | grep openssl  # Should be empty

# Verify pure-Rust Git
cargo tree | grep -i "libgit2\|git2"  # Should be empty

# Run all tests
cargo test --workspace

# Check code quality
cargo clippy --workspace --all-targets
```

---

## Session Stats

- **Files Modified**: 198 (193 clippy + 5 docs/tests)
- **Lines Added**: ~1,600
- **Lines Removed**: ~1,660
- **Net Change**: -60 lines (cleaner code)
- **New Tests**: 13
- **Commits**: 4

---

## Validation

- **Last Validated**: 2026-02-04T13:02:00Z
- **All Tests**: ✅ 148+ passing
- **Build**: ✅ No C dependencies for SSH/Git
- **Clippy**: ✅ No errors, minimal warnings
- **Is Stale**: No
