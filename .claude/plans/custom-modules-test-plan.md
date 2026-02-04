# Custom Modules Test Plan

**Created**: 2026-02-04
**Status**: In Progress
**Modules**: wezterm-fs-explorer, wezterm-watch

---

## Executive Summary

This plan outlines comprehensive testing for the custom WezTerm utilities. Both utilities currently have **zero test coverage** and need unit, integration, and functional tests.

---

## Test Strategy

### Testing Pyramid

```
                    ┌─────────────┐
                    │  E2E/Func   │  ← Manual + Automated UI tests
                   ┌┴─────────────┴┐
                   │  Integration   │  ← File system, Git, IPC
                  ┌┴───────────────┴┐
                  │   Unit Tests     │  ← Pure functions, error handling
                 └───────────────────┘
```

### Coverage Targets

| Module | Unit | Integration | E2E | Target Coverage |
|--------|------|-------------|-----|-----------------|
| wezterm-fs-explorer | 70% | 20% | 10% | 85% |
| wezterm-watch | 70% | 25% | 5% | 85% |

---

## wezterm-fs-explorer Tests

### Unit Tests

#### 1. file_entry.rs
```rust
#[cfg(test)]
mod tests {
    // FileEntry::from_path()
    - test_from_path_file
    - test_from_path_directory
    - test_from_path_symlink
    - test_from_path_nonexistent

    // FileEntry::read_directory()
    - test_read_directory_empty
    - test_read_directory_with_files
    - test_read_directory_hidden_files_shown
    - test_read_directory_hidden_files_hidden
    - test_read_directory_sorting
    - test_read_directory_permission_denied

    // FileEntry::extension()
    - test_extension_single
    - test_extension_double
    - test_extension_none
    - test_extension_dotfile
}
```

#### 2. git_status.rs
```rust
#[cfg(test)]
mod tests {
    // GitStatus::from_repo()
    - test_from_repo_valid
    - test_from_repo_not_git
    - test_from_repo_bare

    // GitStatus::get_status()
    - test_get_status_modified
    - test_get_status_added
    - test_get_status_deleted
    - test_get_status_untracked
    - test_get_status_staged
    - test_get_status_conflicted

    // FileStatus::to_short_str()
    - test_status_display_all_variants
}
```

#### 3. icons.rs
```rust
#[cfg(test)]
mod tests {
    // Icons::get_icon()
    - test_icon_directory
    - test_icon_rust_file
    - test_icon_python_file
    - test_icon_javascript_file
    - test_icon_markdown
    - test_icon_config_files
    - test_icon_unknown_extension
    - test_icon_hidden_file
    - test_icon_executable
}
```

#### 4. operations.rs
```rust
#[cfg(test)]
mod tests {
    // FileOperation::delete()
    - test_delete_file
    - test_delete_directory
    - test_delete_nonexistent
    - test_delete_permission_denied

    // FileOperation::rename()
    - test_rename_file
    - test_rename_directory
    - test_rename_cross_directory
    - test_rename_to_existing

    // FileOperation::copy()
    - test_copy_file
    - test_copy_directory_recursive
    - test_copy_to_existing
    - test_copy_preserve_permissions

    // FileOperation::create_file/directory()
    - test_create_file
    - test_create_directory
    - test_create_nested_directory
}
```

#### 5. keybindings.rs
```rust
#[cfg(test)]
mod tests {
    - test_help_text_not_empty
    - test_format_key_binding
    - test_all_keys_documented
}
```

#### 6. app.rs
```rust
#[cfg(test)]
mod tests {
    // Navigation
    - test_move_down_within_bounds
    - test_move_down_at_bottom
    - test_move_up_within_bounds
    - test_move_up_at_top
    - test_go_top
    - test_go_bottom

    // Selection
    - test_toggle_selection_single
    - test_toggle_selection_multiple
    - test_get_selected_paths_none
    - test_get_selected_paths_multiple

    // Search/Filter
    - test_visible_entries_no_filter
    - test_visible_entries_with_filter
    - test_visible_entries_case_insensitive

    // Mode transitions
    - test_start_search_mode
    - test_start_delete_mode
    - test_confirm_action_delete
    - test_cancel_action
}
```

### Integration Tests

```rust
// tests/integration/fs_explorer_integration.rs

#[test]
fn test_full_directory_navigation() {
    // Create temp dir structure
    // Navigate down, up, into subdirs
    // Verify state at each step
}

#[test]
fn test_file_operations_end_to_end() {
    // Create temp structure
    // Copy, move, delete files
    // Verify filesystem state
}

#[test]
fn test_git_integration() {
    // Create git repo in temp dir
    // Modify files
    // Verify status indicators
}
```

---

## wezterm-watch Tests

### Unit Tests

