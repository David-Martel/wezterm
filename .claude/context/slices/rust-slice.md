# Rust Context Slice

**For:** rust-pro, code-reviewer agents
**Updated:** 2026-02-04

## Project Type
Rust workspace with 19+ crates. Custom utilities are standalone or workspace members.

## Key Crates
- `wezterm-watch/` - File watcher (workspace member)
- `wezterm-fs-explorer/` - FS explorer TUI (standalone)
- `async_rustls/` - Pure-Rust TLS (workspace member)

## Build Commands
```bash
# Build with sccache
just build

# Clippy (disables sccache)
just clippy

# Test with nextest
cargo nextest run

# Coverage
cargo llvm-cov --lib --summary-only
```

## Coding Standards
- Rust 2021 edition
- `thiserror` for library errors
- `anyhow` for application errors
- Tests colocated with `#[cfg(test)]`

## Current Tech Debt
- 32 `cargo_bin` deprecation warnings in E2E tests
- Dead code warnings in ipc.rs

## Dependencies
- gix (not git2) for git operations
- rustls (not openssl) for TLS
- nucleo for fuzzy search
- ratatui for TUI
