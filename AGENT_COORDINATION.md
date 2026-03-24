# Agent Coordination (Shared)

> **Canonical cross-agent IPC protocol for coordinating between Claude, Codex, Gemini, Copilot, and sub-agents.**
> **Protocol Version: v0.4 | Implementation: Rust native (no Python)**
> This is the canonical copy. Mirrors at `~/.claude/`, `~/.codex/`, `~/.gemini/` should sync to this file.

## Quick Start (Rust CLI — v0.4)

```bash
# Rust CLI (primary — ~/bin/agent-bus.exe, 100% Rust native, no Python)
agent-bus health --encoding json              # Check Redis + PostgreSQL
agent-bus send --from-agent claude --to-agent codex --topic "status" --body "ready"
agent-bus read --agent claude --since-minutes 60 --encoding human
agent-bus read --agent claude --since-minutes 60 --encoding toon   # Ultra-compact for LLMs
agent-bus watch --agent claude --history 10 --encoding human       # Live streaming
agent-bus watch --agent claude --history 10                        # Compact streaming
agent-bus ack --agent claude --message-id <id>
agent-bus presence --agent claude --status online --capability mcp
agent-bus monitor --session "session:code-review" --refresh 5      # Live dashboard
agent-bus serve --transport stdio             # MCP server mode (for mcp.json)
agent-bus serve --transport http --port 8400  # HTTP REST mode
agent-bus serve --transport mcp-http --port 8401  # MCP Streamable HTTP (2025-06-18 spec)

# File ownership
agent-bus claim src/redis_bus.rs --agent claude --reason "Adding compression"
agent-bus claims --resource src/redis_bus.rs
agent-bus resolve src/redis_bus.rs --winner claude

# Channels
agent-bus post-direct --from-agent claude --to-agent codex --body "Review done"
agent-bus read-direct --agent-a claude --agent-b codex
agent-bus post-group --group "review-http-rs" --from-agent reviewer --body "3 issues"
agent-bus read-group --group "review-http-rs"

# Codex integration
agent-bus codex-sync --limit 100

# Full help with env vars, encoding modes, examples, and docs links:
agent-bus --help
agent-bus send --help

# PowerShell wrappers (audio notifications, table formatting)
pwsh -NoLogo -NoProfile -File ~/.codex/tools/agent-bus-mcp/scripts/watch-agent-bus.ps1 -Agent claude -Notify
```

## Architecture (v0.4)

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Coordination Bus** | Redis Stream | Realtime message history with TTL-based trimming (~100K max entries) |
| **Pub/Sub Notifications** | Redis Pub/Sub | Event streaming for live monitoring (watch, monitor commands) |
| **Presence Tracking** | Redis Keys + TTL | Agent availability with auto-expiry (default: 180s) |
| **Durable History** | PostgreSQL | Audit trail + retention policy, tag-indexed queries, GIN indexing |
| **Channel System** | Redis Streams (per-channel) | Direct messages, group discussions, ownership arbitration |
| **Transport Modes** | Stdio MCP, HTTP REST, MCP Streamable HTTP | Multiple integration patterns for different agent types |
| **Encoding** | JSON, Compact, Minimal, TOON, Human | Multi-format output for LLM efficiency (TOON saves ~70% tokens) |

## Agent IDs

| ID | Agent |
|----|-------|
| `claude` | Claude Code |
| `codex` | OpenAI Codex CLI |
| `gemini` | Gemini CLI |
| `copilot` | GitHub Copilot |
| `euler` | Custom sub-agent |
| `pasteur` | Custom sub-agent |
| `all` | Broadcast (announcements only) |

Reuse the same ID throughout a session so watch filters and acknowledgements stay coherent.

## CLI Reference (v0.4)

**Binary:** `agent-bus` (Rust native at `~/bin/agent-bus.exe`, ~8 MB, instant startup)

### Core Commands
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `health` | Check Redis + PostgreSQL + runtime stats | `--encoding compact\|json\|human\|toon` |
| `send` | Post a message to bus | `--from-agent`, `--to-agent`, `--topic`, `--body`, `--schema finding\|status\|benchmark`, `--priority`, `--request-ack` |
| `read` | Query message history (chronological) | `--agent`, `--from-agent`, `--since-minutes`, `--limit`, `--encoding` |
| `watch` | Stream live events (Ctrl+C to stop) | `--agent`, `--history N`, `--encoding` |
| `ack` | Acknowledge a message | `--agent`, `--message-id`, `--body` |
| `pending-acks` | List unacknowledged messages | `--agent`, `--encoding` |

### Presence & Monitoring
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `presence` | Set agent availability | `--agent`, `--status online\|offline\|busy`, `--capability`, `--ttl-seconds` |
| `presence-list` | List active agents | `--encoding` |
| `presence-history` | PG audit trail of presence changes | `--agent`, `--since-minutes`, `--limit` |
| `monitor` | Live dashboard of agent activity | `--session`, `--refresh` (seconds) |

### File Ownership & Arbitration
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `claim` | Claim first-edit ownership | `resource`, `--agent`, `--reason` |
| `claims` | List claims (pending/contested/granted) | `--resource`, `--status` |
| `resolve` | Resolve conflicting claim | `resource`, `--winner`, `--reason`, `--resolved-by` |

### Channels (Direct, Groups, Escalation)
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `post-direct` | Send private 1-on-1 message | `--from-agent`, `--to-agent`, `--body`, `--topic` |
| `read-direct` | Read private messages | `--agent-a`, `--agent-b`, `--limit` |
| `post-group` | Post to named group | `--group`, `--from-agent`, `--body`, `--topic` |
| `read-group` | Read group messages | `--group`, `--limit` |