#### 1. watcher.rs
```rust
#[cfg(test)]
mod tests {
    // WatchEvent
    - test_watch_event_created_type
    - test_watch_event_modified_type
    - test_watch_event_deleted_type
    - test_watch_event_renamed_type
    - test_watch_event_error_type
    - test_watch_event_path_extraction

    // FileWatcher configuration
    - test_watcher_default_debounce
    - test_watcher_custom_debounce
    - test_watcher_gitignore_loading
    - test_watcher_custom_ignores
}
```

#### 2. git.rs
```rust
#[cfg(test)]
mod tests {
    // FileStatus
    - test_file_status_to_short_str_all_variants
    - test_file_status_to_colored_str

    // GitMonitor
    - test_is_git_repo_true
    - test_is_git_repo_false
    - test_repo_root_detection
    - test_find_repository_in_subdir
    - test_find_repository_none

    // Caching
    - test_cache_invalidation
    - test_cache_duration
    - test_force_refresh

    // Status retrieval
    - test_get_status_modified_files
    - test_get_status_untracked_files
    - test_get_status_staged_files
    - test_get_ahead_behind_counts
}
```

#### 3. output.rs
```rust
#[cfg(test)]
mod tests {
    // OutputFormat
    - test_output_format_from_str_json
    - test_output_format_from_str_pretty
    - test_output_format_from_str_events
    - test_output_format_from_str_summary
    - test_output_format_from_str_invalid

    // OutputFormatter
    - test_format_event_json
    - test_format_event_pretty
    - test_format_event_events_only
    - test_format_git_info_json
    - test_format_git_info_pretty
    - test_format_with_colors
    - test_format_without_colors
}
```

### Integration Tests

```rust
// tests/integration/watch_integration.rs

#[test]
fn test_file_change_detection() {
    // Create temp dir
    // Start watcher
    // Create/modify/delete files
    // Verify events received
}

#[test]
fn test_debouncing_behavior() {
    // Rapid file modifications
    // Verify debounced to single event
}

#[test]
fn test_gitignore_filtering() {
    // Create repo with .gitignore
    // Modify ignored files
    // Verify no events for ignored
}

#[test]
fn test_git_status_updates() {
    // Create git repo
    // Start watcher
    // Modify tracked files
    // Verify status in output
}
```

---

## E2E/Functional Tests

### wezterm-fs-explorer

```rust
// tests/e2e/fs_explorer_e2e.rs

#[test]
fn test_startup_and_shutdown() {
    // Start app with temp dir
    // Verify initial state
    // Send quit command
    // Verify clean exit
}

#[test]
fn test_keyboard_navigation() {
    // Simulate j/k/gg/G key presses
    // Verify cursor position
}

#[test]
fn test_file_preview() {
    // Create text file
    // Navigate to it
    // Toggle preview
    // Verify preview content
}
```

### wezterm-watch

```rust
// tests/e2e/watch_e2e.rs

#[test]
fn test_cli_output_formats() {
    // Run with --format=json
    // Run with --format=pretty
    // Verify output structure
}

#[test]
fn test_recursive_watch() {
    // Create nested directory structure
    // Modify files at various depths
    // Verify all changes detected
}
```

---

## Test Infrastructure

### Dependencies to Add

```toml
[dev-dependencies]
tempfile = "3.10"
assert_fs = "1.1"
predicates = "3.1"
tokio-test = "0.4"
serial_test = "3.0"  # For tests that can't run in parallel
mockall = "0.12"     # For mocking
```

### Test Utilities Module

```rust
// tests/common/mod.rs

pub fn create_temp_directory_structure() -> tempfile::TempDir { ... }
pub fn create_git_repo(path: &Path) -> git2::Repository { ... }
pub fn wait_for_fs_event(timeout: Duration) -> Option<WatchEvent> { ... }
pub fn simulate_keypress(key: KeyCode) { ... }
```

---

## Implementation Order

### Phase 1: Unit Tests (Priority)
1. [ ] wezterm-watch/src/output.rs tests
2. [ ] wezterm-watch/src/git.rs tests
3. [ ] wezterm-watch/src/watcher.rs tests
4. [ ] wezterm-fs-explorer/src/file_entry.rs tests
5. [ ] wezterm-fs-explorer/src/git_status.rs tests
6. [ ] wezterm-fs-explorer/src/icons.rs tests
7. [ ] wezterm-fs-explorer/src/operations.rs tests
8. [ ] wezterm-fs-explorer/src/keybindings.rs tests
9. [ ] wezterm-fs-explorer/src/app.rs tests

### Phase 2: Integration Tests
1. [ ] wezterm-watch file change detection
2. [ ] wezterm-watch git integration
3. [ ] wezterm-fs-explorer file operations
4. [ ] wezterm-fs-explorer git integration

### Phase 3: E2E Tests
1. [ ] wezterm-watch CLI behavior
2. [ ] wezterm-fs-explorer keyboard navigation

---

## Success Criteria

- [ ] All unit tests pass
- [ ] 85% code coverage on both utilities
- [ ] Integration tests pass with real filesystem
- [ ] Tests complete in < 30 seconds
- [ ] No flaky tests
- [ ] CI-compatible (no GUI requirements)
