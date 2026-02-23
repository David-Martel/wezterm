# Testing Context Slice

**For:** test-automator, test-runner agents
**Updated:** 2026-02-04

## Test Infrastructure

### Test Types
- **Unit tests:** `#[cfg(test)]` modules in source files
- **Integration tests:** `tests/integration.rs`
- **E2E tests:** `tests/e2e.rs` using `assert_cmd`

### Dependencies
```toml
[dev-dependencies]
tempfile = "3.8"
assert_cmd = "2.0"
predicates = "3.0"
serial_test = "3.0"  # For wezterm-watch
futures = "0.3"      # For async tests
```

## Test Commands
```bash
# Run all tests
cargo test --all

# Run specific package tests
cargo test -p wezterm-watch
cd wezterm-fs-explorer && cargo test

# Run with nextest (faster)
cargo nextest run

# Coverage
cargo llvm-cov --lib --summary-only
```

## Coverage Results

### wezterm-watch (85.54%)
| Module | Coverage |
|--------|----------|
| output.rs | 98.48% |
| git.rs | 81.94% |
| watcher.rs | 70.49% |

### wezterm-fs-explorer (59.20%)
Testable modules: 95%+
TUI modules: 0% (require terminal mocking)

## Known Issues
- Windows file events timing-dependent
- `cargo_bin` deprecated (use `cargo::cargo_bin_cmd!`)
- TUI code requires specialized testing framework

## Test Patterns
```rust
// E2E test pattern
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("wezterm-watch").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}
```
