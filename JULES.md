# Jules Integration Guide

> Google's Jules is an asynchronous AI coding agent that runs in the cloud against GitHub repos.
> It complements Claude and Codex by handling tasks that benefit from full repo context,
> parallel execution, and automated PR review without consuming local compute.

**CLI**: `jules` (v0.1.42, installed via npm)
**Repo**: `David-Martel/wezterm`
**Auth**: Google account (run `jules login` to authenticate)

---

## Quick Start

```bash
# Review a PR
jules new --repo David-Martel/wezterm "Review PR #7686 for Rust code quality and performance"

# Write tests for a module
jules new "Write integration tests for wezterm-utils-daemon/src/client.rs"

# Fix an issue
jules new "Fix issue #123: panel toggle crash on Alt+1"

# Parallel exploration (up to 5)
jules new --parallel 3 "Find all unwrap() calls in custom crates and suggest Result-based alternatives"

# Pull results
jules remote pull --session <ID>

# Apply patch directly
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
agent-bus-http.exe send \
  --from-agent claude --to-agent all \
  --topic "jules-findings" \
  --body "$RESULT" \
  --tags "repo:wezterm,jules,review"
```

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
| 16808362411090313266 | PR #7686 review (our main PR) | Running |
| 7825580181922692933 | PR #7683 review (font thickening) | Running |
| 11018576285087526146 | PR #7679 review (vertical tab bar) | Running |
| 432181464351595015 | PR #7673 review (per-pane title bars) | Running |

---

## References

- [Jules Documentation](https://jules.google.com)
- [Jules CLI GitHub](https://github.com/jiahao42/jules-cli)
- [AGENTS.md](./AGENTS.md) — Agent coordination guidelines
- [AGENT_COORDINATION.md](./AGENT_COORDINATION.md) — Cross-agent IPC protocol
- [TODO.md](./TODO.md) — Current task tracking
