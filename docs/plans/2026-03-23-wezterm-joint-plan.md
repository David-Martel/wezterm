# WezTerm Joint Development Plan

> Joint plan between Claude and Codex agents for wezterm optimization and UX redesign completion.
> Created: 2026-03-24

## Current Status

| Phase | Completion | Status |
|-------|-----------|--------|
| Phase 1: Rendering + Config | 95% | Shippable |
| Phase 2: Chrome Overhaul | 85% | Core works, polish needed |
| Phase 3: Panel System | 70% | Toggles work, persistence needed |
| Phase 4: Rust Investment | 10% | Foundations only |
| **Overall** | **87%** | |

## Tier 1: Ship Now (4-6 hours)

| Task | Owner | Status | Description |
|------|-------|--------|-------------|
| A. Fix panels.lua crash | Codex | DONE | Activate-before-close pattern for CloseCurrentPane |
| B. Test baseline | Claude | DONE | 669+ passing, 0 failures, 4 fixed |
| C. Dirty-region rendering | Codex | IN PROGRESS | Prototype in wezterm-gui/src/termwindow/render/ |

## Tier 2: Complete Phase 2+3 (6-10 hours)

| Task | Owner | Description |
|------|-------|-------------|
| D. Panel state persistence | Codex | Save/restore panel state to ui-preferences.lua |
| E. Daemon IPC integration | Claude | Connect panels to wezterm-utils-daemon |
| F. Settings feature tab | Codex | Initial UI (palette-driven or ratatui) |

## Tier 3: Phase 4 Rust Optimization (10-15 hours)

| Task | Owner | Description |
|------|-------|-------------|
| G. Module framework integration | Claude | Move tab title rendering to Rust |
| H. Click-zone support | Codex | Status bar click detection in termwindow |
| I. Performance profiling | Claude | Lua chrome vs Rust chrome benchmarks |
| J. DirectWrite tuning | Both | If rendering quality needs work |

## Performance Gains Achieved

- Chrome Lua callbacks: 26% -> 7% idle CPU (IPC throttling)
- Config reload storm: 8 reloads/21sec -> 1 (state dir moved)
- wezterm-watch: N git status/N events -> 1/batch
- fs-explorer: rayon parallel dir scanning (2-4x on large dirs)
- fs-explorer: panic=unwind, catch_unwind, terminal restore guard

## Test Coverage

| Crate | Unit | Integration | E2E |
|-------|------|------------|-----|
| wezterm-watch | 77 tests | 26KB | 26KB |
| wezterm-fs-explorer | inline | 17KB | 22KB |
| wezterm-module-framework | 14 tests | - | - |
| wezterm-utils-daemon | inline | integration_test.rs | - |
| wezterm-benchmarks | - | - | - |

## File Ownership

**Claude owns:**
- wezterm-utils-daemon/src/*
- wezterm-module-framework/src/*
- wezterm-watch/src/*
- wezterm-benchmarks/src/*
- tools/CargoTools/*
- CLAUDE.md, AGENTS.md

**Codex owns:**
- wezterm-fs-explorer/src/*
- ~/.wezterm.lua
- ~/.config/wezterm/codex_ui/*.lua
- wezterm-gui/src/termwindow/render/* (dirty-region work)

## Related Planning Documents

- `~/docs/superpowers/specs/2026-03-23-wezterm-ux-redesign-design.md` — UX redesign spec (4 phases)
- `~/docs/superpowers/plans/2026-03-23-wezterm-ux-redesign.md` — UX redesign execution plan
- `~/wezterm/.claude/plans/wezterm-customization-plan.md` — Long-term customization roadmap (AI module, upstream merges)
- `~/wezterm/.claude/plans/custom-modules-test-plan.md` — Test coverage targets (85% for custom crates)
- `~/wezterm/RESOURCE_COORDINATION.md` — Shared resource contention protocol
- `~/wezterm/WEZTERM_AI_MODULE_DESIGN.md` — AI assistant module design spec

## Outstanding Items from Customization Plan

From `.claude/plans/wezterm-customization-plan.md` (created 2026-02-04):

### Phase 2: AI Module Implementation (partially started)
- [x] Create wezterm-module-framework crate
- [x] Implement Module trait + ModuleManager (now ModuleRegistry)
- [x] Implement ModuleIpc (now via wezterm-utils-daemon)
- [x] Add tests for module lifecycle (14 tests passing)
- [ ] Hook into config/src/lua.rs for Lua context setup
- [ ] Hook into wezterm-gui/src/main.rs for module initialization
- [ ] Register module domains with mux
- [ ] Create lua-api-crates/module-framework/ Lua API crate
- [ ] Example "hello world" module

### Phase 3: LLM Integration (not started)
- [ ] mistral.rs / gemma.cpp integration
- [ ] Streaming response support
- [ ] MCP client configuration
- [ ] Tool execution pipeline

### Test Coverage Gaps (from test plan)
- Target: 85% for custom crates
- Current: wezterm-module-framework needs integration tests
- Current: wezterm-utils-daemon needs E2E tests with daemon running
- Current: wezterm-benchmarks tests need daemon dependency
