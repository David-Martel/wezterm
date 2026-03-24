# Security Audit Report - Custom Crates

This report summarizes the findings of a security audit performed on the following crates:
- `wezterm-utils-daemon`
- `wezterm-fs-explorer`
- `wezterm-module-framework`
- `wezterm-uds`

## Findings Summary

| ID | Severity | Component | Finding | File:Line |
|----|----------|-----------|---------|-----------|
| 1  | **HIGH** | `wezterm-fs-explorer` | Recursive symlink following in directory copy | `wezterm-fs-explorer/src/operations.rs:60` |
| 2  | **MEDIUM**| `wezterm-fs-explorer` | Path traversal via `..` in file operations | `wezterm-fs-explorer/src/app.rs:252, 273, 283` |
| 3  | **MEDIUM**| `wezterm-utils-daemon`| Arbitrary file deletion during IPC socket binding | `wezterm-utils-daemon/src/server.rs:95` |
| 4  | **MEDIUM**| `wezterm-module-framework` | Capability bypass for read-only Mux state | `wezterm-module-framework/src/context.rs:55-150` |
| 5  | **LOW**   | `wezterm-uds` | Missing `// SAFETY:` documentation in unsafe blocks | `wezterm-uds/src/lib.rs:47, 78, 98` |
| 6  | **LOW**   | `wezterm-utils-daemon`| Potential utility name spoofing in IPC routing | `wezterm-utils-daemon/src/router.rs:247` |

---

## Detailed Findings

### 1. Recursive Symlink Following in Directory Copy (HIGH)
**File**: `wezterm-fs-explorer/src/operations.rs:60`

The `copy_dir_all` function uses `entry_path.is_dir()` to decide whether to recurse. In Rust, `Path::is_dir()` follows symbolic links. If a directory being copied contains a symlink pointing to a directory outside the source tree (e.g., `/etc` or a large data volume), the explorer will recursively copy the contents of the linked directory to the destination. This can lead to unexpected disk space exhaustion, information leakage, or breaking out of intended directory boundaries.

**Recommendation**: Use `entry.file_type()?` from the `DirEntry` returned by `read_dir()` and check `is_dir()` on the file type itself, which does not follow symlinks, or use `symlink_metadata` to explicitly handle links.

### 2. Path Traversal via `..` in File Operations (MEDIUM)
**File**: `wezterm-fs-explorer/src/app.rs:252, 273, 283`

Operations like `rename`, `copy`, and `move` take input directly from the user (`input_buffer`) and join it to the current directory or parent path. There is no validation to ensure the input does not contain `..` or absolute paths. A user could provide a name like `../../id_rsa` to move or copy files outside of the current explorer view.

**Recommendation**: Validate that `input_buffer` is a single path component and does not contain directory separators or parent directory references.

### 3. Arbitrary File Deletion during IPC Socket Binding (MEDIUM)
**File**: `wezterm-utils-daemon/src/server.rs:95`

On Unix systems, the daemon attempts to remove the existing socket file at `self.path` before binding. If the `pipe_name` (used as the socket path on Unix) is provided via an untrusted configuration or CLI argument without validation, the daemon could be used to delete arbitrary files that the user running the daemon has permissions to remove.

**Recommendation**: Restrict the IPC socket path to a specific temporary directory or validate that it resides in an expected location (e.g., `XDG_RUNTIME_DIR`).

### 4. Capability Bypass for Read-only Mux State (MEDIUM)
**File**: `wezterm-module-framework/src/context.rs:55-150`

`ModuleContext` provides methods to query terminal state (e.g., `all_windows`, `get_pane`, `active_workspace`) that do not require any capabilities. A module with zero permissions (e.g., a "sandboxed" module) can still map out the entire user session, which is a privacy risk. Only modification operations currently require the `UI_CREATE_PANE` capability.

**Recommendation**: Introduce a `UI_VIEW_STATE` or similar capability and enforce it for all read-only Mux queries.

### 5. Missing `// SAFETY:` Documentation (LOW)
**File**: `wezterm-uds/src/lib.rs:47, 78, 98`

The `unsafe` blocks in `wezterm-uds` (implementing `FromRawFd`, `FromRawSocket`, and `IoSafe`) lack the `// SAFETY:` comments required by the project's Microsoft Pragmatic Rust Guidelines. While the implementation appears correct, the lack of justification is a policy violation and makes maintenance riskier.

**Recommendation**: Add explicit `// SAFETY:` comments explaining why these implementations are sound.

### 6. Potential Utility Name Spoofing (LOW)
**File**: `wezterm-utils-daemon/src/router.rs:247`

The router routes messages based on the prefix of the method name (e.g., `explorer/navigate` routes to the utility named `explorer`). Since any utility can register with any name on a first-come, first-served basis, a malicious utility could connect and register as `explorer` before the legitimate one, effectively intercepting its messages.

**Recommendation**: Implement a registry of "trusted" utility names or require authentication/capabilities for registering well-known names.

### 7. Secret Handling (Config Files)
**Finding**: No specific issues found.

**Description**: TLS credential handling in `config/src/tls.rs` and `wezterm/src/cli/tls_creds.rs` correctly uses path references rather than storing secrets. A search for common secret-related keywords (password, token, etc.) across the custom crates and configuration logic yielded no instances of hardcoded credentials or insecure logging of sensitive fields.
