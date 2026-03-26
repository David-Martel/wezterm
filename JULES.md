# Jules Integration Guide

> Google's Jules is an asynchronous AI coding agent that runs in the cloud against GitHub repos.
> It complements Claude and Codex by handling tasks that benefit from full repo context,
> parallel execution, and automated PR review without consuming local compute.

**CLI**: `jules` (v0.1.42, installed via npm)
**Repo**: `David-Martel/wezterm`
**Auth**: Google account (run `jules login` to authenticate)

## Scope And Current Role

In this repo, Jules is best used for:

- asynchronous PR review and compatibility review
- broad search/refactor proposals that do not need immediate local iteration
- test-generation suggestions and coverage expansion
- security and dependency review

Jules is not the source of truth for local integration. All Jules output must still flow through:

- [TODO.md](./TODO.md)
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md)
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md)
- local validation (`cargo check`, `cargo nextest`, `sg scan -c sgconfig.yml`, clippy)

---

## Quick Start

```bash
# Review a PR
jules new --repo David-Martel/wezterm "Review latest changes on main for Rust code quality and performance"

# Write tests for a module
jules new "Write integration tests for wezterm-utils-daemon/src/client.rs"

# Fix an issue
jules new "Fix issue #123: panel toggle crash on Alt+1"

# Parallel exploration (up to 5)
jules new --parallel 3 "Find all unwrap() calls in custom crates and suggest Result-based alternatives"

# Pull results first
jules remote pull --session <ID>

# Apply only after local review and validation
jules remote pull --session <ID> --apply

# Or teleport (clone + apply in one step)
jules teleport <ID>
```

---

## CI/CD Integration

### 1. Automated PR Review

Jules can review every PR automatically. Add to `.github/workflows/jules-review.yml`:

```yaml
name: Jules PR Review
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  jules-review:
    runs-on: ubuntu-latest
    steps:
      - name: Review PR with Jules
        run: |
          jules new --repo ${{ github.repository }} \
            "Review PR #${{ github.event.pull_request.number }}: \
             ${{ github.event.pull_request.title }}. \
             Check for: Rust code quality (Microsoft Pragmatic Rust Guidelines), \
             unsafe code without SAFETY comments, performance regressions, \
             test coverage gaps. Post findings as PR review comments."
```

### 2. Pre-Merge Quality Gate

Use Jules to verify code quality before merging:

```yaml
name: Jules Quality Gate
on:
  pull_request:
    types: [labeled]

jobs:
  quality-gate:
    if: contains(github.event.label.name, 'jules-review')
    runs-on: ubuntu-latest
    steps:
      - name: Deep Review
        run: |
          SESSION=$(jules new --repo ${{ github.repository }} \
            "Deep review of PR #${{ github.event.pull_request.number }}: \
             1) Check all new/modified Rust files against clippy pedantic lints \
             2) Verify error handling uses Result not unwrap \
             3) Check that public types implement Debug \
             4) Verify tests exist for new functions \
             5) Check for potential panics in non-test code" \
            2>&1 | grep "ID:" | awk '{print $2}')
          echo "Jules session: $SESSION"
          echo "session_id=$SESSION" >> $GITHUB_OUTPUT
```

### 3. Automated Test Generation

Generate tests for under-covered modules:

```bash
# Generate tests for a specific module
jules new "Write comprehensive unit and integration tests for \
  wezterm-module-framework/src/startup.rs. \
  Target: 85% coverage. Include tests for: \
  - initialize_modules() idempotency \
  - register_lua_apis() with mock Lua context \
  - Module registration failure handling \
  - Concurrent access to ModuleRegistry"

# Generate tests for all custom crates
for crate in wezterm-utils-daemon wezterm-module-framework wezterm-watch; do
  jules new "Write integration tests for $crate. \
    Follow the test patterns in the existing test files. \
    Use k9 assertions where available. \
    Target 85% coverage."
done
```

### 4. Dependency Audit

```bash
jules new "Audit all dependencies in Cargo.toml and Cargo.lock for: \
  1) Known security vulnerabilities (run cargo audit mentally) \
  2) Outdated crates with available updates \
  3) Duplicate dependency versions that could be unified \
  4) Unnecessary dependencies that could be removed \
  Post findings as a summary with severity levels."
```

