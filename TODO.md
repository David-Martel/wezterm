# WezTerm Multi-Agent TODO

Last updated: 2026-03-30
Repo: `C:\Users\david\wezterm`
Primary coordination thread: `wezterm-joint-plan-20260324`
Primary resource thread: `wezterm-resource-coordination-20260324`

## Working Rules

- Check the direct `codex` <-> `claude` bus thread before claiming new work.
- Treat `target/`, `Cargo.lock`, `~/bin` installs, live WezTerm config files, and Windows Terminal settings as exclusive resources.
- Default Rust builds/tests to namespaced `CARGO_TARGET_DIR` values and shared `sccache`.
- Post `RESOURCE_START`, `RESOURCE_UPDATE`, and `RESOURCE_DONE` for exclusive or high-cost work.
- See [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md) for full protocol.

## Current Status

### Tier 1: Ship Now — COMPLETE
- [x] Tier 1.A: Fix panel close-path crash in `~/.config/wezterm/codex_ui/panels.lua` (Codex)
- [x] Tier 1.B: Clean test baseline — 669+ passing, 0 failures (Claude)
- [x] Tier 1.C: First Rust render optimization — plain tab bar retained quad cache (Codex)

### Tier 2: Complete Phase 2+3 — IN PROGRESS
- [x] Tier 2.D: Panel intent persistence outside watched config tree (Codex — `~/.local/state/wezterm-utils/`)
- [x] Tier 2.E: Daemon IPC client for cross-window panel sync (Claude — `client.rs`)
- [ ] Tier 2.F: Settings feature tab / panel UX follow-up (Codex)

### Tier 3: Phase 4 Rust Optimization — IN PROGRESS
- [x] Tier 3.G: Wire module framework startup into GUI bootstrap (Claude — `main.rs` + `startup.rs`)
- [ ] Tier 3.H: Status-bar click zones and richer Rust-side chrome hooks (Codex)
- [ ] Tier 3.I: Performance profiling — Lua chrome vs Rust chrome benchmarks (Claude)
- [ ] Tier 3.J: DirectWrite tuning if rendering quality needs work (Both)
- [x] Tier 3.K: ~~Audit/fix daemon writer path~~ — verified NOT a bug: single channel created in `accept_connection()`, `tx` stored in `Connection`, `rx` passed to writer task. Added doc comments + 2 regression tests.

### Tier 4: Long-Term (from Customization Plan)
- [x] Tier 4.K: Hook module framework into config/src/lua.rs for Lua context setup — `add_context_setup_func` wired in `wezterm-gui/src/main.rs:1221`
- [ ] Tier 4.K2: Register module domains with mux
- [ ] Tier 4.K3: Add module configuration parsing
- [x] Tier 4.L: Module Lua API surface — `wezterm.watcher.*` (watch/unwatch/poll_events/on_event), `wezterm.fs_explorer.*` (spawn/is_available), `wezterm.daemon.*` (ping/status/broadcast/register)
- [ ] Tier 4.M: Example "hello world" module
- [ ] Tier 4.N: AI/LLM integration (mistral.rs, streaming, MCP client)
- [ ] Tier 4.O: Reach 85% test coverage for custom crates (currently ~73%, up from 68%)

