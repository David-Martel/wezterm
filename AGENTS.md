# AGENTS.md - AI Agent Guidelines for WezTerm Development

This document provides guidelines for AI agents (Claude Code, GitHub Copilot, etc.) working on the WezTerm codebase.

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
just clippy             # Lint (no sccache due to probe issue)
just test               # Run tests
just full-local-ci      # Full validation

# Unix (Make)
make build              # Build main binaries
make test               # Run nextest
make fmt                # Format code
```

---

## Multi-Agent Coordination (Agent Bus)

Multiple AI agents may work on this repo concurrently. The **Agent Bus** at `http://localhost:8400` provides coordination.

### Required Protocol

1. **Set presence** on session start with your agent ID and capabilities
2. **Claim files** via `POST /channels/arbitrate/<path>` before editing — check for conflicts
3. **Check messages** every 2-3 tool calls via `GET /messages?agent=<id>&since=10&encoding=toon`
4. **Post completion summary** when done, then poll for follow-up tasks

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
# Announce presence
curl -s -X POST http://localhost:8400/messages -H "Content-Type: application/json" \
  -d '{"sender":"<agent-id>","recipient":"all","topic":"status","body":"ONLINE: working on <task>","tags":["repo:wezterm"]}'

# Claim a file before editing
curl -s -X POST http://localhost:8400/channels/arbitrate/wezterm-gui/src/main.rs \
  -H "Content-Type: application/json" -d '{"agent":"<id>","reason":"fixing render bug"}'

# Check for messages (TOON encoding saves 70% tokens)
curl -s "http://localhost:8400/messages?agent=<id>&since=10&encoding=toon"

# Direct message another agent
curl -s -X POST http://localhost:8400/channels/direct/<target-agent> \
  -H "Content-Type: application/json" -d '{"body":"Need API review","from":"<id>"}'
```

### Rules

- Use `request_ack: true` for blocking handoffs
- Use `thread_id` to group related messages
- Keep messages short: current state, exact ask, expected output, relevant path
- Never send secrets or credentials through the bus
- See `~/.codex/docs/AGENT_BUS.md` for full protocol reference
- See [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) for the repo-specific resource contention protocol

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

---

## References

- [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [WezTerm Documentation](https://wezfurlong.org/wezterm/)
- [CLAUDE.md](./CLAUDE.md) - Project-specific Claude Code guidance
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) - Shared resource protocol
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) - Cross-agent IPC protocol
