# Security Context Slice

**For:** security-auditor agents
**Updated:** 2026-02-04

## Security-Relevant Components

### IPC (wezterm-fs-explorer)
- **File:** `src/ipc.rs`, `src/ipc_client.rs`
- **Socket Type:** Unix domain sockets (uds_windows on Windows)
- **Risk:** Socket permissions, path injection
- **Status:** Needs review

### TLS (async_rustls)
- **File:** `async_rustls/src/verifier.rs`
- **Verification:** CN validation, certificate chain verification
- **Status:** Implemented with x509-parser, tested

### Git Operations
- **Library:** gix (pure Rust, no libgit2 CVEs)
- **Risk:** Repository path validation
- **Status:** Good - no native dependencies

## Security Patterns Used
- No `unsafe` in custom utilities
- Input validation for file paths
- Certificate verification with cryptographic validation
- No shell command injection (using typed APIs)

## Audit Priorities
1. IPC socket binding permissions
2. Path traversal in file operations
3. Certificate CN extraction edge cases

## Vulnerabilities Mitigated
- OpenSSL CVEs eliminated (using rustls)
- libgit2 CVEs eliminated (using gix)
- vcpkg supply chain removed
