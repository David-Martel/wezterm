# AGENTS.md - AI Agent Guidelines for WezTerm Development

This document provides guidelines for AI agents (Claude Code, GitHub Copilot, etc.) working on the WezTerm codebase.

## Repo Prompt Surface

Keep the following prompt/guidance files aligned when the workflow changes:

- `AGENTS.md` - cross-agent repo guidance
- `CLAUDE.md` - primary Claude-facing repo guide
- `.claude/CLAUDE.md` - repo-local Claude session overlay
- `JULES.md` - Jules async-agent operating guide

When shared workflow guidance changes, also cross-check:

- [TODO.md](./TODO.md)
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md)
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md)
- `~/.agents/rust-guidelines.txt`
- `~/.agents/rust-development-guide.md`

---

## Fork Policy (All Agents)

This is a **downstream fork** of `wezterm/wezterm`. No agent may create PRs, push commits, or contribute changes back to upstream. The `upstream` remote is fetch-only. All PRs and pushes target `David-Martel/wezterm` exclusively. Upstream is used only for pulling meaningful updates (e.g., `git fetch upstream && git merge upstream/main`).

---

## Quick Reference

| Task | Agent | Command |
|------|-------|---------|
| Rust development | `rust-pro` | Task tool |
| Code review | `code-reviewer`, `architect-reviewer` | Task tool |
| Debugging | `debugger`, `error-detective` | Task tool |
| Security audit | `security-auditor` | Task tool |
| Performance | `performance-engineer` | Task tool |
| Testing | `test-automator` | Task tool |

---

## Microsoft Rust Guidelines (Mandatory)

All Rust code in this repository must follow the [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/).

### Safety Guidelines (M-UNSAFE)

**Valid reasons for `unsafe`:**
1. Novel abstractions (new smart pointers, allocators)
2. Performance optimization (after benchmarking with evidence)
3. FFI and platform calls

**Never use `unsafe` to:**
- Shortcut otherwise safe implementations
- Circumvent compiler bounds (`Send`, `Sync`)
- Bypass lifetime requirements via `transmute`

**Requirements for `unsafe` code:**
```rust
// SAFETY: Explain why this is safe
// - Precondition 1
// - Precondition 2
// - Invariant maintained
unsafe { ... }
```

### Naming Guidelines (M-CONCISE-NAMES)

**Avoid weasel words:**
- Bad: `BookingService`, `ConnectionManager`, `ConfigFactory`
- Good: `Bookings`, `Connections`, `ConfigBuilder`

**Prefer regular functions:**
```rust
// Prefer this (regular function)
fn process_escape_sequence(seq: &[u8]) -> Action { ... }

// Over this (unnecessary associated function)
impl Parser {
    fn process_escape_sequence(seq: &[u8]) -> Action { ... }
}
```

### Error Handling (M-PANIC-IS-STOP)

**Panics are for:**
- Unrecoverable programming errors
- Contract violations (invalid invariants)
- Const context requirements
- User-requested operations (e.g., `unwrap()` in tests)

**Return `Result` for:**
- Expected failure modes (I/O errors, parse errors)
- Recoverable conditions
- User input validation

```rust
// Good: Panic on programming error
fn get_cell(&self, x: usize, y: usize) -> &Cell {
    assert!(x < self.width && y < self.height, "cell index out of bounds");
    &self.cells[y * self.width + x]
}

// Good: Return Result for recoverable error
fn parse_escape_sequence(input: &[u8]) -> Result<Action, ParseError> {
    // ...
}
```

### Performance Guidelines (M-THROUGHPUT, M-HOTPATH)

**Hot path identification:**
- Terminal rendering loop
- Escape sequence parsing
- Cell buffer operations
- Font glyph lookup

**Performance rules:**
1. Benchmark before optimizing with `unsafe`
2. Design APIs for batched operations
3. Exploit CPU cache locality
4. Avoid allocations in hot paths

**Async yield points (M-YIELD-POINTS):**
```rust
// For CPU-bound work, yield every 10-100μs
async fn process_large_buffer(&mut self) {
    for chunk in self.buffer.chunks(1024) {
        self.process_chunk(chunk);
        tokio::task::yield_now().await; // Cooperative yielding
    }
}
```

### Code Quality (M-STATIC-VERIFICATION)

**Required tools:**
- `cargo fmt --all` - Code formatting
- `cargo clippy --workspace --all-targets -- -D warnings` - Linting
- `cargo test` / `cargo nextest run` - Testing
- `cargo audit` - Security vulnerabilities

**Lint overrides:**
```rust
// Prefer #[expect] over #[allow] - warns if lint becomes unnecessary
#[expect(clippy::type_complexity)]
type ComplexType = ...;
```

**Public types must implement Debug:**
```rust
#[derive(Debug)]
pub struct TerminalState { ... }

// For sensitive data, use custom impl
impl Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}
```

### Documentation (M-DOCUMENTED-MAGIC)

**Document magic values:**
```rust
/// Maximum escape sequence length before we assume corruption.
/// This is based on the longest known escape sequence (OSC with base64 image)
/// plus a safety margin. Changing this may affect memory usage.
const MAX_ESCAPE_SEQ_LEN: usize = 1024 * 1024; // 1MB
```