### Tier 5: Repo-Managed Runtime + Coordination Tooling — NEW
- [ ] Tier 5.P: Validate repo-managed symlinked home config with GUI smoke test and Windows Terminal launch path
- [ ] Tier 5.Q: Verify repo-vendored `codex_ui` and `wezterm-utils` stay in sync with live runtime after restart/reload
- [ ] Tier 5.R: Adopt `agent-bus-http.exe` TOON / `compact-context` / `session-summary` flow for long WezTerm coordination sessions
- [ ] Tier 5.S: Triage `agent-bus-http.exe` PostgreSQL `jsonb` serialization warning seen in `read`/`compact-context`; decide fix vs documented Redis fallback
- [ ] Tier 5.T: Validate Windows-local `pre-commit` + `lefthook` installation and confirm both call the repo hook scripts correctly
- [ ] Tier 5.U: Expand ast-grep rule coverage with test-aware Rust panic/unwrap exclusions and keep auto-fix limited to syntax-preserving rewrites
- [ ] Tier 5.V: Extend warnings-as-errors enforcement to the remaining installer/build helper entrypoints without breaking third-party or dependency builds
- [ ] Tier 5.W: Resolve ast-grep inline `#[cfg(test)] mod` / attributed test-function exclusions without breaking rule parsing
- [ ] Tier 5.X: Reconcile global git `core.hooksPath` (`~/.git-hooks`) with repo-local `lefthook install` so hook installation does not clobber shared user hooks
- [x] Tier 5.Y: Custom-crate production code is now panic-free; all unwrap/expect/panic removed from non-test code in wezterm-module-framework and wezterm-utils-daemon; clippy -D warnings passes

### Tier 6: Jules-Assisted Review And Automation — NEW
- [x] Tier 6.T: ~~PR #7686 closed~~ — fork-only policy enforced; Jules session `16808362411090313266` findings still usable for internal code quality
- [x] Tier 6.U: Jules upstream PR review sessions terminated (fork-only policy); fork is 80 commits ahead of upstream with 0 behind — no cherry-picks needed
- [x] Tier 6.V: Keep Jules as an on-demand review tool only for now; the API is useful for targeted session review/feedback, but broad automated patch application is too noisy for this fork's dirty multi-agent workspace
- [ ] Tier 6.W: Use Jules to expand integration/coverage work for `wezterm-utils-daemon` and module-framework crates if local review confirms the current direction

### Tier 7: Runtime Integration Testing — NEW
- [x] Tier 7.A: Add `Test-Subcommands` to `Test-PostBuild.ps1` — verifies daemon, watch, explore, validate-config in `--help` + each subcommand's own `--help`
- [x] Tier 7.B: Add `Test-ValidateConfig` to `Test-PostBuild.ps1` — runs `wezterm validate-config --format json`, parses JSON, checks valid/config_file/warnings/watch_paths
- [x] Tier 7.C: Create `Test-Integration.ps1` — daemon lifecycle (start/IPC/shutdown), named pipe protocol tests (ping, status, register+subscribe+broadcast), config validation
- [x] Tier 7.D: Add Just targets `test-postbuild`, `test-integration`, `test-runtime`
- [x] Tier 7.E: Add CI step in `windows-ci.yml` — install binaries + run integration tests (continue-on-error initially)
- [ ] Tier 7.F: Harden integration tests — remove continue-on-error after first green CI run
- [ ] Tier 7.G: Add watcher module integration test (start wezterm-watch, trigger file event, verify output)
- [ ] Tier 7.H: Add cross-subcommand integration test (daemon + watch coordination via IPC)

## Active Owners