### 5. Issue Triage Pipeline

```bash
# Auto-assign issues to Jules
gh issue list --assignee @me --limit 5 --json number,title | \
  jq -r '.[] | "Fix issue #\(.number): \(.title)"' | \
  while IFS= read -r task; do
    jules new "$task"
  done

# Or use Gemini to pick the best issue
gemini -p "Pick the most impactful open issue:\n$(gh issue list --limit 20)" | jules new
```

---

## Workflow Patterns for WezTerm

### Pattern 1: Parallel Exploration

Use `--parallel` to explore multiple approaches:

```bash
# Try 3 different approaches to dirty-region rendering
jules new --parallel 3 \
  "Implement dirty-region tracking in wezterm-gui/src/termwindow/render/paint.rs. \
   The goal: only repaint regions that changed (tab bar, status bar, pane content). \
   Currently paint_impl() repaints everything on every frame."
```

### Pattern 2: Cross-PR Compatibility Check

```bash
# Check if an upstream PR conflicts with our fork
jules new "Analyze PR #7683 (font thickening) for conflicts with: \
  1) Our WebGpu/DirectWrite rendering in .wezterm.lua (gpu_score, pick_webgpu_adapter) \
  2) Our font stack configuration (Cascadia Mono, Regular weight) \
  3) Our chrome.lua tab bar rendering. \
  Report: will this PR merge cleanly? Any behavioral changes?"
```

### Pattern 3: Refactoring Assistant

```bash
# Large-scale refactoring
jules new "Refactor wezterm-benchmarks/src/ to fix all 16 unused import warnings. \
  Also replace any remaining .unwrap() in non-test code with proper error handling. \
  Follow Microsoft Pragmatic Rust Guidelines (M-PANIC-IS-STOP)."
```

### Pattern 4: Documentation Generation

```bash
# Generate missing docs
jules new "Add module-level documentation (//! comments) to all files in \
  wezterm-module-framework/src/ that are missing them. \
  Follow M-MODULE-DOCS from Microsoft Rust Guidelines. \
  Include: purpose, usage examples, and cross-references."
```

### Pattern 5: Coordinated Multi-Agent Review

```bash
# Jules reviews what Claude and Codex built
jules new "Review the recent changes by Claude and Codex agents: \
  1) wezterm-utils-daemon/src/client.rs (new IPC client) \
  2) wezterm-module-framework/src/startup.rs (new startup hooks) \
  3) wezterm-gui/src/main.rs (module init hookup) \
  4) wezterm-watch/src/main.rs (batched git refresh) \
  5) codex_ui/chrome.lua (IPC throttling, tab cache) \
  Check each for correctness, edge cases, and Microsoft Rust Guidelines compliance."
```

---

## Operating Rules For This Repo

0. **Fork-only policy**: This repo is a downstream fork of `wezterm/wezterm`. Never create PRs, push commits, or contribute changes back to upstream. Jules sessions should target `David-Martel/wezterm` exclusively. Upstream is fetch-only for pulling meaningful updates.
1. Post new Jules sessions and material findings to the direct Codex/Claude bus thread with `agent-bus-http.exe post-direct`.
2. Convert actionable Jules findings into concrete [TODO.md](./TODO.md) items before applying patches.
3. Do not apply Jules patches blindly; review the diff locally first.
4. If a Jules patch touches exclusive resources or heavy build/install surfaces, coordinate first via [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md).
5. For Rust-heavy validation after adopting Jules output, prefer namespaced targets:

```powershell
$env:RUSTC_WRAPPER='sccache'
$env:CARGO_TARGET_DIR='C:/Users/david/.cache/<agent>/<task>'
cargo nextest run -p wezterm-utils-daemon --no-fail-fast
```

6. If Jules findings conflict with local implementation reality, update [TODO.md](./TODO.md) and the shared bus thread rather than forcing the patch.

## Current Priority Uses

Cross-check these against [TODO.md](./TODO.md):