**Structured logging:**
```rust
// Use OpenTelemetry conventions
log::info!(target: "terminal.render", "Frame rendered";
    "frame_time_ms" => frame_time.as_millis(),
    "cells_updated" => cells_updated
);
```

---

## WezTerm-Specific Guidelines

### Workspace Structure

```
wezterm/
├── term/                    # Core terminal emulator (platform-agnostic)
├── wezterm-gui/             # GUI application (wgpu rendering)
├── mux/                     # Multiplexer
├── config/                  # Configuration with hot-reload
├── wezterm-escape-parser/   # ANSI parser (no_std compatible)
├── lua-api-crates/          # Lua API modules
├── wezterm-fs-explorer/     # Custom utility (standalone)
└── wezterm-watch/           # Custom utility (workspace member)
```

### Key Patterns

**Separation of concerns:**
- Terminal logic in `term/` - no GUI dependencies
- Rendering in `wezterm-gui/` - no terminal logic
- Platform abstraction via traits

**GPU acceleration:**
- Uses wgpu for cross-platform rendering
- Batched glyph operations for performance

**Configuration:**
- Lua-based with hot-reloading
- Type-safe via `wezterm-dynamic`

### Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use k9::assert_equal; // Preferred assertion library

    #[test]
    fn test_escape_sequence_parsing() {
        let input = b"\x1b[31m";
        let result = parse_sgr(input);
        assert_equal!(result, Action::SetForeground(Color::Red));
    }
}
```

---

## Agent-Specific Instructions

### For rust-pro Agent

1. Always check existing patterns before introducing new ones
2. Prefer editing existing files over creating new ones
3. Run `just clippy` after making changes (Windows)
4. Follow the module structure in `lua-api-crates/` for new Lua APIs

### For code-reviewer Agent

1. Check for Microsoft Rust Guidelines compliance
2. Verify `unsafe` code has proper safety documentation
3. Look for performance regressions in hot paths
4. Ensure public types implement `Debug`

### For security-auditor Agent

1. Review all `unsafe` blocks
2. Check FFI boundaries for soundness
3. Verify path validation in filesystem operations
4. Look for potential injection vectors in Lua API

### For test-automator Agent

1. Use `k9` assertions for readability
2. Colocate tests with source using `#[cfg(test)]`
3. Use `cargo nextest run` for test execution
4. Add comments explaining test intent

---

## Build Commands

```bash
# Windows (Just)
just build              # Build with sccache
just clippy             # Strict custom-crate/fs-explorer lint (no sccache)
just clippy-workspace   # Explicit full-workspace lint; legacy warning debt still exists
just test               # Run tests
just full-local-ci      # Full validation

# Unix (Make)
make build              # Build main binaries
make test               # Run nextest
make fmt                # Format code
```

For heavy Rust builds/tests in parallel sessions:

- use a private `CARGO_TARGET_DIR` by default
- keep `sccache` shared
- do not use repo-default `target/` unless the resource thread says it is free

Recommended pattern:

```powershell
$env:RUSTC_WRAPPER='sccache'
$env:CARGO_TARGET_DIR='C:/Users/david/.cache/<agent>/<task>'
cargo nextest run -p wezterm-gui --no-fail-fast
```

For Rust quality references, prefer:

- `~/.agents/rust-guidelines.txt`
- `~/.agents/rust-development-guide.md`

---

## Multi-Agent Coordination (Agent Bus)

Multiple AI agents may work on this repo concurrently. The **Agent Bus** at `http://localhost:8400` provides coordination.

**Preferred agent binary**: `~/bin/agent-bus-http.exe` for normal send/read loops, direct channel coordination, `compact-context`, and `session-summary` against the running HTTP service.
**Backend/admin binary**: `~/bin/agent-bus.exe` for MCP stdio, server startup, and backend debugging.

### Required Protocol

1. **Set presence** on session start with your agent ID and capabilities
2. **Claim files** via `POST /channels/arbitrate/<path>` before editing — check for conflicts
3. **Check the direct channel** every 2-3 tool calls for shared work
4. **Post completion summary** when done, then poll for follow-up tasks

Practical default:

- `curl.exe -s http://localhost:8400/health`
- `agent-bus-http.exe read-direct --agent-a codex --agent-b claude --limit 20 --encoding toon`
- `agent-bus-http.exe compact-context --max-tokens 2500 --since-minutes 120`

### Agent IDs

| ID | Role |
|----|------|
| `claude` | Primary Claude Code session |
| `claude-docs` | Documentation/CLAUDE.md work |
| `claude-ux` | UX/UI layout and .wezterm.lua work |
| `codex` | OpenAI Codex sessions |
| `gemini` | Gemini CLI sessions |

### Coordination Examples