### Batch & Storage
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `batch-send` | Send NDJSON file of messages | `--file`, `--encoding` |
| `journal` | Export messages to per-repo NDJSON | `--tag`, `--from-agent`, `--output` |
| `export` | Export to stdout as NDJSON | `--agent`, `--since-minutes`, `--limit` |
| `sync` | Backfill Redis → PostgreSQL (one-time) | `--limit` |
| `prune` | Delete old PG records | `--older-than-days` |

### Integration & Servers
| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `codex-sync` | Sync bus findings to Codex | `--limit`, `--encoding` |
| `serve` | Start MCP/HTTP server | `--transport stdio\|http\|mcp-http`, `--port` |

## PowerShell Wrappers

Located at `~/.codex/tools/agent-bus-mcp/scripts/`:

| Script | Purpose |
|--------|---------|
| `send-agent-bus.ps1` | Wrapper for `send` with dual-transport fallback |
| `read-agent-bus.ps1` | Human-readable table formatting |
| `watch-agent-bus.ps1` | Audio notification (`-Notify`), formatted output |

## MCP Registration

Register the Rust binary in any agent's MCP config:

```json
{
  "agent-bus": {
    "command": "C:\\Users\\david\\bin\\agent-bus.exe",
    "args": ["serve", "--transport", "stdio"],
    "env": {
      "AGENT_BUS_REDIS_URL": "redis://localhost:6380/0",
      "AGENT_BUS_SERVICE_AGENT_ID": "agent-bus",
      "RUST_LOG": "error"
    }
  }
}
```

## Message Contract (Protocol v1.0 — Implemented in v0.4)

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | auto | Generated on creation |
| `timestamp_utc` | ISO 8601 | auto | UTC, generated on creation |
| `protocol_version` | string | auto | Always `"1.0"` |
| `from` | string | yes | Sender agent ID |
| `to` | string | yes | Recipient ID or `"all"` |
| `topic` | string | yes | Message type (e.g. `review-findings`, `file-ownership`) |
| `body` | string | yes | Message content |
| `thread_id` | string | no | Group related handoffs |
| `tags` | string[] | no | Filtering labels |
| `priority` | enum | no | `low`, `normal` (default), `high`, `urgent` |
| `request_ack` | bool | no | Request acknowledgement |
| `reply_to` | string | no | Defaults to sender |
| `metadata` | object | no | Arbitrary key-value pairs |

## Conventions

- Use `to=all` only for broad announcements.
- Use `request_ack` when you need the other agent to confirm receipt.
- Use `thread_id` to group related handoffs into a conversation.
- Keep `topic` short and stable so message threads are easy to scan.
- Put the actual ask in `body`: current state, exact task, expected output, paths/run IDs.
- Prefer one owner per file cluster or deploy step to avoid accidental overlap.
- Advertise presence when starting a longer session and refresh it during sustained work.
- Close the loop with a final message when the task is done or abandoned.
- Never send secrets, credentials, or raw health data through the bus.
- Use `localhost` in config; do not use numeric loopback addresses.

## v0.4 Implementation (Rust Native)

All components are Rust native (no Python dependency):
- **Binary:** `~/bin/agent-bus.exe` (~8 MB, instant startup)
- **Codecs:** JSON (serde_json), Compact JSON, TOON (ultra-compact, ~70% token savings), MessagePack (internal)
- **Compression:** LZ4 (bodies >512 bytes auto-compressed)
- **Storage:** Redis (realtime) + PostgreSQL (durable)
- **Allocator:** mimalloc (per Microsoft Pragmatic Rust Guidelines)
- **Transport:** stdio MCP, HTTP REST (Axum), MCP Streamable HTTP (2025-06-18 spec)

See `~/.codex/tools/agent-bus-mcp/IMPLEMENTATION_NOTES.md` for architecture details.

## Interop Roadmap

- **v0.4 stable:** All major features (channels, TOON, batch send, monitoring, Codex bridge)
- **v0.5+:** A2A task adapters for cross-framework handoffs, if needed
- **Future:** Distributed coordination across multiple machines (gossip protocol)

## Shared Skills Framework

See `~/.agents/SKILLS.md` for the cross-platform skill registry, coordination patterns, and agent capabilities matrix.

## Key Resources

| Resource | Path | Purpose |
|----------|------|---------|
| **This file (canonical)** | `~/.agents/AGENT_COORDINATION.md` | Cross-agent protocol spec |
| **Rust CLI binary** | `~/bin/agent-bus.exe` | Entrypoint for all commands |
| Rust CLI source | `~/.codex/tools/agent-bus-mcp/rust-cli/` | ~4500 LOC, 13 modules |
| Communication guide | `~/.codex/tools/agent-bus-mcp/AGENT_COMMUNICATIONS.md` | Detailed usage guide |
| Implementation notes | `~/.codex/tools/agent-bus-mcp/IMPLEMENTATION_NOTES.md` | Architecture, benchmarks |
| PowerShell wrappers | `~/.codex/tools/agent-bus-mcp/scripts/` | Terminal helpers |
| Coordination patterns | `~/.claude/context/agent-bus-coordination-patterns-20260313.md` | Multi-agent patterns |
| Skills registry | `~/.agents/SKILLS.md` | Agent capability matrix |
| MCP configs | `~/.claude/mcp.json`, `~/.codex/config.toml`, `~/.gemini/settings.json` | Agent registration |
