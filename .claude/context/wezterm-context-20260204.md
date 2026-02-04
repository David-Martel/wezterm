# WezTerm Project Context

**Context ID**: ctx-wezterm-20260204
**Created**: 2026-02-04
**Branch**: main @ 566db118d

---

## Project Overview

WezTerm is a GPU-accelerated cross-platform terminal emulator written in Rust. This context tracks the recent work on implementing a pure-Rust SSH backend using russh to eliminate OpenSSL/vcpkg dependencies.

## Current State

### Summary
All planned tasks for the pure-Rust SSH backend (russh) and module framework improvements are complete. The russh backend provides SSH, PTY, and SFTP operations without any C library dependencies. Search algorithms in wezterm-fs-utils have been optimized, and comprehensive documentation and test coverage have been added.

### Recent Changes
- `wezterm-ssh/src/russh_backend/` - Complete russh SSH backend implementation
- `wezterm-ssh/src/russh_backend/sftp.rs` - Full SFTP integration
- `wezterm-fs-utils/src/search.rs` - Bounded heap optimization (O(n log k))
- `wezterm-fs-utils/src/walker.rs` - Edge case tests for Unicode, symlinks, deep nesting
- All russh backend modules - Comprehensive documentation with architecture diagrams

### Work Completed
1. **Phase 0**: Critical bug fixes (unsafe ptr::read, deadlock, thread leak, Windows paths)
2. **Phase 1**: Russh core connection, PTY operations, SFTP integration
3. **Phase 2**: Search algorithm optimizations
4. **Phase 3**: Documentation and test coverage expansion

### Test Results
- wezterm-fs-utils: 33 passing tests + 1 doctest
- wezterm-ssh russh: 28 unit tests + 18 integration tests

---

## Architecture Decisions

### Decision 1: Russh with Ring Crypto Backend
**Topic**: SSH library selection
**Decision**: Use russh 0.57 with ring crypto backend
**Rationale**:
- Pure Rust, no C dependencies
- ring works on Windows without pthread issues (unlike aws-lc-rs)
- Eliminates vcpkg/OpenSSL build complexity
**Alternatives Considered**: libssh-rs (C dependency), ssh2 (C dependency), russh with aws-lc-rs (pthread linking issues on Windows)

### Decision 2: Async/Sync Bridge Pattern
**Topic**: How to integrate async russh with sync wezterm-ssh
**Decision**: Shared tokio runtime with `block_on()` calls
**Rationale**:
- Single runtime instance via OnceLock
- 2 worker threads for async operations
- Minimal overhead, clean integration
**Implementation**: `wezterm-ssh/src/russh_backend/mod.rs:91-110`

### Decision 3: Bounded Heap for Search
**Topic**: Search result optimization
**Decision**: Use BinaryHeap<Reverse<T>> for top-K results
**Rationale**: O(n log k) vs O(n log n) for k results
**Implementation**: `wezterm-fs-utils/src/search.rs:164-196`

---

## Patterns Established

### Coding Conventions
- Module documentation with ASCII architecture diagrams
- Operation summary tables in doc comments
- Feature-gated compilation with `#[cfg(feature = "russh")]`
- Comprehensive error handling with anyhow

### Testing Strategy
- Unit tests colocated with source via `#[cfg(test)]` modules
- Integration tests in `tests/` directory
- Edge case tests for Unicode, symlinks, concurrency
- Ignored tests for platform-specific features

### Error Handling
- Use `SftpChannelError` for SFTP operations
- Convert russh errors to io::Error for compatibility
- Context-rich error messages via anyhow

---

## Agent Work Registry

| Agent | Task | Files | Status | Handoff |
|-------|------|-------|--------|---------|
| rust-pro | Russh backend | wezterm-ssh/src/russh_backend/*.rs | Complete | Tests passing |
| code-reviewer | Quality review | All russh files | Complete | 56 tests verified |
| docs-architect | Documentation | All module docs | Complete | Architecture diagrams added |

---

## File Inventory

### Core Russh Backend
- `wezterm-ssh/src/russh_backend/mod.rs` - Module entry, runtime management
- `wezterm-ssh/src/russh_backend/session.rs` - Connection and auth
- `wezterm-ssh/src/russh_backend/channel.rs` - PTY and command channels
- `wezterm-ssh/src/russh_backend/sftp.rs` - SFTP file operations
- `wezterm-ssh/src/russh_backend/handler.rs` - SSH event handler
- `wezterm-ssh/src/russh_backend/tests.rs` - Unit tests

### FS Utils
- `wezterm-fs-utils/src/search.rs` - Fuzzy search with nucleo-matcher
- `wezterm-fs-utils/src/walker.rs` - Gitignore-aware directory traversal
- `wezterm-fs-utils/src/watcher.rs` - File watching with notify

---

## Roadmap

### Completed
- [x] All Phase 0 critical bug fixes
- [x] Russh core connection (Task #13)
- [x] PTY channel operations (Task #14)
- [x] SFTP integration (Task #15)
- [x] Search optimization (Task #16)
- [x] Documentation (Task #17)
- [x] Test coverage (Task #18)

### Future Considerations
- [ ] SSH agent forwarding via Pageant (Windows)
- [ ] Ed25519 key support verification
- [ ] Performance benchmarking vs libssh-rs
- [ ] Remove legacy SSH backends (Phase 4)

---

## Validation

**Last Validated**: 2026-02-04
**Test Command**: `cargo test -p wezterm-ssh --features russh --no-default-features && cargo test -p wezterm-fs-utils`
**All Tests Passing**: Yes

---

## Next Agent Recommendations

Based on current state:
1. **code-reviewer**: Final review before merging changes
2. **deployment-engineer**: Update CI/CD for russh-only builds
3. **performance-engineer**: Benchmark russh vs libssh-rs