```bash
# Announce presence / planning via the service-facing binary
agent-bus-http.exe send --from-agent <agent-id> --to-agent all --topic status \
  --body "ONLINE: working on <task>" --tag "repo:wezterm"

# Claim a file before editing
agent-bus-http.exe claim wezterm-gui/src/main.rs --agent <id> --reason "fixing render bug"

# Read direct messages with TOON encoding before shared edits
agent-bus-http.exe read-direct --agent-a codex --agent-b claude --limit 20 --encoding toon

# Compact recent context before resuming a long wave
agent-bus-http.exe compact-context --max-tokens 2500 --since-minutes 120

# Server/MCP work still uses the backend binary
agent-bus.exe serve --transport stdio
```

### Positive Examples

- Good: `agent-bus-http.exe read-direct --agent-a codex --agent-b claude --limit 20 --encoding toon` before claiming a shared WezTerm file.
- Good: `agent-bus-http.exe session-summary --session session:wezterm-wave --encoding compact` at the end of a wave that consistently tagged `session:<id>`.
- Good: `agent-bus-http.exe send --from-agent codex --to-agent claude --topic planning --body "Taking Tier 5 validation" --tag "repo:wezterm"`.

### Negative Examples

- Avoid broad `agent-bus-http.exe read --agent codex --since-minutes 1440` calls during multi-repo work; narrow by direct channel, repo, session, or thread when possible.
- Avoid treating `compact-context` as fully healthy if it emits the known PostgreSQL `jsonb` fallback warning; coordination still works, but the read path is degraded and should be treated as advisory.
- Avoid using `agent-bus-http.exe` as the documented MCP stdio entrypoint; keep `agent-bus.exe serve --transport stdio` for that path.
- Avoid treating `watch --encoding toon` as the canonical record in PowerShell; use it as a live probe, then confirm state with `read-direct` or `session-summary`.

### Rules

- Use `request_ack: true` for blocking handoffs
- Use `thread_id` to group related messages
- Keep messages short: current state, exact ask, expected output, relevant path
- Never send secrets or credentials through the bus
- Prefer `read-direct` for pairwise planning/review and `compact-context` before long resumes
- See `~/.codex/docs/AGENT_BUS.md` for command selection and `agent-bus-http.exe` guidance
- See [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) for the repo-specific resource contention protocol
- See `C:/codedev/agent-bus/AGENT_COMMUNICATIONS.md` for the canonical protocol spec

### Quality Gates and Automated Tooling

Default enforcement now flows through repo-local PowerShell wrappers:

- `tools/hooks/Invoke-AstGrep.ps1`
- `tools/hooks/Invoke-WorkspaceRustChecks.ps1`

Current operating model:

- `just quick-check` runs before `just build` and `just release`
- `just lint-ast-grep` = broader custom-crate scan useful for backlog work
- `just lint-ast-grep-gate` = safe gate for changed files / CI-friendly enforcement
- `just ast-grep-fix-safe` = only syntax-preserving autofixes
- `just clippy` = strict custom-crate/fs-explorer lane
- `just clippy-workspace` = explicit full-workspace lane, still subject to legacy warning debt

Auto-fix policy:

- allow only local, syntax-preserving rewrites
- never auto-rewrite semantic control flow such as blanket `unwrap()` -> `?`
- stage safe rewrites first, then fix logic intentionally in code review

Hook notes:

- `pre-commit` and `lefthook` are both supported
- this machine may have `core.hooksPath=~/.git-hooks`; do not reset or override that globally without coordination
- if hook install behavior changes, update this file, `CLAUDE.md`, `.claude/CLAUDE.md`, and [TODO.md](./TODO.md)

### Resource Contention Protocol

The short version for this repo:

1. Use private `CARGO_TARGET_DIR` values by default for cargo work
2. Share `sccache`, but never stop/reset it without notice and ack
3. Treat repo-default `target/`, `Cargo.lock`, `~/bin/*` installs, runtime config, and WT settings as exclusive resources
4. Post direct-channel `RESOURCE_START` / `RESOURCE_DONE` notifications for exclusive work
5. If work can be namespaced safely, reroute it instead of blocking another agent

---

## Planning & Task Tracking

- [TODO.md](./TODO.md) — Current task tracking, tier priorities, agent ownership
- [docs/plans/](./docs/plans/) — Development plans (joint plan, UX redesign, customization, test coverage)
- [docs/specs/](./docs/specs/) — Approved design specs (UX redesign 4-phase)
- [docs/design/](./docs/design/) — Architecture documents (AI module)
- [JULES.md](./JULES.md) — Jules (Google) async agent integration for CI/CD, PR review, test generation

Prompt/guidance updates should reflect the current TODO state, especially:

- Tier 5.R/S for `agent-bus-http.exe` adoption and the current PostgreSQL fallback warning
- Tier 5.T/U/V/W/X/Y for hook installation, ast-grep rollout, warnings-as-errors scope, inline-test suppression research, and remaining unwrap/panic backlog
- Tier 6.T/U/V/W for current Jules review and automation work

---

## References

- [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [WezTerm Documentation](https://wezfurlong.org/wezterm/)
- [CLAUDE.md](./CLAUDE.md) - Project-specific Claude Code guidance
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) - Shared resource protocol
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) - Cross-agent IPC protocol
