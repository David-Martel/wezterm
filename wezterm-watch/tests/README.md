# Integration Tests for wezterm-watch

This directory contains comprehensive integration tests for the wezterm-watch utility.

## Overview

The integration tests verify the interaction between:
- `FileWatcher` - File system watching with debouncing
- `GitMonitor` - Git status tracking
- Real filesystem operations
- Git repository operations

## Test Coverage

### File Change Detection (3 tests)
- `test_file_change_detection` - Create, modify, delete file operations
- `test_directory_operations` - Nested directory and file operations
- `test_multiple_file_changes` - Concurrent file modifications

### Debouncing Behavior (2 tests)
- `test_debouncing_rapid_changes` - Rapid modifications should be debounced
- `test_debouncing_with_delays` - Delayed changes should trigger separate events

### Gitignore Filtering (3 tests)
- `test_gitignore_filtering` - .gitignore patterns should be respected
- `test_custom_ignore_patterns` - Custom ignore patterns via CLI
- `test_default_ignore_patterns` - Default patterns (.git, target, etc.)

### Git Status Integration (5 tests)
- `test_git_status_integration` - Modified file status tracking
- `test_git_status_untracked_files` - Untracked file detection
- `test_git_watcher_integration` - Combined watcher + git monitor
- `test_git_branch_detection` - Current branch detection
- `test_git_cache_behavior` - Cache invalidation and performance

### Edge Cases (3 tests)
- `test_watcher_on_nonexistent_path` - Handle invalid paths
- `test_git_monitor_non_git_directory` - Non-git directory handling
- `test_watcher_with_permission_errors` - Permission errors (Unix only)

### Stress Tests (2 tests)
- `test_concurrent_file_operations` - Concurrent file modifications
- `test_watcher_stability_under_load` - High-volume file operations

## Running the Tests

### Run all integration tests
```bash
cargo test --test integration
```

### Run specific test
```bash
cargo test --test integration test_file_change_detection
```

### Run with single thread and output
```bash
cargo test --test integration -- --test-threads=1 --nocapture
```

### Run only debouncing tests
```bash
cargo test --test integration debouncing
```

## Test Architecture

### Helper Functions

#### `init_git_repo(path: &Path)` - Initialize git repository
Creates a git repository with:
- Initial empty commit
- Configured user email and name

#### `collect_events(watcher, timeout, max_events)` - Collect events with timeout
Collects file system events up to a maximum count or timeout.

#### `wait_for_event_type(watcher, timeout, event_type)` - Wait for specific event
Blocks until a specific event type is received or timeout.

### Test Structure

Each test follows the pattern:
1. **Setup** - Create temp directory, initialize watcher/git
2. **Execute** - Perform file system operations
3. **Verify** - Assert expected events were received
4. **Cleanup** - Automatic via `TempDir::drop()`

## Dependencies

The integration tests require:
- `tempfile` - Temporary directory creation
- `serial_test` - Test serialization (if needed)
- `git` - Git command-line tool (for git tests)

## Platform Compatibility

All tests are CI-compatible:
- No GUI requirements
- No user interaction
- Deterministic behavior
- Appropriate timeouts for CI environments

### Platform-Specific Tests

Some tests are conditionally compiled:
- `test_watcher_with_permission_errors` - Unix only (uses chmod)

## Performance Considerations

Tests use short debounce intervals (50-300ms) for faster execution:
- Production default: 100ms
- Test default: 50ms (faster feedback)
- Debounce tests: 150-300ms (to verify debouncing works)

## Troubleshooting

### Tests timing out
Increase timeout durations in `collect_events()` or `wait_for_event_type()` calls.

### File lock errors
Ensure no other cargo processes are running. Use `--test-threads=1` to serialize tests.

### Git command failures
Ensure git is installed and available in PATH.

### Flaky tests
File system operations may have timing variations. Tests use generous timeouts to minimize flakiness.

## Future Enhancements

Potential additions:
- Rename event detection (currently reserved)
- Symbolic link handling
- Network filesystem watching
- Large directory stress tests (1000+ files)
- Cross-platform IPC integration tests