### Codex
- [x] Render cache tranche validated (12/12 tests)
- [x] Panel intent persistence implemented
- [ ] Validate panel persistence with runtime restart checks
- [ ] Review Claude's GUI bootstrap hook (`wezterm-gui/src/main.rs`)
- [ ] Settings feature tab (Tier 2.F)
- [ ] Click-zone support (Tier 3.H)
- [x] Daemon writer-path audit — verified correct, documented + tested (Tier 3.K)
- [ ] Repo-managed config GUI/Windows Terminal validation (Tier 5.P)
- [ ] Agent-bus HTTP workflow adoption + warning triage proposal (Tier 5.R / Tier 5.S)
- [ ] Ast-grep / pre-commit / lefthook validation and safe autofix rollout (Tier 5.T / Tier 5.U)
- [ ] Strict warning gate rollout for remaining build helpers (Tier 5.V)
- [ ] Ast-grep inline-test false-positive suppression research and validation (Tier 5.W)
- [ ] Hook installation strategy for global `~/.git-hooks` / `lefthook install` compatibility (Tier 5.X)
- [ ] Ast-grep unwrap/panic backlog reduction for custom crates and benchmarks (Tier 5.Y)
- [x] Review Jules session `16808362411090313266` findings for internal code quality (Tier 6.T — PR #7686 closed, fork-only)

### Claude
- [x] Daemon IPC client (`wezterm-utils-daemon/src/client.rs`)
- [x] Module framework startup (`wezterm-module-framework/src/startup.rs`)
- [x] GUI bootstrap hookup (`wezterm-gui/src/main.rs`)
- [x] Subcommand registration (daemon, watch, explore in `wezterm/src/main.rs`)
- [x] Validate-config subcommand with JSON output (`wezterm/src/validate_config.rs`)
- [x] Runtime integration test harness (Tier 7.A-E)
- [ ] Review Codex render cache patch
- [ ] Performance profiling (Tier 3.I)
- [x] Module Lua API surface complete (Tier 4.L — watcher, fs_explorer, daemon bindings)
- [ ] Test coverage improvements (Tier 4.O)
- [ ] Harden integration tests after first green CI (Tier 7.F)
- [ ] Watcher + cross-subcommand integration tests (Tier 7.G-H)
- [x] Surface actionable findings from active Jules upstream-compat sessions (`#7683`, `#7679`, `#7673`) (Tier 6.U)

## P0: Cross-Review And Integration

- [ ] Claude reviews retained plain-tab-bar cache patch in `wezterm-gui/src/termwindow/render/`
- [ ] Codex reviews `wezterm-gui/src/main.rs` module init hook
- [ ] Codex reviews current Claude/Jules-assisted patch set in `wezterm-gui/src/main.rs` and `wezterm-utils-daemon/src/*` before integration
- [ ] Run full workspace nextest on default target after both patches merged
- [ ] Decide: next Rust chrome step = cache-focused or click zones?
- [ ] Confirm module-framework follow-through after `main.rs` hook: mux domain registration + Lua context hook sequencing
- [x] Pull Jules session `16808362411090313266` and reconcile its findings with local cross-review results

## P1: Validation And Quality

- [ ] Confirm panel toggles work after restart/config reload
- [ ] Run focused WezTerm config validation after panel persistence changes
- [ ] Run GUI smoke test against repo-managed home symlink layout (`.wezterm.lua` + `.config/wezterm/*`)
- [ ] Validate Windows Terminal profile still launches through the symlinked home config path
- [ ] Validate `pre-commit install` + `lefthook install` on Windows and confirm the generated git hooks invoke the repo-managed scripts
- [ ] Keep ast-grep safe autofix limited to syntax-preserving rewrites; reject semantic rewrites like blanket `unwrap()` -> `?`
- [ ] Run `cargo nextest run --workspace` on default target (both patches integrated)
- [ ] Keep utility tests and daemon/module tests green on namespaced targets
- [ ] Keep `agent-bus-http.exe` compact-context/session-summary usable without PostgreSQL fallback regressions
- [ ] Validate any Jules-assisted daemon/bootstrap patch with focused `cargo check` / `cargo nextest` before adoption

## Jules Review Notes

- Session `995757760320847112` (`AWAITING_USER_FEEDBACK`): keep the session alive, but narrow scope to `.jules` and `rules/rust/*` only; reject deletion of `rules/rust/avoid-unwrap.yml`, keep the conservative `avoid-sync-mutex-in-async` stance unless false positives are proven reduced, and avoid unrelated repo cleanup.
- Session `7825580181922692933` (`COMPLETED`): reject upstream PR `#7683` for current Windows UX work. Jules found the feature is FreeType-only, ignored on the DirectWrite/WebGpu path, redundant for Cascadia variable fonts, and under-tested.
- Session `11018576285087526146` (`COMPLETED`): treat upstream PR `#7679` as a future design reference only. The session completed, but did not produce a concrete patch or final recommendation suitable for direct adoption.
- Session `432181464351595015` (`FAILED`): no actionable compatibility conclusion for upstream PR `#7673`; rerun only if per-pane title bars become a near-term priority.
- Session `16808362411090313266` (`COMPLETED`): accept the review direction, not the full patch. Keep the SAFETY comment guidance and startup-hook review, but validate daemon IPC changes locally before adopting any Jules-generated cross-platform refactor.

## UX Redesign Phase Status

| Phase | Completion | Next Action |
|-------|-----------|-------------|
| Phase 1: Rendering + Config | 95% | Verification testing only |
| Phase 2: Chrome Overhaul | 85% | Settings tab, click handlers |
| Phase 3: Panel System | 75% | State persistence validation, Settings tab |
| Phase 4: Rust Investment | 55% | Module framework integrated, daemon client done, subcommands wired, validate-config, runtime test harness |

## Known Constraints

- Frequent writes under `~/.config/wezterm` trigger the WezTerm config watcher — reload churn.
- Panel persistence uses `~/.local/state/wezterm-utils/` (NOT `~/.config/wezterm/`).
- Persisted state = desired intent (booleans), NOT live pane ids or window ids.
- Claude owns `wezterm-gui/src/main.rs` for startup hook; Codex owns render path files.
- Build lock contention: use per-agent `CARGO_TARGET_DIR` per [RESOURCE_COORDINATION.md](./RESOURCE_COORDINATION.md).
- `agent-bus-http.exe` currently warns and falls back from PostgreSQL to Redis for some reads because of a `jsonb` serialization mismatch; coordination still works, but context compaction quality/latency should be treated as degraded until triaged.
- Jules runs asynchronously against GitHub and may produce useful review patches/comments, but local agents still need to pull, review, and validate those results before applying them.

## Validation Commands

```powershell
# Lua/config validation
wezterm --config-file $env:USERPROFILE\.wezterm.lua show-keys

# Full workspace test (use namespaced target or claim default)
$env:RUSTC_WRAPPER='sccache'
$env:CARGO_TARGET_DIR='C:/Users/david/.cache/<agent>/nextest'
cargo nextest run --workspace --no-fail-fast

# Focused GUI validation
$env:CARGO_TARGET_DIR='C:/Users/david/.cache/<agent>/wezterm-gui'
cargo check -p wezterm-gui
cargo nextest run -p wezterm-gui --no-fail-fast

# Custom utilities only
cargo nextest run -p wezterm-watch -p wezterm-module-framework -p wezterm-utils-daemon
```

## Planning Documents

All plans consolidated under `docs/`:

| Document | Location | Description |
|----------|----------|-------------|
| **Joint Development Plan** | [docs/plans/2026-03-23-wezterm-joint-plan.md](docs/plans/2026-03-23-wezterm-joint-plan.md) | Active task ownership, tier priorities, test coverage |
| **UX Redesign Execution** | [docs/plans/2026-03-23-wezterm-ux-redesign.md](docs/plans/2026-03-23-wezterm-ux-redesign.md) | 4-phase implementation roadmap |
| **Customization Roadmap** | [docs/plans/wezterm-customization-plan.md](docs/plans/wezterm-customization-plan.md) | Long-term: AI module, upstream merges, guidelines |
| **Test Coverage Plan** | [docs/plans/custom-modules-test-plan.md](docs/plans/custom-modules-test-plan.md) | 85% target, test pyramid strategy |
| **UX Design Spec** | [docs/specs/2026-03-23-wezterm-ux-redesign-design.md](docs/specs/2026-03-23-wezterm-ux-redesign-design.md) | Approved 4-phase design spec |
| **AI Module Design** | [docs/design/WEZTERM_AI_MODULE_DESIGN.md](docs/design/WEZTERM_AI_MODULE_DESIGN.md) | Module framework + LLM architecture |
| **Resource Protocol** | [RESOURCE_COORDINATION.md](RESOURCE_COORDINATION.md) | Shared resource contention protocol |
| **Agent Protocol** | [AGENT_COORDINATION.md](AGENT_COORDINATION.md) | Cross-agent IPC protocol |