- Tier 6.T: ~~PR `#7686`~~ (closed, fork-only policy) — session `16808362411090313266` findings still relevant for code quality, but the proposed daemon IPC refactor must be validated locally before adoption
- Tier 6.U: compatibility reviews for upstream PRs `#7683`, `#7679`, `#7673`; current disposition is "reference only", not "auto-adopt"
- Tier 6.V: keep Jules as an on-demand review tool for now; avoid broad automated PR review rollout until session noise is lower and patch quality is more predictable in this fork
- Tier 6.W: targeted test/coverage expansion for `wezterm-utils-daemon` and `wezterm-module-framework`

## Integration with Agent Bus

Jules sessions can be tracked through the agent bus for multi-agent coordination:

```bash
# Post Jules session to agent bus for tracking
agent-bus-http.exe post-direct \
  --from-agent claude --to-agent codex \
  --body "JULES SESSION: $(jules new 'review task' 2>&1 | grep ID)" \
  --topic jules-review

# After session completes, pull and share results
RESULT=$(jules remote pull --session <ID> 2>&1)
agent-bus-http.exe post-direct \
  --from-agent claude --to-agent codex \
  --topic "jules-findings" \
  --body "$RESULT" \
  --tag "repo:wezterm"
```

Practical bus usage:

- prefer `post-direct` / `read-direct` for Codex<->Claude Jules coordination
- use `session-summary` or `compact-context` for long review waves
- health-check `http://localhost:8400/health` before relying on the HTTP path for long sessions
- treat `watch` as a live probe, not the canonical record

---

## Best Practices

1. **Be specific**: Jules works best with detailed task descriptions including file paths, line numbers, and expected behavior.

2. **Use parallel for exploration**: When unsure of the best approach, use `--parallel 3-5` to get multiple solutions.

3. **Pull before applying**: Always `jules remote pull --session <ID>` to review before `--apply`.

4. **Combine with local agents**: Use Jules for async background work while Claude/Codex handle interactive development.

5. **Track sessions**: Log session IDs to the agent bus so all agents can reference Jules findings.

6. **Pipe from issues**: `gh issue list | jules new` is powerful for automated issue resolution.

7. **Use for cross-repo analysis**: Jules can work on any GitHub repo — useful for checking upstream WezTerm changes.

---

## Session Management

```bash
# List all sessions
jules remote list --session

# List repos Jules can access
jules remote list --repo

# Pull specific session result
jules remote pull --session <ID>

# Apply session patch to local repo
jules remote pull --session <ID> --apply

# Or teleport (clone + checkout + apply)
jules teleport <ID>
```

---

## Current Active Sessions

| Session ID | PR/Task | Status |
|-----------|---------|--------|
| 17869272520108303187 | Windows launcher panic / bundle review | In Progress |
| 995757760320847112 | ast-grep review and rule suggestions | Awaiting User Feedback |
| 16808362411090313266 | ~~PR #7686~~ (closed, fork-only) — code quality findings | Completed |
| 7825580181922692933 | PR #7683 review (font thickening) | Completed |
| 11018576285087526146 | PR #7679 review (vertical tab bar) | Completed |
| 432181464351595015 | PR #7673 review (per-pane title bars) | Failed |

### Current Disposition

- `995757760320847112`: continue, but constrain follow-up to `.jules` and `rules/rust/*`; do not accept deletion of `rules/rust/avoid-unwrap.yml`, and keep async-mutex linting conservative unless the false-positive story improves.
- `7825580181922692933`: do not adopt upstream PR `#7683` for current Windows UX work; Jules found it is FreeType-only, not effective on the DirectWrite/WebGpu path, and weaker than simply using variable font weights for Cascadia.
- `11018576285087526146`: keep as background design input only; no direct patch or firm recommendation was produced.
- `432181464351595015`: rerun later if per-pane title bars become a near-term product goal.
- `16808362411090313266`: treat the patch as review material rather than an auto-apply candidate, especially around daemon IPC generalization.

---

## References

- [Jules Documentation](https://jules.google.com)
- [Jules CLI GitHub](https://github.com/jiahao42/jules-cli)
- [AGENTS.md](./AGENTS.md) — Agent coordination guidelines
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) — Cross-agent IPC protocol
- [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) — Exclusive-resource and heavy-build protocol
- [TODO.md](./TODO.md) — Current task tracking
